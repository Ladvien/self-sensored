# app/db/insert_logic.py - Complete Idempotent Implementation

import hashlib
import json
from typing import Dict, Any, Optional, List
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.dialects.postgresql import insert
from sqlalchemy import select
from uuid import uuid4
import logging

from app.api.models import HealthPayload, HealthMetric
import app.db.models as db_models

logger = logging.getLogger(__name__)

# Map API model names to SQLAlchemy model classes
SPECIALIZED_DB_MODELS = {
    "blood_pressure": db_models.BloodPressure,
    "heart_rate": db_models.HeartRate,
    "sleep_analysis": db_models.SleepAnalysis,
    "blood_glucose": db_models.BloodGlucose,
    "sexual_activity": db_models.SexualActivity,
    "handwashing": db_models.HygieneEvent,
    "toothbrushing": db_models.HygieneEvent,
    "insulin_delivery": db_models.InsulinDelivery,
    "symptom": db_models.Symptom,
    "state_of_mind": db_models.StateOfMind,
    "ecg": db_models.ECG,
    "heart_rate_notifications": db_models.HeartRateNotification,
}


def generate_payload_hash(payload_data: dict) -> str:
    """Generate a deterministic hash for the payload to detect duplicates"""
    # Sort keys and normalize data for consistent hashing
    normalized = json.dumps(payload_data, sort_keys=True, default=str)
    return hashlib.sha256(normalized.encode()).hexdigest()


def generate_metric_hash(metric_name: str, metric_data: List[Dict[str, Any]]) -> str:
    """Generate a hash for a specific metric and its data"""
    data_str = json.dumps(
        {"name": metric_name, "data": metric_data}, sort_keys=True, default=str
    )
    return hashlib.md5(data_str.encode()).hexdigest()


async def check_payload_exists(payload_hash: str, db: AsyncSession) -> Optional[str]:
    """Check if a payload with this hash already exists, return its ID if found"""
    result = await db.execute(
        select(db_models.HealthPayload.id).where(
            db_models.HealthPayload.payload_hash == payload_hash
        )
    )
    return result.scalar_one_or_none()


async def insert_health_data(
    payload: HealthPayload, db: AsyncSession, raw_payload: dict = None
) -> Dict[str, Any]:
    """
    Idempotent insert of health data with comprehensive duplicate handling

    Returns:
        dict: Status information including:
            - status: 'success', 'duplicate', 'partial_success'
            - message: Human-readable message
            - payload_id: ID of the payload (new or existing)
            - metrics_processed: Number of metrics processed
            - metrics_skipped: Number of metrics skipped as duplicates
    """

    # Generate payload hash
    payload_dict = raw_payload or payload.model_dump()
    payload_hash = generate_payload_hash(payload_dict)

    # Check if payload already exists
    existing_payload_id = await check_payload_exists(payload_hash, db)

    if existing_payload_id:
        logger.info(f"Payload with hash {payload_hash[:8]}... already exists")
        return {
            "status": "duplicate",
            "message": "Payload already processed",
            "payload_id": str(existing_payload_id),
            "metrics_processed": 0,
            "metrics_skipped": len(payload.metrics),
        }

    metrics_processed = 0
    metrics_skipped = 0

    try:
        async with db.begin():
            # Insert new payload
            payload_id = uuid4()
            payload_stmt = insert(db_models.HealthPayload).values(
                id=payload_id, payload_hash=payload_hash
            )

            # Use on_conflict_do_update to handle race conditions
            payload_stmt = payload_stmt.on_conflict_do_update(
                index_elements=["payload_hash"],
                set_=dict(received_at=payload_stmt.excluded.received_at),
            ).returning(db_models.HealthPayload.id)

            result = await db.execute(payload_stmt)
            actual_payload_id = result.scalar_one()

            # If the ID differs, another process inserted it
            if actual_payload_id != payload_id:
                logger.info("Payload inserted by concurrent process")
                return {
                    "status": "duplicate",
                    "message": "Payload processed by another request",
                    "payload_id": str(actual_payload_id),
                    "metrics_processed": 0,
                    "metrics_skipped": len(payload.metrics),
                }

            # Process each metric
            for metric in payload.metrics:
                was_processed = await insert_metric_idempotent(
                    metric, actual_payload_id, db
                )
                if was_processed:
                    metrics_processed += 1
                else:
                    metrics_skipped += 1

            # Process workouts if present
            if hasattr(payload, "workouts") and payload.workouts:
                for workout in payload.workouts:
                    await insert_workout_idempotent(workout, actual_payload_id, db)

            status = "success" if metrics_skipped == 0 else "partial_success"

            logger.info(
                f"Processed payload {payload_hash[:8]}...: "
                f"{metrics_processed} metrics added, {metrics_skipped} skipped"
            )

            return {
                "status": status,
                "message": f"Data processed: {metrics_processed} new metrics, {metrics_skipped} duplicates",
                "payload_id": str(actual_payload_id),
                "metrics_processed": metrics_processed,
                "metrics_skipped": metrics_skipped,
            }

    except Exception as e:
        logger.error(f"Failed to process payload: {e}", exc_info=True)
        await db.rollback()
        raise


async def insert_metric_idempotent(
    metric: HealthMetric, payload_id: str, db: AsyncSession
) -> bool:
    """
    Insert a metric with duplicate detection

    Returns:
        bool: True if metric was inserted, False if it was a duplicate
    """

    # Generate hash for this metric's data
    metric_data_hash = generate_metric_hash(
        metric.name,
        [
            item.model_dump() if hasattr(item, "model_dump") else item
            for item in metric.data
        ],
    )

    try:
        # Check if this exact metric data already exists
        existing_metric = await db.execute(
            select(db_models.HealthMetric.id).where(
                db_models.HealthMetric.payload_id == payload_id,
                db_models.HealthMetric.name == metric.name,
                db_models.HealthMetric.data_hash == metric_data_hash,
            )
        )

        if existing_metric.scalar_one_or_none():
            logger.debug(f"Metric '{metric.name}' with same data already exists")
            return False

        # Insert metric with data hash
        metric_id = uuid4()
        metric_stmt = insert(db_models.HealthMetric).values(
            id=metric_id,
            payload_id=payload_id,
            name=metric.name,
            units=metric.units,
            data_hash=metric_data_hash,
        )

        # Handle race condition with on_conflict
        metric_stmt = metric_stmt.on_conflict_do_nothing(
            index_elements=["payload_id", "name", "data_hash"]
        ).returning(db_models.HealthMetric.id)

        result = await db.execute(metric_stmt)
        actual_metric_id = result.scalar_one_or_none()

        if not actual_metric_id:
            # Metric was inserted by another process
            return False

        # Insert metric data
        if metric.data:
            metric_type = metric.name.lower()
            db_model_cls = SPECIALIZED_DB_MODELS.get(metric_type)

            if db_model_cls is None:
                await insert_quantity_data_idempotent(metric.data, actual_metric_id, db)
            else:
                await insert_specialized_data_idempotent(
                    metric.data, actual_metric_id, db_model_cls, metric_type
                )

        return True

    except Exception as e:
        logger.warning(f"Error processing metric '{metric.name}': {e}")
        return False


async def insert_quantity_data_idempotent(
    data_entries: List[Any], metric_id: str, db: AsyncSession
):
    """Insert quantity timestamp data with duplicate handling"""

    if not data_entries:
        return

    records = []
    for entry in data_entries:
        try:
            record = {
                "id": uuid4(),
                "metric_id": metric_id,
                "date": entry.get_date(),
                "qty": entry.qty,
                "source": getattr(entry, "source", None),
            }
            records.append(record)
        except Exception as e:
            logger.warning(f"Skipping invalid quantity entry: {e}")

    if records:
        stmt = insert(db_models.QuantityTimestamp).values(records)
        stmt = stmt.on_conflict_do_update(
            index_elements=["metric_id", "date", "source"],
            set_=dict(
                qty=stmt.excluded.qty,
                # Update timestamp to track last update
                id=stmt.excluded.id,  # This ensures we keep track of updates
            ),
        )
        await db.execute(stmt)


async def insert_specialized_data_idempotent(
    data_entries: List[Any],
    metric_id: str,
    model_cls: type,
    metric_type: str,
    db: AsyncSession,
):
    """Insert specialized health data with model-specific conflict resolution"""

    if not data_entries:
        return

    records = []
    for entry in data_entries:
        try:
            record = entry.model_dump(exclude={"id"}, exclude_unset=True)
            record["id"] = uuid4()
            record["metric_id"] = metric_id
            records.append(record)
        except Exception as e:
            logger.warning(f"Skipping invalid {metric_type} entry: {e}")

    if not records:
        return

    stmt = insert(model_cls).values(records)

    # Define conflict resolution based on model type
    conflict_handlers = {
        "heart_rate": {
            "index_elements": ["metric_id", "date", "context"],
            "set_": ["min", "avg", "max", "source"],
        },
        "blood_pressure": {
            "index_elements": ["metric_id", "date"],
            "set_": ["systolic", "diastolic"],
        },
        "sleep_analysis": {
            "index_elements": ["metric_id", "start_date", "end_date"],
            "set_": ["value", "qty", "source"],
        },
        "blood_glucose": {
            "index_elements": ["metric_id", "date", "meal_time"],
            "set_": ["qty"],
        },
        "sexual_activity": {
            "index_elements": ["metric_id", "date"],
            "set_": ["unspecified", "protection_used", "protection_not_used"],
        },
        "handwashing": {
            "index_elements": ["metric_id", "date"],
            "set_": ["qty", "value"],
        },
        "toothbrushing": {
            "index_elements": ["metric_id", "date"],
            "set_": ["qty", "value"],
        },
        "insulin_delivery": {
            "index_elements": ["metric_id", "date", "reason"],
            "set_": ["qty"],
        },
        "symptom": {
            "index_elements": ["metric_id", "start", "name"],
            "set_": ["end", "severity", "user_entered", "source"],
        },
        "state_of_mind": {
            "index_elements": ["metric_id", "start", "kind"],
            "set_": [
                "end",
                "valence",
                "valence_classification",
                "labels",
                "associations",
                "metadata",
            ],
        },
        "ecg": {
            "index_elements": ["metric_id", "start"],
            "set_": [
                "end",
                "classification",
                "severity",
                "average_heart_rate",
                "number_of_voltage_measurements",
                "voltage_measurements",
                "sampling_frequency",
                "source",
            ],
        },
        "heart_rate_notifications": {
            "index_elements": ["metric_id", "start", "end"],
            "set_": ["threshold", "heart_rate", "heart_rate_variation"],
        },
    }

    handler = conflict_handlers.get(metric_type)
    if handler:
        # Build the set_ dict dynamically
        set_dict = {field: getattr(stmt.excluded, field) for field in handler["set_"]}
        stmt = stmt.on_conflict_do_update(
            index_elements=handler["index_elements"], set_=set_dict
        )
    else:
        # Default: do nothing on conflict
        stmt = stmt.on_conflict_do_nothing()

    await db.execute(stmt)


async def insert_workout_idempotent(workout: Any, payload_id: str, db: AsyncSession):
    """Insert workout data with duplicate handling"""

    try:
        workout_id = uuid4()

        # Create workout record
        workout_data = {
            "id": workout_id,
            "payload_id": payload_id,
            "name": workout.name,
            "start": workout.start,
            "end": workout.end,
            "elevation": workout.elevation.model_dump() if workout.elevation else None,
        }

        stmt = insert(db_models.Workout).values(workout_data)
        stmt = stmt.on_conflict_do_update(
            index_elements=["payload_id", "name", "start"],
            set_=dict(end=stmt.excluded.end, elevation=stmt.excluded.elevation),
        ).returning(db_models.Workout.id)

        result = await db.execute(stmt)
        actual_workout_id = result.scalar_one_or_none()

        if not actual_workout_id:
            return

        # Insert workout values
        workout_values = []
        for attr_name, attr_value in [
            ("total_energy", workout.total_energy),
            ("active_energy", workout.active_energy),
            ("max_heart_rate", workout.max_heart_rate),
            ("avg_heart_rate", workout.avg_heart_rate),
            ("step_count", workout.step_count),
            ("step_cadence", workout.step_cadence),
            ("total_swimming_stroke_count", workout.total_swimming_stroke_count),
            ("swim_cadence", workout.swim_cadence),
            ("distance", workout.distance),
            ("speed", workout.speed),
            ("flights_climbed", workout.flights_climbed),
            ("intensity", getattr(workout, "intensity", None)),
            ("temperature", workout.temperature),
            ("humidity", workout.humidity),
        ]:
            if attr_value:
                workout_values.append(
                    {
                        "id": uuid4(),
                        "workout_id": actual_workout_id,
                        "name": attr_name,
                        "qty": attr_value.qty,
                        "units": attr_value.units,
                    }
                )

        if workout_values:
            stmt = insert(db_models.WorkoutValue).values(workout_values)
            stmt = stmt.on_conflict_do_nothing(index_elements=["workout_id", "name"])
            await db.execute(stmt)

        # Insert heart rate points
        if workout.heart_rate:
            await insert_workout_points(
                workout.heart_rate, actual_workout_id, "heart_rate", db
            )

        if workout.heart_rate_recovery:
            await insert_workout_points(
                workout.heart_rate_recovery,
                actual_workout_id,
                "heart_rate_recovery",
                db,
            )

        # Insert route points
        if workout.route:
            await insert_route_points(workout.route, actual_workout_id, db)

    except Exception as e:
        logger.warning(f"Error inserting workout: {e}")


async def insert_workout_points(
    points: List[Any], workout_id: str, stream: str, db: AsyncSession
):
    """Insert workout point data (heart rate, etc.)"""

    if not points:
        return

    records = []
    for point in points:
        records.append(
            {
                "id": uuid4(),
                "workout_id": workout_id,
                "stream": stream,
                "date": point.date,
                "qty": point.qty,
                "units": point.units,
            }
        )

    if records:
        stmt = insert(db_models.WorkoutPoint).values(records)
        stmt = stmt.on_conflict_do_nothing(
            index_elements=["workout_id", "stream", "date"]
        )
        await db.execute(stmt)


async def insert_route_points(points: List[Any], workout_id: str, db: AsyncSession):
    """Insert workout route points"""

    if not points:
        return

    records = []
    for point in points:
        records.append(
            {
                "id": uuid4(),
                "workout_id": workout_id,
                "lat": point.lat,
                "lon": point.lon,
                "altitude": point.altitude,
                "timestamp": point.timestamp,
            }
        )

    if records:
        stmt = insert(db_models.WorkoutRoutePoint).values(records)
        stmt = stmt.on_conflict_do_nothing(index_elements=["workout_id", "timestamp"])
        await db.execute(stmt)
