# app/api/v1/endpoints.py - Improved with better error handling and performance

from fastapi import APIRouter, status, Request, Depends, HTTPException, BackgroundTasks
from fastapi.responses import JSONResponse
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.exc import SQLAlchemyError, IntegrityError
from sqlalchemy import select, func, text
import logging
import re
import time
from typing import Dict, Any, Optional
from contextlib import asynccontextmanager

import app.api.models as api_models
from app.dependencies import get_db
from app.db.insert_logic import insert_health_data  # Use your original insert_logic
from app.db import models as db_models

router = APIRouter()
logger = logging.getLogger(__name__)


class DataProcessingError(Exception):
    """Custom exception for data processing errors"""

    def __init__(self, message: str, details: Optional[Dict] = None):
        self.message = message
        self.details = details or {}
        super().__init__(self.message)


def normalize_datetime_strings(payload: dict) -> dict:
    """
    Normalize datetime strings to ISO8601 format with improved regex patterns
    """

    def fix_datetime_string(dt_str: str) -> str:
        dt_str = dt_str.strip()

        # Convert space-separated date/time to T format
        dt_str = re.sub(r"^(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2})", r"\1T\2", dt_str)

        # Fix timezone format (add colon between hours and minutes)
        dt_str = re.sub(r"([+-]\d{2})(\d{2})$", r"\1:\2", dt_str)

        # Remove extra spaces between time and timezone
        dt_str = re.sub(r"(T\d{2}:\d{2}:\d{2})\s+([+-]\d{2}:\d{2})", r"\1\2", dt_str)

        return dt_str

    def walk(obj):
        if isinstance(obj, dict):
            return {k: walk(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [walk(i) for i in obj]
        elif isinstance(obj, str):
            # More precise datetime detection
            if re.search(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}", obj) and re.search(
                r"[+-]\d{2}:?\d{2}$", obj
            ):
                return fix_datetime_string(obj)
        return obj

    return walk(payload)


@asynccontextmanager
async def request_timing():
    """Context manager to track request processing time"""
    start_time = time.time()
    try:
        yield
    finally:
        processing_time = time.time() - start_time
        logger.info(f"Request processed in {processing_time:.3f}s")


async def validate_payload_structure(raw_payload: dict) -> dict:
    """
    Validate payload structure with detailed error reporting
    """
    if not isinstance(raw_payload, dict):
        raise DataProcessingError("Payload must be a JSON object")

    if "data" not in raw_payload:
        raise DataProcessingError("Missing 'data' field in payload")

    data = raw_payload["data"]
    if not isinstance(data, dict):
        raise DataProcessingError("'data' field must be an object")

    if "metrics" not in data:
        raise DataProcessingError("Missing 'metrics' field in data")

    if not isinstance(data["metrics"], list):
        raise DataProcessingError("'metrics' field must be an array")

    # Validate basic metric structure
    for i, metric in enumerate(data["metrics"]):
        if not isinstance(metric, dict):
            raise DataProcessingError(f"Metric {i} must be an object")

        required_fields = ["name", "units", "data"]
        for field in required_fields:
            if field not in metric:
                raise DataProcessingError(f"Metric {i} missing required field: {field}")

        if not isinstance(metric["data"], list):
            raise DataProcessingError(f"Metric {i} 'data' field must be an array")

    return data


@router.post("/sync", status_code=status.HTTP_201_CREATED)
async def receive_health_data(
    request: Request,
    background_tasks: BackgroundTasks,
    db: AsyncSession = Depends(get_db),
):
    """
    Receive health data from iOS Auto Export app with improved error handling

    This endpoint is idempotent - sending the same data multiple times
    will not create duplicates. The response indicates processing status.

    Returns:
        201: Data successfully processed (new or partial)
        200: Data already exists (complete duplicate)
        400: Invalid data format or structure
        422: Valid format but data validation errors
        500: Server error
    """
    async with request_timing():
        try:
            # Parse raw payload with size limit check
            raw = await request.json()

            # Basic size validation (adjust as needed)
            payload_size = len(str(raw))
            if payload_size > 50 * 1024 * 1024:  # 50MB limit
                raise HTTPException(
                    status_code=413, detail="Payload too large (max 50MB)"
                )

            # Validate structure
            try:
                data = await validate_payload_structure(raw)
            except DataProcessingError as e:
                raise HTTPException(
                    status_code=400, detail=f"Invalid payload structure: {e.message}"
                )

            # Normalize datetime formats
            normalized = normalize_datetime_strings(raw)

            # Parse the payload with detailed error tracking
            try:
                parsed = api_models.parse_payload(normalized["data"])
            except Exception as e:
                logger.error(f"Failed to parse payload: {e}", exc_info=True)
                raise HTTPException(
                    status_code=422, detail=f"Data validation failed: {str(e)}"
                )

            logger.info(
                f"Received payload: {len(parsed.metrics)} metrics, "
                f"{len(parsed.workouts)} workouts, {payload_size} bytes"
            )

            # Process the data using optimized batch operations
            try:
                result = await insert_health_data(
                    parsed, db, raw_payload=normalized["data"]
                )
            except IntegrityError as e:
                logger.error(f"Database integrity error: {e}")
                raise HTTPException(
                    status_code=409, detail="Data conflicts with existing records"
                )
            except SQLAlchemyError as e:
                logger.error(f"Database error: {e}")
                raise HTTPException(status_code=500, detail="Database operation failed")

            # Log result for monitoring
            logger.info(
                f"Processing result: {result['status']}, "
                f"processed: {result['metrics_processed']}, "
                f"skipped: {result['metrics_skipped']}"
            )

            # Determine response based on result
            if result["status"] == "duplicate":
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
                # Schedule background cleanup if needed
                if result["status"] == "partial_success":
                    background_tasks.add_task(log_partial_success_details, result)

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
                logger.error(f"Unexpected result status: {result['status']}")
                raise HTTPException(
                    status_code=500, detail="Unexpected processing result"
                )

        except HTTPException:
            # Re-raise FastAPI exceptions as-is
            raise
        except Exception as e:
            logger.exception("Unexpected error in /sync endpoint")
            raise HTTPException(
                status_code=500, detail=f"Internal server error: {str(e)}"
            )


async def log_partial_success_details(result: Dict[str, Any]):
    """Background task to log details about partial success"""
    logger.info(
        f"Partial success details for payload {result['payload_id']}: "
        f"Some metrics were duplicates or failed validation"
    )


@router.get("/health")
async def health_check(db: AsyncSession = Depends(get_db)):
    """
    Enhanced health check with database connectivity test
    """
    try:
        # Test database connectivity
        result = await db.execute(text("SELECT 1"))
        db_healthy = result.scalar() == 1

        return {
            "status": "healthy" if db_healthy else "degraded",
            "service": "apple-health-sync",
            "database": "connected" if db_healthy else "disconnected",
            "timestamp": time.time(),
        }
    except Exception as e:
        logger.error(f"Health check failed: {e}")
        return {
            "status": "unhealthy",
            "service": "apple-health-sync",
            "database": "error",
            "error": str(e),
            "timestamp": time.time(),
        }


@router.get("/stats")
async def get_stats(db: AsyncSession = Depends(get_db)):
    """
    Get comprehensive statistics about stored data
    """
    try:
        # Use optimized queries with proper joins
        stats_query = text(
            """
            SELECT 
                (SELECT COUNT(*) FROM apple_health.health_payload) as total_payloads,
                (SELECT COUNT(*) FROM apple_health.health_metric) as total_metrics,
                (SELECT COUNT(*) FROM apple_health.quantity_timestamp) as total_data_points,
                (SELECT COUNT(*) FROM apple_health.workout) as total_workouts,
                (SELECT MAX(received_at) FROM apple_health.health_payload) as latest_sync,
                (SELECT COUNT(DISTINCT name) FROM apple_health.health_metric) as unique_metrics
        """
        )

        result = await db.execute(stats_query)
        row = result.fetchone()

        # Get top metrics by data volume
        top_metrics_query = text(
            """
            SELECT 
                hm.name,
                COUNT(qt.id) as data_points
            FROM apple_health.health_metric hm
            LEFT JOIN apple_health.quantity_timestamp qt ON hm.id = qt.metric_id
            GROUP BY hm.name
            ORDER BY data_points DESC
            LIMIT 10
        """
        )

        top_metrics_result = await db.execute(top_metrics_query)
        top_metrics = [
            {"name": row[0], "data_points": row[1]}
            for row in top_metrics_result.fetchall()
        ]

        return {
            "total_payloads": row[0] or 0,
            "total_metrics": row[1] or 0,
            "total_data_points": row[2] or 0,
            "total_workouts": row[3] or 0,
            "unique_metric_types": row[5] or 0,
            "latest_sync": row[4].isoformat() if row[4] else None,
            "top_metrics": top_metrics,
        }

    except Exception as e:
        logger.error(f"Failed to get stats: {e}")
        raise HTTPException(status_code=500, detail="Failed to retrieve statistics")


@router.get("/metrics")
async def get_metric_types(db: AsyncSession = Depends(get_db)):
    """
    Get list of all metric types with their data counts
    """
    try:
        query = text(
            """
            SELECT 
                hm.name,
                hm.units,
                COUNT(DISTINCT hm.id) as metric_instances,
                MIN(hp.received_at) as first_seen,
                MAX(hp.received_at) as last_seen
            FROM apple_health.health_metric hm
            JOIN apple_health.health_payload hp ON hm.payload_id = hp.id
            GROUP BY hm.name, hm.units
            ORDER BY metric_instances DESC
        """
        )

        result = await db.execute(query)
        metrics = [
            {
                "name": row[0],
                "units": row[1],
                "instances": row[2],
                "first_seen": row[3].isoformat() if row[3] else None,
                "last_seen": row[4].isoformat() if row[4] else None,
            }
            for row in result.fetchall()
        ]

        return {"metrics": metrics, "total_types": len(metrics)}

    except Exception as e:
        logger.error(f"Failed to get metric types: {e}")
        raise HTTPException(status_code=500, detail="Failed to retrieve metric types")


@router.delete("/data/{payload_id}")
async def delete_payload(payload_id: str, db: AsyncSession = Depends(get_db)):
    """
    Delete a specific payload and all associated data
    """
    try:
        # Verify payload exists
        result = await db.execute(
            select(db_models.HealthPayload).where(
                db_models.HealthPayload.id == payload_id
            )
        )
        payload = result.scalar_one_or_none()

        if not payload:
            raise HTTPException(status_code=404, detail="Payload not found")

        # Delete payload (cascades to all related data)
        try:
            await db.delete(payload)
            await db.commit()

        except Exception as e:
            logger.error(f"Transaction failed: {e}")
            await db.rollback()
            raise

        logger.info(f"Deleted payload {payload_id}")

        return {
            "message": f"Payload {payload_id} and all associated data deleted",
            "deleted_at": time.time(),
        }

    except HTTPException:
        raise
    except Exception as e:
        await db.rollback()
        logger.error(f"Failed to delete payload {payload_id}: {e}")
        raise HTTPException(status_code=500, detail="Failed to delete payload")
