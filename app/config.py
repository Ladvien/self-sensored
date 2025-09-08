# app/config.py - Centralized configuration management

import os
from typing import Optional, List, Dict, Any
from pydantic import BaseSettings, validator, Field
from functools import lru_cache
import logging


class DatabaseSettings(BaseSettings):
    """Database configuration"""

    # Connection settings
    url: str = Field(
        default="postgresql+asyncpg://health_user:dev_password_123@localhost:5432/health_export_dev",
        env="DATABASE_URL",
    )

    # Pool settings
    pool_size: int = Field(default=20, env="DB_POOL_SIZE")
    max_overflow: int = Field(default=30, env="DB_MAX_OVERFLOW")
    pool_timeout: int = Field(default=30, env="DB_POOL_TIMEOUT")
    pool_recycle: int = Field(default=3600, env="DB_POOL_RECYCLE")

    # Query settings
    query_timeout: int = Field(default=60, env="DB_QUERY_TIMEOUT")
    echo_queries: bool = Field(default=False, env="DB_ECHO")

    # Batch processing
    default_batch_size: int = Field(default=1000, env="DB_BATCH_SIZE")
    max_batch_size: int = Field(default=5000, env="DB_MAX_BATCH_SIZE")

    class Config:
        env_prefix = "DB_"


class APISettings(BaseSettings):
    """API configuration"""

    # Server settings
    host: str = Field(default="0.0.0.0", env="API_HOST")
    port: int = Field(default=8000, env="API_PORT")
    debug: bool = Field(default=False, env="API_DEBUG")

    # Request limits
    max_payload_size: int = Field(
        default=50 * 1024 * 1024, env="API_MAX_PAYLOAD_SIZE"
    )  # 50MB
    request_timeout: int = Field(default=300, env="API_REQUEST_TIMEOUT")  # 5 minutes

    # CORS settings
    cors_origins: List[str] = Field(default=["*"], env="API_CORS_ORIGINS")
    cors_methods: List[str] = Field(
        default=["GET", "POST", "DELETE"], env="API_CORS_METHODS"
    )

    # Rate limiting
    rate_limit_enabled: bool = Field(default=False, env="API_RATE_LIMIT_ENABLED")
    rate_limit_requests: int = Field(default=100, env="API_RATE_LIMIT_REQUESTS")
    rate_limit_window: int = Field(default=3600, env="API_RATE_LIMIT_WINDOW")  # 1 hour

    @validator("cors_origins", pre=True)
    def parse_cors_origins(cls, v):
        if isinstance(v, str):
            return [origin.strip() for origin in v.split(",")]
        return v

    class Config:
        env_prefix = "API_"


class LoggingSettings(BaseSettings):
    """Logging configuration"""

    level: str = Field(default="INFO", env="LOG_LEVEL")
    format: str = Field(
        default="%(asctime)s - %(name)s - %(levelname)s - %(message)s", env="LOG_FORMAT"
    )

    # File logging
    file_enabled: bool = Field(default=True, env="LOG_FILE_ENABLED")
    file_path: str = Field(default="logs", env="LOG_ABSOLUTE_DIRECTORY_PATH")
    file_max_size: int = Field(
        default=50 * 1024 * 1024, env="LOG_FILE_MAX_SIZE"
    )  # 50MB
    file_backup_count: int = Field(default=10, env="LOG_FILE_BACKUP_COUNT")

    # Console logging
    console_enabled: bool = Field(default=True, env="LOG_CONSOLE_ENABLED")

    # Error logging
    error_file_enabled: bool = Field(default=True, env="LOG_ERROR_FILE_ENABLED")
    error_file_max_size: int = Field(
        default=10 * 1024 * 1024, env="LOG_ERROR_FILE_MAX_SIZE"
    )  # 10MB
    error_file_backup_count: int = Field(default=5, env="LOG_ERROR_FILE_BACKUP_COUNT")

    @validator("level")
    def validate_log_level(cls, v):
        valid_levels = ["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]
        if v.upper() not in valid_levels:
            raise ValueError(f"Invalid log level. Must be one of: {valid_levels}")
        return v.upper()

    class Config:
        env_prefix = "LOG_"


class ProcessingSettings(BaseSettings):
    """Data processing configuration"""

    # Batch processing
    enable_batch_processing: bool = Field(default=True, env="PROCESSING_BATCH_ENABLED")
    batch_size: int = Field(default=1000, env="PROCESSING_BATCH_SIZE")
    max_workers: int = Field(default=4, env="PROCESSING_MAX_WORKERS")

    # Data validation
    strict_validation: bool = Field(default=True, env="PROCESSING_STRICT_VALIDATION")
    skip_invalid_entries: bool = Field(default=True, env="PROCESSING_SKIP_INVALID")

    # Deduplication
    enable_deduplication: bool = Field(default=True, env="PROCESSING_DEDUP_ENABLED")
    hash_algorithm: str = Field(default="sha256", env="PROCESSING_HASH_ALGORITHM")

    # Performance tuning
    use_bulk_copy: bool = Field(default=False, env="PROCESSING_USE_BULK_COPY")
    bulk_copy_threshold: int = Field(
        default=10000, env="PROCESSING_BULK_COPY_THRESHOLD"
    )

    @validator("hash_algorithm")
    def validate_hash_algorithm(cls, v):
        valid_algorithms = ["md5", "sha1", "sha256", "sha512"]
        if v.lower() not in valid_algorithms:
            raise ValueError(
                f"Invalid hash algorithm. Must be one of: {valid_algorithms}"
            )
        return v.lower()

    class Config:
        env_prefix = "PROCESSING_"


class MonitoringSettings(BaseSettings):
    """Monitoring and observability configuration"""

    # Health checks
    health_check_enabled: bool = Field(
        default=True, env="MONITORING_HEALTH_CHECK_ENABLED"
    )
    health_check_interval: int = Field(
        default=30, env="MONITORING_HEALTH_CHECK_INTERVAL"
    )

    # Metrics
    metrics_enabled: bool = Field(default=True, env="MONITORING_METRICS_ENABLED")
    metrics_endpoint: str = Field(default="/metrics", env="MONITORING_METRICS_ENDPOINT")

    # Request tracking
    track_requests: bool = Field(default=True, env="MONITORING_TRACK_REQUESTS")
    slow_request_threshold: float = Field(
        default=5.0, env="MONITORING_SLOW_REQUEST_THRESHOLD"
    )

    # Database monitoring
    monitor_db_performance: bool = Field(default=True, env="MONITORING_DB_PERFORMANCE")
    db_stats_interval: int = Field(
        default=300, env="MONITORING_DB_STATS_INTERVAL"
    )  # 5 minutes

    class Config:
        env_prefix = "MONITORING_"


class Settings(BaseSettings):
    """Main application settings"""

    # Environment
    environment: str = Field(default="development", env="ENVIRONMENT")
    debug: bool = Field(default=False, env="DEBUG")

    # Application info
    app_name: str = Field(default="Apple Health Sync API", env="APP_NAME")
    app_version: str = Field(default="1.0.0", env="APP_VERSION")
    app_description: str = Field(
        default="REST API for Apple Health data synchronization", env="APP_DESCRIPTION"
    )

    # Component settings
    database: DatabaseSettings = DatabaseSettings()
    api: APISettings = APISettings()
    logging: LoggingSettings = LoggingSettings()
    processing: ProcessingSettings = ProcessingSettings()
    monitoring: MonitoringSettings = MonitoringSettings()

    @validator("environment")
    def validate_environment(cls, v):
        valid_envs = ["development", "staging", "production", "testing"]
        if v.lower() not in valid_envs:
            raise ValueError(f"Invalid environment. Must be one of: {valid_envs}")
        return v.lower()

    @property
    def is_production(self) -> bool:
        return self.environment == "production"

    @property
    def is_development(self) -> bool:
        return self.environment == "development"

    class Config:
        env_file = ".env"
        env_file_encoding = "utf-8"
        case_sensitive = False


@lru_cache()
def get_settings() -> Settings:
    """Get cached settings instance"""
    return Settings()


# Example .env file content
ENV_TEMPLATE = """
# Environment Configuration
ENVIRONMENT=development
DEBUG=false

# Database Settings
DATABASE_URL=postgresql+asyncpg://user:password@localhost:5432/dbname
DB_POOL_SIZE=20
DB_MAX_OVERFLOW=30
DB_BATCH_SIZE=1000

# API Settings
API_HOST=0.0.0.0
API_PORT=8000
API_MAX_PAYLOAD_SIZE=52428800
API_CORS_ORIGINS=*

# Logging Settings
LOG_LEVEL=INFO
LOG_ABSOLUTE_DIRECTORY_PATH=./logs
LOG_FILE_ENABLED=true

# Processing Settings
PROCESSING_BATCH_ENABLED=true
PROCESSING_BATCH_SIZE=1000
PROCESSING_STRICT_VALIDATION=true

# Monitoring Settings
MONITORING_HEALTH_CHECK_ENABLED=true
MONITORING_METRICS_ENABLED=true
MONITORING_TRACK_REQUESTS=true
"""


def create_env_file(path: str = ".env") -> None:
    """Create example .env file"""
    if not os.path.exists(path):
        with open(path, "w") as f:
            f.write(ENV_TEMPLATE)
        print(f"Created example environment file: {path}")
    else:
        print(f"Environment file already exists: {path}")


if __name__ == "__main__":
    # Create example .env file if run directly
    create_env_file()

    # Print current configuration
    settings = get_settings()
    print(f"Environment: {settings.environment}")
    print(f"Database URL: {settings.database.url}")
    print(f"API Port: {settings.api.port}")
    print(f"Debug Mode: {settings.debug}")
