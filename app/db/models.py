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


class AppleHealthMixin:
    __table_args__ = {"schema": "apple_health"}
    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())


class HealthPayload(Base, AppleHealthMixin):
    __tablename__ = "health_payload"
    __table_args__ = {"schema": "apple_health"}

    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())
    received_at = Column(DateTime(timezone=True), server_default=func.now())
    metrics = relationship(
        "HealthMetric", back_populates="payload", cascade="all, delete-orphan"
    )

    def __repr__(self):
        return f"<HealthPayload(id={self.id}, received_at={self.received_at})>"


class HealthMetric(Base, AppleHealthMixin):
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
    blood_pressure = relationship(
        "BloodPressure", backref="metric", cascade="all, delete-orphan"
    )
    heart_rate = relationship(
        "HeartRate", backref="metric", cascade="all, delete-orphan"
    )
    sleep_analysis = relationship(
        "SleepAnalysis", backref="metric", cascade="all, delete-orphan"
    )

    def __repr__(self):
        return f"<HealthMetric(id={self.id}, name={self.name}, units={self.units})>"


class QuantityTimestamp(Base, AppleHealthMixin):
    __tablename__ = "quantity_timestamp"
    __table_args__ = {"schema": "apple_health"}

    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())
    date = Column(
        DateTime(timezone=True), primary_key=True, nullable=False
    )  # Part of composite PK
    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    qty = Column(Float, nullable=False)
    source = Column(Text)

    # Relationship to HealthMetric
    metric = relationship("HealthMetric", back_populates="quantity_data")

    def __repr__(self):
        return f"<QuantityTimestamp(id={self.id}, date={self.date}, qty={self.qty}, source={self.source})>"


class BloodPressure(Base, AppleHealthMixin):
    __tablename__ = "blood_pressure"
    __table_args__ = {"schema": "apple_health"}

    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())
    metric_id = Column(
        UUID, ForeignKey("apple_health.health_metric.id", ondelete="CASCADE")
    )
    date = Column(DateTime(timezone=True), nullable=False)
    systolic = Column(Float, nullable=False)
    diastolic = Column(Float, nullable=False)

    def __repr__(self):
        return f"<BloodPressure(id={self.id}, date={self.date}, systolic={self.systolic}, diastolic={self.diastolic})>"


class HeartRate(Base, AppleHealthMixin):
    __tablename__ = "heart_rate"
    __table_args__ = {"schema": "apple_health"}

    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())
    metric_id = Column(
        UUID, ForeignKey("apple_health.health_metric.id", ondelete="CASCADE")
    )
    date = Column(DateTime(timezone=True), nullable=False)
    min = Column(Float)
    avg = Column(Float)
    max = Column(Float)
    context = Column(Text)
    source = Column(Text)

    def __repr__(self):
        return f"<HeartRate(id={self.id}, date={self.date}, min={self.min}, avg={self.avg}, max={self.max}, context={self.context}, source={self.source})>"


class SleepAnalysis(Base, AppleHealthMixin):
    __tablename__ = "sleep_analysis"
    __table_args__ = {"schema": "apple_health"}

    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())
    metric_id = Column(
        UUID, ForeignKey("apple_health.health_metric.id", ondelete="CASCADE")
    )
    start_date = Column(DateTime(timezone=True), nullable=False)
    end_date = Column(DateTime(timezone=True), nullable=False)
    value = Column(Text)
    qty = Column(Float)
    source = Column(Text)

    def __repr__(self):
        return f"<SleepAnalysis(id={self.id}, start_date={self.start_date}, end_date={self.end_date}, value={self.value}, qty={self.qty}, source={self.source})>"
