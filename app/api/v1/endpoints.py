from fastapi import APIRouter, status, Request, Depends
from fastapi.responses import JSONResponse
from sqlalchemy.ext.asyncio import AsyncSession
import logging
import re
from rich import print

import app.api.models as api_models
from app.dependencies import get_db
from app.db.insert_logic import insert_health_data

router = APIRouter()
logger = logging.getLogger(__name__)


def normalize_datetime_strings(payload: dict) -> dict:
    def fix_datetime_string(dt_str: str) -> str:
        dt_str = dt_str.strip()
        dt_str = re.sub(r"^(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2})", r"\1T\2", dt_str)
        dt_str = re.sub(r"([+-]\d{2})(\d{2})$", r"\1:\2", dt_str)
        dt_str = re.sub(r"(T\d{2}:\d{2}:\d{2})\s+([+-]\d{2}:\d{2})", r"\1\2", dt_str)
        return dt_str

    def walk(obj):
        if isinstance(obj, dict):
            return {k: walk(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [walk(i) for i in obj]
        elif isinstance(obj, str):
            if re.search(r"\d{2}:\d{2}:\d{2}", obj) and re.search(
                r"[+-]\d{2}:?\d{2}$", obj
            ):
                return fix_datetime_string(obj)
        return obj

    return walk(payload)


@router.post("/sync", status_code=status.HTTP_201_CREATED)
async def receive_health_data(request: Request, db: AsyncSession = Depends(get_db)):
    try:
        raw = await request.json()
        normalized = normalize_datetime_strings(raw)
        parsed = api_models.parse_payload(normalized["data"])

        print(
            f"Received {len(parsed.metrics)} metrics and {len(parsed.workouts)} workouts"
        )

        await insert_health_data(parsed, db)

        return {"message": "Data received and stored successfully"}
    except Exception as e:
        logger.exception("Failed to process /sync payload")
        return JSONResponse(status_code=400, content={"error": str(e)})
