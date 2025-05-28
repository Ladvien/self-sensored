import asyncio
import asyncpg
import os
from dotenv import load_dotenv

load_dotenv()

DATABASE_URL = os.environ["DATABASE_URL"]


async def test_connection():
    try:
        conn = await asyncpg.connect(DATABASE_URL)
        print("✅ Connected to the PostgreSQL database successfully!")

        # Optional: Run a simple query
        result = await conn.fetch("SELECT current_database(), current_user;")
        print("📄 Query result:", result)

        await conn.close()
    except Exception as e:
        print("❌ Failed to connect or query the database.")
        print(e)


if __name__ == "__main__":
    asyncio.run(test_connection())
