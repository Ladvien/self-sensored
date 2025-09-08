from fastapi import FastAPI
import os
from sqlalchemy.ext.asyncio import create_async_engine
from dotenv import load_dotenv
from contextlib import asynccontextmanager

from app.logging_config import setup_logging
from app.api.v1.endpoints import router as api_router
from app import db

load_dotenv()

# Update DATABASE_URL to use STORY-008 infrastructure
DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgresql+asyncpg://health_user:dev_password_123@localhost:5432/health_export_dev",
)

engine = create_async_engine(DATABASE_URL, echo=False)


async def run_schema_ddl():
    await db.create_tables(engine)


@asynccontextmanager
async def lifespan(app: FastAPI):
    setup_logging()
    await run_schema_ddl()
    yield


app = FastAPI(lifespan=lifespan)
app.include_router(api_router, prefix="/api/v1")
