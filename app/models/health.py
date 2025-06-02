from datetime import datetime, date
from typing import List, Optional, Dict, Any, Literal, Union
from pydantic import BaseModel, Field, ConfigDict


class TZBaseModel(BaseModel):
    model_config = ConfigDict(str_to_datetime_mode="iso8601")


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


class BloodPressureData(TZBaseModel):
    date: datetime
    systolic: float
    diastolic: float


class HeartRateData(TZBaseModel):
    date: datetime
    min: float = Field(..., alias="Min")
    avg: float = Field(..., alias="Avg")
    max: float = Field(..., alias="Max")


class SleepAnalysisData(TZBaseModel):
    date: date
    asleep: float
    sleep_start: datetime
    sleep_end: datetime
    sleep_source: str
    in_bed: float
    in_bed_start: datetime
    in_bed_end: datetime
    in_bed_source: str


class BloodGlucoseData(TZBaseModel):
    date: datetime
    qty: float
    meal_time: Literal["Before Meal", "After Meal", "Unspecified"]


class SexualActivityData(TZBaseModel):
    date: datetime
    unspecified: float = Field(..., alias="Unspecified")
    protection_used: float = Field(..., alias="Protection Used")
    protection_not_used: float = Field(..., alias="Protection Not Used")


class HygieneEventData(TZBaseModel):
    date: datetime
    qty: float
    value: Literal["Complete", "Incomplete"]


class InsulinDeliveryData(TZBaseModel):
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


class SymptomData(TZBaseModel):
    start: datetime
    end: datetime
    name: str
    severity: str
    user_entered: bool
    source: str


# ---- State of Mind ----


class StateOfMindData(TZBaseModel):
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


class ECGData(TZBaseModel):
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


class WorkoutData(TZBaseModel):
    name: str
    start: datetime
    end: datetime
    heart_rate_data: Optional[List[WorkoutPoint]] = None
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
    workouts: Optional[List[WorkoutData]] = Field(default_factory=list)


class WrappedHealthPayload(TZBaseModel):
    data: HealthPayload


# ---- Dispatch Utilities ----

SPECIALIZED_METRICS = {
    "blood pressure": BloodPressureData,
    "heart rate": HeartRateData,
    "sleep analysis": SleepAnalysisData,
    "blood glucose": BloodGlucoseData,
    "sexual activity": SexualActivityData,
    "handwashing": HygieneEventData,
    "toothbrushing": HygieneEventData,
    "insulin delivery": InsulinDeliveryData,
    "heart rate notifications": HeartRateNotification,
    "symptoms": SymptomData,
    "state of mind": StateOfMindData,
    "ecg": ECGData,
}

import logging

logger = logging.getLogger(__name__)


def parse_metric(metric: HealthMetric) -> List[TZBaseModel]:
    model_cls = SPECIALIZED_METRICS.get(metric.name.lower())
    parsed = []

    for entry in metric.data:
        try:
            if model_cls:
                parsed.append(model_cls(**entry))
            else:
                parsed.append(QuantityTimestamp(**entry))
        except Exception as e:
            logger.warning(f"Skipping invalid entry in '{metric.name}': {e}")

    return parsed


def parse_payload(data: dict) -> HealthPayload:
    payload = HealthPayload(**data)
    for metric in payload.metrics:
        metric.data = parse_metric(metric)
    return payload
