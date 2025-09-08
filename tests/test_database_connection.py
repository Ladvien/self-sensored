import pytest
import asyncio
from sqlalchemy.ext.asyncio import create_async_engine
from sqlalchemy import text
import os

DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgresql+asyncpg://health_user:dev_password_123@localhost:5432/health_export_dev"
)


@pytest.mark.asyncio
async def test_database_connection():
    """Test that we can connect to the database"""
    engine = create_async_engine(DATABASE_URL)
    
    async with engine.begin() as conn:
        result = await conn.execute(text('SELECT version()'))
        version = result.fetchone()
        assert version is not None
        assert 'PostgreSQL' in version[0]
    
    await engine.dispose()


@pytest.mark.asyncio
async def test_apple_health_schema_exists():
    """Test that apple_health schema was created"""
    engine = create_async_engine(DATABASE_URL)
    
    async with engine.begin() as conn:
        result = await conn.execute(
            text("SELECT schema_name FROM information_schema.schemata WHERE schema_name = 'apple_health'")
        )
        schema = result.fetchone()
        assert schema is not None
        assert schema[0] == 'apple_health'
    
    await engine.dispose()