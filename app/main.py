from fastapi import FastAPI
import os
from sqlalchemy.ext.asyncio import create_async_engine
from dotenv import load_dotenv
from contextlib import asynccontextmanager

from app.logging_config import setup_logging
from app.api.v1.endpoints import router as api_router
from app import db

load_dotenv()


DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgresql+asyncpg://self_sensored_user:S3curePa$$123@192.168.1.104:5432/self_sensored",
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
