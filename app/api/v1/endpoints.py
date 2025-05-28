from fastapi import APIRouter, status
from pydantic import BaseModel
from app.db.models import insert_health_data
from app.db.database import SessionLocal

router = APIRouter()


class HealthDataPayload(BaseModel):
    timestamp: str
    type: str
    value: float
    unit: str


@router.post("/health-data", status_code=status.HTTP_201_CREATED)
async def receive_health_data(payload: HealthDataPayload):
    db = SessionLocal()
    try:
        insert_health_data(payload, db)
    finally:
        db.close()
    return {"message": "Data received"}
