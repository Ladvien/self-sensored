# app/db/init.py

from app.db.models import Base


async def create_tables(engine, ddl_path: str = "app/db/schema.sql"):
    async with engine.begin() as conn:
        with open(ddl_path, "r", encoding="utf-8") as ddl_file:
            queries = ddl_file.read().split(";")

            # Ensure the schema exists before executing queries
            await conn.exec_driver_sql("CREATE SCHEMA IF NOT EXISTS apple_health")
            for query in queries:
                await conn.exec_driver_sql(query.strip())
