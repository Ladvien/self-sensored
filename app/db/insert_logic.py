# app/db/insert_logic.py - Idempotent version

import hashlib
import json
from app.db.models import HealthPayload, HealthMetric, QuantityTimestamp
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.dialects.postgresql import insert
from sqlalchemy import select
from uuid import uuid4
import logging

import app.db.models as db_models

logger = logging.getLogger(__name__)

# Map schema keys to SQLAlchemy model classes
SPECIALIZED_DB_MODELS = {
    "blood_pressure": db_models.BloodPressure,
    "heart_rate": db_models.HeartRate,
    "sleep_analysis": db_models.SleepAnalysis,
}


def generate_payload_hash(payload_data: dict) -> str:
    """Generate a deterministic hash for the payload to detect duplicates"""
    # Sort keys to ensure consistent hashing
    normalized = json.dumps(payload_data, sort_keys=True, default=str)
    return hashlib.sha256(normalized.encode()).hexdigest()


async def check_payload_exists(payload_hash: str, db: AsyncSession) -> bool:
    """Check if a payload with this hash already exists"""
    result = await db.execute(
        select(HealthPayload.id).where(HealthPayload.payload_hash == payload_hash)
    )
    return result.scalar_one_or_none() is not None


async def insert_health_data_idempotent(
    payload: HealthPayload, db: AsyncSession, raw_payload: dict = None
):
    """Idempotent version of insert_health_data using upserts and deduplication"""

    # Generate payload hash for deduplication
    payload_hash = generate_payload_hash(raw_payload or payload.model_dump())

    # Check if we've already processed this exact payload
    if await check_payload_exists(payload_hash, db):
        logger.info(
            f"Payload with hash {payload_hash[:8]}... already processed, skipping"
        )
        return {"status": "duplicate", "message": "Payload already processed"}

    try:
        async with db.begin():
            # Insert payload with hash using upsert
            payload_stmt = insert(HealthPayload).values(
                id=uuid4(), payload_hash=payload_hash
            )
            payload_stmt = payload_stmt.on_conflict_do_nothing(
                index_elements=["payload_hash"]
            ).returning(HealthPayload.id)

            result = await db.execute(payload_stmt)
            payload_id = result.scalar_one_or_none()

            if not payload_id:
                # Payload was already inserted by another process
                logger.info("Payload was inserted by concurrent process, skipping")
                return {
                    "status": "concurrent_insert",
                    "message": "Payload processed by another request",
                }

            # Process metrics
            for metric in payload.metrics:
                await insert_metric_idempotent(metric, payload_id, db)

            logger.info(f"Successfully processed payload {payload_hash[:8]}...")
            return {"status": "success", "message": "Data processed successfully"}

    except Exception as e:
        logger.error(f"Failed to process payload: {e}")
        raise


async def insert_metric_idempotent(metric, payload_id: str, db: AsyncSession):
    """Insert a metric using upsert to handle duplicates"""

    # Insert metric using upsert
    metric_stmt = insert(HealthMetric).values(
        id=uuid4(), payload_id=payload_id, name=metric.name, units=metric.units
    )
    metric_stmt = metric_stmt.on_conflict_do_update(
        index_elements=["payload_id", "name"],
        set_=dict(units=metric_stmt.excluded.units),  # Update units if changed
    ).returning(HealthMetric.id)

    result = await db.execute(metric_stmt)
    metric_id = result.scalar_one()

    if not metric.data:
        return

    # Determine which database model to use
    metric_type = metric.name.lower()
    db_model_cls = SPECIALIZED_DB_MODELS.get(metric_type)

    try:
        if db_model_cls is None:
            # Handle quantity timestamp data with upsert
            await insert_quantity_data_idempotent(metric.data, metric_id, db)
        else:
            # Handle specialized data with upsert
            await insert_specialized_data_idempotent(
                metric.data, metric_id, db_model_cls, metric_type
            )

    except Exception as e:
        logger.warning(f"Skipping metric '{metric.name}' due to error: {e}")


async def insert_quantity_data_idempotent(
    data_entries, metric_id: str, db: AsyncSession
):
    """Insert quantity timestamp data with conflict resolution"""

    records = []
    for entry in data_entries:
        try:
            records.append(
                {
                    "id": uuid4(),
                    "metric_id": metric_id,
                    "date": entry.get_date(),
                    "qty": entry.qty,
                    "source": entry.source,
                }
            )
        except Exception as e:
            logger.warning(f"Skipping invalid quantity entry: {e}")

    if records:
        stmt = insert(QuantityTimestamp).values(records)
        stmt = stmt.on_conflict_do_update(
            index_elements=["metric_id", "date", "source"],
            set_=dict(
                qty=stmt.excluded.qty,  # Update with latest value
            ),
        )
        await db.execute(stmt)


async def insert_specialized_data_idempotent(
    data_entries, metric_id: str, db: AsyncSession, model_cls, metric_type: str
):
    """Insert specialized health data with conflict resolution"""

    records = []
    for entry in data_entries:
        try:
            record = entry.model_dump(exclude={"id"}, exclude_unset=True)
            record["id"] = uuid4()
            record["metric_id"] = metric_id
            records.append(record)
        except Exception as e:
            logger.warning(f"Skipping invalid {metric_type} entry: {e}")

    if records:
        stmt = insert(model_cls).values(records)

        # Define conflict resolution based on model type
        if metric_type == "heart_rate":
            stmt = stmt.on_conflict_do_update(
                index_elements=["metric_id", "date"],
                set_=dict(
                    min=stmt.excluded.min,
                    avg=stmt.excluded.avg,
                    max=stmt.excluded.max,
                    context=stmt.excluded.context,
                    source=stmt.excluded.source,
                ),
            )
        elif metric_type == "blood_pressure":
            stmt = stmt.on_conflict_do_update(
                index_elements=["metric_id", "date"],
                set_=dict(
                    systolic=stmt.excluded.systolic, diastolic=stmt.excluded.diastolic
                ),
            )
        elif metric_type == "sleep_analysis":
            stmt = stmt.on_conflict_do_update(
                index_elements=["metric_id", "start_date"],
                set_=dict(
                    end_date=stmt.excluded.end_date,
                    value=stmt.excluded.value,
                    qty=stmt.excluded.qty,
                    source=stmt.excluded.source,
                ),
            )
        else:
            # Default: do nothing on conflict
            stmt = stmt.on_conflict_do_nothing()

        await db.execute(stmt)


# Keep the original function for backward compatibility
async def insert_health_data(payload: HealthPayload, db: AsyncSession):
    """Original insert function - now calls idempotent version"""
    return await insert_health_data_idempotent(payload, db)
