# app/db/insert_logic.py

from app.db.models import HealthPayload, HealthMetric, QuantityTimestamp
from sqlalchemy.ext.asyncio import AsyncSession
from uuid import uuid4


async def insert_health_data(payload: HealthPayload, db: AsyncSession):
    payload_obj = HealthPayload()

    metric_objs = []
    for metric in payload.metrics:
        metric_id = uuid4()
        metric_obj = HealthMetric(
            id=metric_id, name=metric.name, units=metric.units, payload=payload_obj
        )
        metric_objs.append(metric_obj)

        if isinstance(metric.data[0], QuantityTimestamp):
            quantity_objs = [
                QuantityTimestamp(
                    metric=metric_obj, date=q.date, qty=q.qty, source=q.source
                )
                for q in metric.data
            ]
            db.add_all(quantity_objs)
        else:
            # TODO: insert specialized data similarly
            pass

    db.add(payload_obj)
    db.add_all(metric_objs)
    await db.commit()
