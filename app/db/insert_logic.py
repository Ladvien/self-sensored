# app/db/insert_logic.py

from app.db.models import HealthPayload, HealthMetric, QuantityTimestamp
from sqlalchemy.ext.asyncio import AsyncSession
from uuid import uuid4
import logging

import app.db.models as db_models

logger = logging.getLogger(__name__)

# Map schema keys to SQLAlchemy model classes
SPECIALIZED_DB_MODELS = {
    "blood_pressure": db_models.BloodPressure,
    "heart_rate": db_models.HeartRate,
    "sleep_analysis": db_models.SleepAnalysis,
    # "blood_glucose": db_models.BloodGlucose,
    # "sexual_activity": db_models.SexualActivity,
    # "handwashing": db_models.HygieneEvent,
    # "toothbrushing": db_models.HygieneEvent,
    # "insulin_delivery": db_models.InsulinDelivery,
    # "heart_rate_notifications": db_models.HeartRateNotification,
    # "symptoms": db_models.Symptom,
    # "state_of_mind": db_models.StateOfMind,
    # "ecg": db_models.ECG,
}


async def insert_health_data(payload: HealthPayload, db: AsyncSession):
    payload_obj = HealthPayload()
    db.add(payload_obj)
    await db.flush()

    for metric in payload.metrics:
        metric_obj = HealthMetric(
            id=uuid4(), name=metric.name, units=metric.units, payload=payload_obj
        )
        db.add(metric_obj)

        metric_type = metric.name.lower()
        db_model_cls = SPECIALIZED_DB_MODELS.get(metric_type)

        if not metric.data:
            continue

        try:
            if db_model_cls is None:
                # Default to QuantityTimestamp
                db_objs = [
                    QuantityTimestamp(
                        metric=metric_obj,
                        date=entry.get_date(),
                        qty=entry.qty,
                        source=entry.source,
                    )
                    for entry in metric.data
                ]
            else:
                # Dynamically insert specialized model
                db_objs = [
                    db_model_cls(
                        metric=metric_obj,
                        **entry.model_dump(exclude={"id"}, exclude_unset=True),
                    )
                    for entry in metric.data
                ]

            db.add_all(db_objs)
            await db.flush()

        except Exception as e:
            await db.rollback()
            logger.warning(f"Skipping metric '{metric.name}' due to error: {e}")

        await db.commit()
