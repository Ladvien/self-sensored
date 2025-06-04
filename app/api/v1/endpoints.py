from fastapi import APIRouter, status, Request, Depends, HTTPException
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
    """Normalize datetime strings to ISO8601 format"""

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
    """
    Receive health data from iOS Auto Export app

    This endpoint is idempotent - sending the same data multiple times
    will not create duplicates. The response will indicate whether the
    data was new, duplicate, or partially duplicate.

    Returns:
        201: Data successfully processed (new or partial)
        200: Data already exists (complete duplicate)
        400: Invalid data format
        500: Server error
    """
    try:
        # Get raw payload
        raw = await request.json()

        # Validate basic structure
        if "data" not in raw:
            raise HTTPException(
                status_code=400, detail="Missing 'data' field in payload"
            )

        # Normalize datetime formats
        normalized = normalize_datetime_strings(raw)

        # Parse the payload
        try:
            parsed = api_models.parse_payload(normalized["data"])
        except Exception as e:
            logger.error(f"Failed to parse payload: {e}")
            raise HTTPException(
                status_code=400, detail=f"Invalid payload format: {str(e)}"
            )

        logger.info(
            f"Received payload with {len(parsed.metrics)} metrics "
            f"and {len(parsed.workouts)} workouts"
        )

        # Process the data (idempotent)
        result = await insert_health_data(parsed, db, raw_payload=normalized["data"])

        # Determine response status based on result
        if result["status"] == "duplicate":
            # Complete duplicate - return 200 OK
            return JSONResponse(
                status_code=status.HTTP_200_OK,
                content={
                    "message": result["message"],
                    "payload_id": result["payload_id"],
                    "metrics_processed": result["metrics_processed"],
                    "metrics_skipped": result["metrics_skipped"],
                    "duplicate": True,
                },
            )
        elif result["status"] in ["success", "partial_success"]:
            # New or partial data - return 201 Created
            return JSONResponse(
                status_code=status.HTTP_201_CREATED,
                content={
                    "message": result["message"],
                    "payload_id": result["payload_id"],
                    "metrics_processed": result["metrics_processed"],
                    "metrics_skipped": result["metrics_skipped"],
                    "duplicate": False,
                    "partial_duplicate": result["status"] == "partial_success",
                },
            )
        else:
            # Unexpected result
            logger.error(f"Unexpected result status: {result['status']}")
            raise HTTPException(status_code=500, detail="Unexpected processing result")

    except HTTPException:
        # Re-raise FastAPI exceptions
        raise
    except Exception as e:
        logger.exception("Failed to process /sync payload")
        raise HTTPException(status_code=500, detail=f"Internal server error: {str(e)}")


@router.get("/health")
async def health_check():
    """Simple health check endpoint"""
    return {"status": "healthy", "service": "apple-health-sync"}


@router.get("/stats")
async def get_stats(db: AsyncSession = Depends(get_db)):
    """Get statistics about stored data"""
    try:
        from sqlalchemy import select, func
        from app.db.models import HealthPayload, HealthMetric, QuantityTimestamp

        # Count payloads
        payload_count = await db.scalar(select(func.count()).select_from(HealthPayload))

        # Count metrics
        metric_count = await db.scalar(select(func.count()).select_from(HealthMetric))

        # Count data points
        data_point_count = await db.scalar(
            select(func.count()).select_from(QuantityTimestamp)
        )

        # Get latest payload date
        latest_payload = await db.scalar(
            select(HealthPayload.received_at)
            .order_by(HealthPayload.received_at.desc())
            .limit(1)
        )

        return {
            "total_payloads": payload_count or 0,
            "total_metrics": metric_count or 0,
            "total_data_points": data_point_count or 0,
            "latest_sync": latest_payload.isoformat() if latest_payload else None,
        }

    except Exception as e:
        logger.error(f"Failed to get stats: {e}")
        raise HTTPException(status_code=500, detail="Failed to retrieve statistics")
