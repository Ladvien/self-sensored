from datetime import datetime, date
from typing import List, Optional, Dict, Any, Literal, Type, Union
from pydantic import BaseModel, Field, ConfigDict
from rich import print
import logging


class TZBaseModel(BaseModel):
    model_config = ConfigDict(str_to_datetime_mode="iso8601", populate_by_name=True)

    def get_date(self) -> datetime:
        return getattr(self, "date", getattr(self, "timestamp", None))


# ---- Reusable Value Types ----


class QuantityTimestamp(TZBaseModel):
    qty: float
    date: datetime
    source: Optional[str] = None

    class Config:
        arbitrary_types_allowed = True
        extra = "allow"
        populate_by_name = True


class WorkoutValue(TZBaseModel):
    qty: float
    units: str


class RoutePoint(TZBaseModel):
    lat: float
    lon: float
    altitude: float
    timestamp: datetime


class ElevationData(TZBaseModel):
    ascent: float
    descent: float
    units: str


# ---- Health Metrics (General) ----


class HealthMetric(TZBaseModel):
    name: str
    units: str
    data: List[Any]


# ---- Specialized Metrics ----


class BloodPressure(TZBaseModel):
    date: datetime
    systolic: float
    diastolic: float


class HeartRate(TZBaseModel):
    date: datetime
    min: Optional[float] = Field(None, alias="Min")
    avg: Optional[float] = Field(None, alias="Avg")
    max: Optional[float] = Field(None, alias="Max")
    context: Optional[str] = None
    source: Optional[str] = None


class SleepAnalysis(TZBaseModel):
    start_date: datetime = Field(..., alias="startDate")
    end_date: datetime = Field(..., alias="endDate")
    value: Optional[str] = None
    qty: Optional[float] = None
    source: Optional[str] = None


class BloodGlucose(TZBaseModel):
    date: datetime
    qty: float
    meal_time: Literal["Before Meal", "After Meal", "Unspecified"]


class SexualActivity(TZBaseModel):
    date: datetime
    unspecified: float = Field(..., alias="Unspecified")
    protection_used: float = Field(..., alias="Protection Used")
    protection_not_used: float = Field(..., alias="Protection Not Used")


class HygieneEvent(TZBaseModel):
    timestamp: datetime = Field(alias="date")
    qty: Optional[float] = None
    value: Optional[str] = None
    source: Optional[str] = None


class InsulinDelivery(TZBaseModel):
    date: datetime
    qty: float
    reason: Literal["Bolus", "Basal"]


# ---- Heart Rate Notifications ----


class HRSubMeasurement(TZBaseModel):
    hr: float
    units: str
    timestamp: Dict[str, Union[str, float]]


class HRVSubMeasurement(TZBaseModel):
    hrv: float
    units: str
    timestamp: Dict[str, Union[str, float]]


class HeartRateNotification(TZBaseModel):
    start: datetime
    end: datetime
    threshold: Optional[float] = None
    heart_rate: List[HRSubMeasurement]
    heart_rate_variation: List[HRVSubMeasurement]


# ---- Symptoms ----


class Symptom(TZBaseModel):
    start: datetime
    end: datetime
    name: str
    severity: str
    user_entered: bool
    source: str


# ---- State of Mind ----


class StateOfMind(TZBaseModel):
    id: str
    start: datetime
    end: datetime
    kind: str
    labels: List[str]
    associations: List[str]
    valence: float
    valence_classification: float
    metadata: Dict[str, str]


# ---- ECG ----


class VoltageMeasurement(TZBaseModel):
    date: datetime
    voltage: float
    units: str


class ECG(TZBaseModel):
    start: datetime
    end: datetime
    classification: str
    severity: str
    average_heart_rate: float
    number_of_voltage_measurements: int
    voltage_measurements: List[VoltageMeasurement]
    sampling_frequency: float
    source: str


# ---- Workouts ----


class WorkoutPoint(TZBaseModel):
    date: datetime
    qty: float
    units: str


class Workout(TZBaseModel):
    name: str
    start: datetime
    end: datetime
    heart_rate: Optional[List[WorkoutPoint]] = None
    heart_rate_recovery: Optional[List[WorkoutPoint]] = None
    route: Optional[List[RoutePoint]] = None
    total_energy: Optional[WorkoutValue] = None
    active_energy: Optional[WorkoutValue] = None
    max_heart_rate: Optional[WorkoutValue] = None
    avg_heart_rate: Optional[WorkoutValue] = None
    step_count: Optional[WorkoutValue] = None
    step_cadence: Optional[WorkoutValue] = None
    total_swimming_stroke_count: Optional[WorkoutValue] = None
    swim_cadence: Optional[WorkoutValue] = None
    distance: Optional[WorkoutValue] = None
    speed: Optional[WorkoutValue] = None
    flights_climbed: Optional[WorkoutValue] = None
    intesity: Optional[WorkoutValue] = None
    temperature: Optional[WorkoutValue] = None
    humidity: Optional[WorkoutValue] = None
    elevation: Optional[ElevationData] = None


# ---- Root Schema ----


class HealthPayload(TZBaseModel):
    metrics: List[HealthMetric]
    workouts: Optional[List[Workout]] = Field(default_factory=list)


class WrappedHealthPayload(TZBaseModel):
    data: HealthPayload


# ---- Dispatch Utilities ----

SPECIALIZED_METRICS = {
    "blood_pressure": BloodPressure,
    "heart_rate": HeartRate,
    "sleep_analysis": SleepAnalysis,
    "blood_glucose": BloodGlucose,
    "sexual_activity": SexualActivity,
    "handwashing": HygieneEvent,
    "toothbrushing": HygieneEvent,
    "insulin_delivery": InsulinDelivery,
    "heart_rate_notifications": HeartRateNotification,
    "symptoms": Symptom,
    "state_of_mind": StateOfMind,
    "ecg": ECG,
}


logger = logging.getLogger(__name__)


def parse_metric(metric: HealthMetric) -> List[TZBaseModel]:
    model_cls: Optional[Type[TZBaseModel]] = SPECIALIZED_METRICS.get(
        metric.name.lower()
    )
    parsed: List[TZBaseModel] = []
    for i, entry in enumerate(metric.data):
        try:
            # Dispatch to specialized or default model
            model = model_cls(**entry) if model_cls else QuantityTimestamp(**entry)
            parsed.append(model)
        except Exception as e:
            logger.warning(
                f"Skipping invalid entry #{i} in metric '{metric.name}': {e}\nEntry: {entry}"
            )

    return parsed


def parse_payload(data: dict) -> HealthPayload:
    payload = HealthPayload(**data)
    for metric in payload.metrics:
        metric.data = parse_metric(metric)
    return payload
