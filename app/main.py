from fastapi import FastAPI, Depends
from sqlalchemy.ext.asyncio import AsyncSession
from app.dependencies import get_db
from app.db.database import create_db_and_tables

app = FastAPI()


@app.on_event("startup")
async def on_startup():
    await create_db_and_tables()


@app.get("/health")
async def health_check(db: AsyncSession = Depends(get_db)):
    result = await db.execute("SELECT 1")
    return {"ok": result.scalar_one() == 1}
