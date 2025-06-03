# app/db/models.py

from sqlalchemy import (
    Column,
    DateTime,
    Float,
    String,
    Text,
    JSON,
    ARRAY,
    UUID,
    ForeignKey,
)
from sqlalchemy.orm import declarative_base, relationship
from sqlalchemy.sql import func

Base = declarative_base()


class HealthPayload(Base):
    __tablename__ = "health_payload"
    __table_args__ = {"schema": "apple_health"}

    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())
    received_at = Column(DateTime(timezone=True), server_default=func.now())
    metrics = relationship(
        "HealthMetric", back_populates="payload", cascade="all, delete-orphan"
    )


class HealthMetric(Base):
    __tablename__ = "health_metric"
    __table_args__ = {"schema": "apple_health"}

    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())
    payload_id = Column(
        UUID, ForeignKey("apple_health.health_payload.id", ondelete="CASCADE")
    )
    name = Column(Text, nullable=False)
    units = Column(Text, nullable=False)
    payload = relationship("HealthPayload", back_populates="metrics")
    quantity_data = relationship(
        "QuantityTimestamp", back_populates="metric", cascade="all, delete-orphan"
    )


class QuantityTimestamp(Base):
    __tablename__ = "quantity_timestamp"
    __table_args__ = {"schema": "apple_health"}

    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())
    metric_id = Column(
        UUID, ForeignKey("apple_health.health_metric.id", ondelete="CASCADE")
    )
    date = Column(DateTime(timezone=True), nullable=False)
    qty = Column(Float, nullable=False)
    source = Column(Text)
    metric = relationship("HealthMetric", back_populates="quantity_data")
