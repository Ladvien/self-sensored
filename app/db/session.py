from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
import os
from dotenv import load_dotenv

load_dotenv()


DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgresql+asyncpg://health_user:dev_password_123@localhost:5432/health_export_dev",
)

engine = create_async_engine(
    DATABASE_URL,
    pool_size=20,  # Increase connection pool
    max_overflow=30,  # Allow more overflow connections
    pool_pre_ping=True,  # Verify connections
    pool_recycle=3600,  # Recycle connections hourly
    echo=False,
    connect_args={
        "command_timeout": 60,
        "server_settings": {
            "application_name": "health_pipeline",
            "jit": "off",  # Disable JIT for bulk operations
        },
    },
)
AsyncSessionLocal = async_sessionmaker(bind=engine, expire_on_commit=False)
