# app/logging_config.py
import logging
import logging.handlers
from pathlib import Path
import os
from dotenv import load_dotenv

load_dotenv()


def setup_logging():
    """Configure logging to both journal and NAS file"""

    # Create logs directory on NAS mount or log to this folder
    project_dir = Path(__file__).resolve().parent.parent
    nas_log_dir = Path(
        os.getenv(
            "LOG_ABSOLUTE_DIRECTORY_PATH",
            project_dir / "logs",  # Default to logs folder in project directory
        )
    )

    nas_log_dir.mkdir(parents=True, exist_ok=True)

    # Configure root logger
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        handlers=[
            # Console handler (goes to systemd journal)
            logging.StreamHandler(),
            # File handler on NAS with rotation
            logging.handlers.RotatingFileHandler(
                nas_log_dir / "health-pipeline.log",
                maxBytes=50 * 1024 * 1024,  # 50MB
                backupCount=10,
                encoding="utf-8",
            ),
            # Separate error log
            logging.handlers.RotatingFileHandler(
                nas_log_dir / "health-pipeline-errors.log",
                maxBytes=10 * 1024 * 1024,  # 10MB
                backupCount=5,
                encoding="utf-8",
            ),
        ],
    )

    # Set error handler to only log errors
    error_handler = logging.getLogger().handlers[-1]
    error_handler.setLevel(logging.ERROR)
