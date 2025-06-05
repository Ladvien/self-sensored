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
    Boolean,
    Integer,
    UniqueConstraint,
    Index,
)
from sqlalchemy.orm import declarative_base, relationship
from sqlalchemy.sql import func

Base = declarative_base()


class AppleHealthMixin:
    """Common fields for all Apple Health tables"""

    __table_args__ = {"schema": "apple_health"}
    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())


class HealthPayload(Base, AppleHealthMixin):
    __tablename__ = "health_payload"
    __table_args__ = (
        UniqueConstraint("payload_hash", name="uq_health_payload_hash"),
        {"schema": "apple_health"},
    )

    received_at = Column(DateTime(timezone=True), server_default=func.now())
    payload_hash = Column(Text, nullable=False)

    # Relationships
    metrics = relationship(
        "HealthMetric", back_populates="payload", cascade="all, delete-orphan"
    )
    workouts = relationship(
        "Workout", back_populates="payload", cascade="all, delete-orphan"
    )

    def __repr__(self):
        return f"<HealthPayload(id={self.id}, received_at={self.received_at})>"


class HealthMetric(Base, AppleHealthMixin):
    __tablename__ = "health_metric"
    __table_args__ = (
        UniqueConstraint("payload_id", "name", name="uq_health_metric_payload_name"),
        UniqueConstraint(
            "payload_id", "name", "data_hash", name="uq_health_metric_data"
        ),
        Index("idx_health_metric_payload_id", "payload_id"),
        Index("idx_health_metric_name", "name"),
        {"schema": "apple_health"},
    )

    payload_id = Column(
        UUID,
        ForeignKey("apple_health.health_payload.id", ondelete="CASCADE"),
        nullable=False,
    )
    name = Column(Text, nullable=False)
    units = Column(Text, nullable=False)
    data_hash = Column(Text)  # Hash of metric data for deduplication

    # Relationships
    payload = relationship("HealthPayload", back_populates="metrics")
    quantity_data = relationship(
        "QuantityTimestamp", back_populates="metric", cascade="all, delete-orphan"
    )
    blood_pressure = relationship(
        "BloodPressure", back_populates="metric", cascade="all, delete-orphan"
    )
    heart_rate = relationship(
        "HeartRate", back_populates="metric", cascade="all, delete-orphan"
    )
    sleep_analysis = relationship(
        "SleepAnalysis", back_populates="metric", cascade="all, delete-orphan"
    )
    blood_glucose = relationship(
        "BloodGlucose", back_populates="metric", cascade="all, delete-orphan"
    )
    sexual_activity = relationship(
        "SexualActivity", back_populates="metric", cascade="all, delete-orphan"
    )
    hygiene_events = relationship(
        "HygieneEvent", back_populates="metric", cascade="all, delete-orphan"
    )
    insulin_delivery = relationship(
        "InsulinDelivery", back_populates="metric", cascade="all, delete-orphan"
    )
    symptoms = relationship(
        "Symptom", back_populates="metric", cascade="all, delete-orphan"
    )
    state_of_mind = relationship(
        "StateOfMind", back_populates="metric", cascade="all, delete-orphan"
    )
    ecg = relationship("ECG", back_populates="metric", cascade="all, delete-orphan")
    heart_rate_notifications = relationship(
        "HeartRateNotification", back_populates="metric", cascade="all, delete-orphan"
    )

    def __repr__(self):
        return f"<HealthMetric(id={self.id}, name={self.name}, units={self.units})>"


class QuantityTimestamp(Base, AppleHealthMixin):
    __tablename__ = "quantity_timestamp"
    __table_args__ = (
        UniqueConstraint(
            "metric_id",
            "date",
            "source",
            name="uq_quantity_timestamp_metric_date_source",
        ),
        Index("idx_quantity_timestamp_metric_id", "metric_id"),
        Index("idx_quantity_timestamp_date", "date"),
        Index("idx_quantity_timestamp_metric_date", "metric_id", "date"),
        {"schema": "apple_health"},
    )

    date = Column(DateTime(timezone=True), nullable=False)
    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    qty = Column(Float, nullable=False)
    source = Column(Text)

    # Relationship
    metric = relationship("HealthMetric", back_populates="quantity_data")

    def __repr__(self):
        return f"<QuantityTimestamp(id={self.id}, date={self.date}, qty={self.qty})>"


class BloodPressure(Base, AppleHealthMixin):
    __tablename__ = "blood_pressure"
    __table_args__ = (
        UniqueConstraint("metric_id", "date", name="uq_blood_pressure_metric_date"),
        Index("idx_blood_pressure_metric_id", "metric_id"),
        Index("idx_blood_pressure_date", "date"),
        {"schema": "apple_health"},
    )

    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    date = Column(DateTime(timezone=True), nullable=False)
    systolic = Column(Float, nullable=False)
    diastolic = Column(Float, nullable=False)

    # Relationship
    metric = relationship("HealthMetric", back_populates="blood_pressure")

    def __repr__(self):
        return f"<BloodPressure(date={self.date}, {self.systolic}/{self.diastolic})>"


class HeartRate(Base, AppleHealthMixin):
    __tablename__ = "heart_rate"
    __table_args__ = (
        UniqueConstraint(
            "metric_id", "date", "context", name="uq_heart_rate_metric_date_context"
        ),
        Index("idx_heart_rate_metric_id", "metric_id"),
        Index("idx_heart_rate_date", "date"),
        {"schema": "apple_health"},
    )

    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    date = Column(DateTime(timezone=True), nullable=False)
    min = Column(Float)
    avg = Column(Float)
    max = Column(Float)
    context = Column(Text)
    source = Column(Text)

    # Relationship
    metric = relationship("HealthMetric", back_populates="heart_rate")

    def __repr__(self):
        return f"<HeartRate(date={self.date}, avg={self.avg})>"


class SleepAnalysis(Base, AppleHealthMixin):
    __tablename__ = "sleep_analysis"
    __table_args__ = (
        UniqueConstraint(
            "metric_id", "start_date", "end_date", name="uq_sleep_analysis_metric_dates"
        ),
        Index("idx_sleep_analysis_metric_id", "metric_id"),
        Index("idx_sleep_analysis_start_date", "start_date"),
        {"schema": "apple_health"},
    )

    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    start_date = Column(DateTime(timezone=True), nullable=False)
    end_date = Column(DateTime(timezone=True), nullable=False)
    value = Column(Text)
    qty = Column(Float)
    source = Column(Text)

    # Relationship
    metric = relationship("HealthMetric", back_populates="sleep_analysis")

    def __repr__(self):
        return f"<SleepAnalysis(start={self.start_date}, end={self.end_date})>"


class BloodGlucose(Base, AppleHealthMixin):
    __tablename__ = "blood_glucose"
    __table_args__ = (
        UniqueConstraint(
            "metric_id", "date", "meal_time", name="uq_blood_glucose_metric_date_meal"
        ),
        {"schema": "apple_health"},
    )

    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    date = Column(DateTime(timezone=True), nullable=False)
    qty = Column(Float, nullable=False)
    meal_time = Column(
        Text, nullable=False
    )  # 'Before Meal', 'After Meal', 'Unspecified'

    # Relationship
    metric = relationship("HealthMetric", back_populates="blood_glucose")


class SexualActivity(Base, AppleHealthMixin):
    __tablename__ = "sexual_activity"
    __table_args__ = (
        UniqueConstraint("metric_id", "date", name="uq_sexual_activity_metric_date"),
        {"schema": "apple_health"},
    )

    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    date = Column(DateTime(timezone=True), nullable=False)
    unspecified = Column(Float)
    protection_used = Column(Float)
    protection_not_used = Column(Float)

    # Relationship
    metric = relationship("HealthMetric", back_populates="sexual_activity")


class HygieneEvent(Base, AppleHealthMixin):
    __tablename__ = "hygiene_event"
    __table_args__ = (
        UniqueConstraint("metric_id", "date", name="uq_hygiene_event_metric_date"),
        {"schema": "apple_health"},
    )

    id = Column(UUID, primary_key=True, server_default=func.gen_random_uuid())
    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    date = Column(DateTime(timezone=True), nullable=False)  # only date column
    qty = Column(Float)
    source = Column(Text)
    value = Column(Text)

    metric = relationship("HealthMetric", back_populates="hygiene_events")


class InsulinDelivery(Base, AppleHealthMixin):
    __tablename__ = "insulin_delivery"
    __table_args__ = (
        UniqueConstraint(
            "metric_id", "date", "reason", name="uq_insulin_delivery_metric_date_reason"
        ),
        {"schema": "apple_health"},
    )

    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    date = Column(DateTime(timezone=True), nullable=False)
    qty = Column(Float, nullable=False)
    reason = Column(Text, nullable=False)  # 'Bolus', 'Basal'

    # Relationship
    metric = relationship("HealthMetric", back_populates="insulin_delivery")


class Symptom(Base, AppleHealthMixin):
    __tablename__ = "symptom"
    __table_args__ = (
        UniqueConstraint(
            "metric_id", "start", "name", name="uq_symptom_metric_start_name"
        ),
        {"schema": "apple_health"},
    )

    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    start = Column(DateTime(timezone=True), nullable=False)
    end = Column(DateTime(timezone=True), nullable=False)
    name = Column(Text, nullable=False)
    severity = Column(Text, nullable=False)
    user_entered = Column(Boolean, nullable=False)
    source = Column(Text)

    # Relationship
    metric = relationship("HealthMetric", back_populates="symptoms")


class StateOfMind(Base, AppleHealthMixin):
    __tablename__ = "state_of_mind"
    __table_args__ = {"schema": "apple_health"}

    metric_id = Column(
        UUID, ForeignKey("apple_health.health_metric.id", ondelete="CASCADE")
    )
    start = Column(DateTime(timezone=True))
    end = Column(DateTime(timezone=True))
    kind = Column(Text)
    valence = Column(Float)
    valence_classification = Column(Float)
    metadata_json = Column("metadata", JSON)  # Maps to 'metadata' column in DB
    labels = Column(ARRAY(Text))
    associations = Column(ARRAY(Text))

    # Relationship
    metric = relationship("HealthMetric", back_populates="state_of_mind")


class ECG(Base, AppleHealthMixin):
    __tablename__ = "ecg"
    __table_args__ = (
        UniqueConstraint("metric_id", "start", name="uq_ecg_metric_start"),
        {"schema": "apple_health"},
    )

    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    start = Column(DateTime(timezone=True), nullable=False)
    end = Column(DateTime(timezone=True), nullable=False)
    classification = Column(Text)
    severity = Column(Text)
    average_heart_rate = Column(Float)
    number_of_voltage_measurements = Column(Integer)
    sampling_frequency = Column(Float)
    source = Column(Text)
    voltage_measurements = Column(JSON)

    # Relationship
    metric = relationship("HealthMetric", back_populates="ecg")


class HeartRateNotification(Base, AppleHealthMixin):
    __tablename__ = "heart_rate_notification"
    __table_args__ = (
        UniqueConstraint(
            "metric_id", "start", "end", name="uq_heart_rate_notification_metric_times"
        ),
        {"schema": "apple_health"},
    )

    metric_id = Column(
        UUID,
        ForeignKey("apple_health.health_metric.id", ondelete="CASCADE"),
        nullable=False,
    )
    start = Column(DateTime(timezone=True), nullable=False)
    end = Column(DateTime(timezone=True), nullable=False)
    threshold = Column(Float)
    heart_rate = Column(JSON)
    heart_rate_variation = Column(JSON)

    # Relationship
    metric = relationship("HealthMetric", back_populates="heart_rate_notifications")


class Workout(Base, AppleHealthMixin):
    __tablename__ = "workout"
    __table_args__ = (
        UniqueConstraint(
            "payload_id", "name", "start", name="uq_workout_payload_name_start"
        ),
        {"schema": "apple_health"},
    )

    payload_id = Column(
        UUID,
        ForeignKey("apple_health.health_payload.id", ondelete="CASCADE"),
        nullable=False,
    )
    name = Column(Text, nullable=False)
    start = Column(DateTime(timezone=True), nullable=False)
    end = Column(DateTime(timezone=True), nullable=False)
    elevation = Column(JSON)

    # Relationships
    payload = relationship("HealthPayload", back_populates="workouts")
    workout_values = relationship(
        "WorkoutValue", back_populates="workout", cascade="all, delete-orphan"
    )
    workout_points = relationship(
        "WorkoutPoint", back_populates="workout", cascade="all, delete-orphan"
    )
    route_points = relationship(
        "WorkoutRoutePoint", back_populates="workout", cascade="all, delete-orphan"
    )


class WorkoutValue(Base, AppleHealthMixin):
    __tablename__ = "workout_value"
    __table_args__ = (
        UniqueConstraint("workout_id", "name", name="uq_workout_value_workout_name"),
        {"schema": "apple_health"},
    )

    workout_id = Column(
        UUID, ForeignKey("apple_health.workout.id", ondelete="CASCADE"), nullable=False
    )
    name = Column(Text, nullable=False)
    qty = Column(Float)
    units = Column(Text)

    # Relationship
    workout = relationship("Workout", back_populates="workout_values")


class WorkoutPoint(Base, AppleHealthMixin):
    __tablename__ = "workout_point"
    __table_args__ = (
        UniqueConstraint(
            "workout_id", "stream", "date", name="uq_workout_point_workout_stream_date"
        ),
        {"schema": "apple_health"},
    )

    workout_id = Column(
        UUID, ForeignKey("apple_health.workout.id", ondelete="CASCADE"), nullable=False
    )
    stream = Column(Text, nullable=False)  # 'heart_rate', 'heart_rate_recovery'
    date = Column(DateTime(timezone=True), nullable=False)
    qty = Column(Float)
    units = Column(Text)

    # Relationship
    workout = relationship("Workout", back_populates="workout_points")


class WorkoutRoutePoint(Base, AppleHealthMixin):
    __tablename__ = "workout_route_point"
    __table_args__ = (
        UniqueConstraint(
            "workout_id", "timestamp", name="uq_workout_route_point_workout_timestamp"
        ),
        {"schema": "apple_health"},
    )

    workout_id = Column(
        UUID, ForeignKey("apple_health.workout.id", ondelete="CASCADE"), nullable=False
    )
    lat = Column(Float, nullable=False)
    lon = Column(Float, nullable=False)
    altitude = Column(Float)
    timestamp = Column(DateTime(timezone=True), nullable=False)

    # Relationship
    workout = relationship("Workout", back_populates="route_points")

    def __repr__(self):
        return f"<WorkoutRoutePoint(lat={self.lat}, lon={self.lon}, timestamp={self.timestamp})>"
