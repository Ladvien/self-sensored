from contextlib import asynccontextmanager
from app.db.database import AsyncSessionLocal


@asynccontextmanager
async def get_db():
    async with AsyncSessionLocal() as session:
        yield session
