#!/usr/bin/env python3
"""
Test script to verify that all critical imports work correctly
"""

def test_imports():
    """Test that all the main modules import without errors"""
    
    print("Testing API models import...")
    try:
        from app.api import models as api_models
        print("✅ API models imported successfully")
        
        # Test that key classes exist
        assert hasattr(api_models, 'HealthPayload')
        assert hasattr(api_models, 'HealthMetric')
        assert hasattr(api_models, 'TZBaseModel')
        assert hasattr(api_models, 'SPECIALIZED_METRICS')
        print("✅ All expected API model classes found")
        
    except Exception as e:
        print(f"❌ API models import failed: {e}")
        return False
    
    print("\nTesting DB models import...")
    try:
        from app.db import models as db_models
        print("✅ DB models imported successfully")
        
        # Test that key classes exist
        assert hasattr(db_models, 'HealthPayload')
        assert hasattr(db_models, 'HealthMetric') 
        assert hasattr(db_models, 'QuantityTimestamp')
        assert hasattr(db_models, 'MODEL_REGISTRY')
        print("✅ All expected DB model classes found")
        
    except Exception as e:
        print(f"❌ DB models import failed: {e}")
        return False
    
    print("\nTesting insert logic import...")
    try:
        from app.db import insert_logic
        print("✅ Insert logic imported successfully")
        
        # Test that key functions exist
        assert hasattr(insert_logic, 'insert_health_data')
        assert hasattr(insert_logic, 'SPECIALIZED_DB_MODELS')
        print("✅ All expected insert logic functions found")
        
    except Exception as e:
        print(f"❌ Insert logic import failed: {e}")
        return False
    
    print("\nTesting endpoints import...")
    try:
        from app.api.v1 import endpoints
        print("✅ Endpoints imported successfully")
        
    except Exception as e:
        print(f"❌ Endpoints import failed: {e}")
        return False
    
    print("\nTesting main app import...")
    try:
        from app import main
        print("✅ Main app imported successfully")
        
    except Exception as e:
        print(f"❌ Main app import failed: {e}")
        return False
    
    print("\n🎉 All imports successful!")
    return True


def test_model_instantiation():
    """Test that we can create model instances"""
    print("\nTesting model instantiation...")
    
    try:
        from app.api.models import QuantityTimestamp, HealthMetric
        from datetime import datetime
        
        # Test creating a quantity timestamp
        qt = QuantityTimestamp(
            qty=100.0,
            date=datetime.now(),
            source="test"
        )
        print("✅ QuantityTimestamp created successfully")
        
        # Test creating a health metric
        hm = HealthMetric(
            name="test_metric",
            units="count",
            data=[]
        )
        print("✅ HealthMetric created successfully")
        
        # Test get_primary_date method
        date = qt.get_primary_date()
        assert date is not None
        print("✅ get_primary_date() works")
        
    except Exception as e:
        print(f"❌ Model instantiation failed: {e}")
        return False
    
    print("✅ Model instantiation tests passed!")
    return True


if __name__ == "__main__":
    print("Running import and instantiation tests...\n")
    
    import_success = test_imports()
    model_success = test_model_instantiation()
    
    if import_success and model_success:
        print("\n🎉 All tests passed! Your fixes are working correctly.")
    else:
        print("\n❌ Some tests failed. Check the error messages above.")