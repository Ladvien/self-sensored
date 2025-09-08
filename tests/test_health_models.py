import pytest
from datetime import datetime
from app.api.models import (
    HealthPayload, HealthMetric, BloodPressure, HeartRate, 
    SleepAnalysis, QuantityTimestamp, Workout
)


def test_health_metric_creation():
    """Test basic health metric model creation"""
    metric = HealthMetric(
        name="HeartRate",
        units="count/min",
        data=[]
    )
    assert metric.name == "HeartRate"
    assert metric.units == "count/min"


def test_quantity_timestamp_creation():
    """Test QuantityTimestamp model creation"""
    now = datetime.now()
    qt = QuantityTimestamp(
        qty=75.0,
        date=now,
        source="Apple Watch"
    )
    assert qt.qty == 75.0
    assert qt.date == now
    assert qt.source == "Apple Watch"
    assert qt.get_primary_date() == now


def test_blood_pressure_model():
    """Test BloodPressure specialized model"""
    now = datetime.now()
    bp = BloodPressure(
        date=now,
        systolic=120,
        diastolic=80
    )
    assert bp.systolic == 120
    assert bp.diastolic == 80
    assert bp.date == now


def test_heart_rate_model():
    """Test HeartRate specialized model"""
    now = datetime.now()
    hr = HeartRate(
        date=now,
        hr=75,
        hr_min=65,
        hr_max=85,
        hr_avg=75
    )
    assert hr.hr == 75
    assert hr.hr_min == 65
    assert hr.hr_max == 85
    assert hr.hr_avg == 75


def test_health_payload_creation():
    """Test complete HealthPayload creation"""
    payload = HealthPayload(
        data={'metrics': [], 'workouts': []}
    )
    assert 'metrics' in payload.data
    assert 'workouts' in payload.data