from fastapi import APIRouter, status, Request
from fastapi.responses import JSONResponse
from app.models.health import parse_payload
import logging
import re
from rich import print

router = APIRouter()
logger = logging.getLogger(__name__)

# ---- Timezone Normalizer ----


def normalize_datetime_strings(payload: dict) -> dict:
    def fix_datetime_string(dt_str: str) -> str:
        dt_str = dt_str.strip()

        # Fix 1: Insert "T" between date and time if missing
        dt_str = re.sub(r"^(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2})", r"\1T\2", dt_str)

        # Fix 2: Ensure timezone has colon (e.g., -0500 → -05:00)
        dt_str = re.sub(r"([+-]\d{2})(\d{2})$", r"\1:\2", dt_str)

        # Fix 3: Remove space between time and timezone if present (e.g., T18:08:49 -05:00 → T18:08:49-05:00)
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


# ---- /sync Endpoint ----


@router.post("/sync", status_code=status.HTTP_201_CREATED)
async def receive_health_data(request: Request):
    try:
        raw = await request.json()
        normalized = normalize_datetime_strings(raw)
        parsed_data = parse_payload(normalized["data"])

        print(
            f"Received {len(parsed_data.metrics)} metrics and {len(parsed_data.workouts)} workouts"
        )

        print(parsed_data.metrics)

        # Insert storage logic here
        return {"message": "Data received successfully"}
    except Exception as e:
        logger.exception("Failed to parse payload")
        return JSONResponse(status_code=400, content={"error": str(e)})
