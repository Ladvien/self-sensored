from fastapi import FastAPI
from app.api.v1.endpoints import router as api_router
import os
from sqlalchemy.ext.asyncio import create_async_engine
from dotenv import load_dotenv
import asyncio

load_dotenv()

DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgresql+asyncpg://self_sensored_user:S3curePa$$123@192.168.1.104:5432/self_sensored",
)


app = FastAPI()

app.include_router(api_router, prefix="/api/v1")

# Path to your SQL DDL file
DDL_PATH = "app/db/schema.sql"

engine = create_async_engine(DATABASE_URL, echo=False)


async def run_schema_ddl():
    async with engine.begin() as conn:
        with open(DDL_PATH, "r", encoding="utf-8") as ddl_file:

            queries = ddl_file.read().split(";")

            for query in queries:
                await conn.exec_driver_sql(query.strip())


@app.on_event("startup")
async def initialize_database():
    await run_schema_ddl()
