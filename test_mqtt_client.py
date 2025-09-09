#!/usr/bin/env python3
"""
MQTT Test Client for Health Export API
Tests MQTT connectivity without requiring the full Rust application to compile
"""

import json
import time
import uuid
from datetime import datetime
import paho.mqtt.client as mqtt

# Configuration
MQTT_BROKER = "localhost"
MQTT_PORT = 9001  # WebSocket port
MQTT_USER = "health_export"
MQTT_PASS = "HealthData2024!"
TEST_USER_ID = "b0d8f483-fadf-46bb-ad54-fa694238424a"
TOPIC = f"health/data/{TEST_USER_ID}"

def on_connect(client, userdata, flags, rc):
    """Callback for when the client connects to the broker"""
    if rc == 0:
        print(f"✓ Connected to MQTT broker at {MQTT_BROKER}:{MQTT_PORT}")
        # Subscribe to health data topics
        client.subscribe("health/data/+")
        print(f"✓ Subscribed to health/data/+ topics")
    else:
        print(f"✗ Failed to connect, return code {rc}")

def on_message(client, userdata, msg):
    """Callback for when a message is received"""
    print(f"📨 Received message on {msg.topic}:")
    try:
        data = json.loads(msg.payload.decode())
        print(json.dumps(data, indent=2))
    except:
        print(msg.payload.decode())

def on_publish(client, userdata, mid):
    """Callback for when a message is published"""
    print(f"✓ Message published (ID: {mid})")

def create_test_health_data():
    """Create sample health data similar to Auto Health Export format"""
    return {
        "data": {
            "metrics": [
                {
                    "type": "HeartRate",
                    "source": "MQTT Test Client",
                    "avg_bpm": 72,
                    "min_bpm": 65,
                    "max_bpm": 85,
                    "recorded_at": datetime.utcnow().isoformat() + "Z"
                },
                {
                    "type": "Steps",
                    "count": 5432,
                    "date": datetime.now().strftime("%Y-%m-%d"),
                    "source": "MQTT Test"
                },
                {
                    "type": "BloodPressure",
                    "systolic": 120,
                    "diastolic": 80,
                    "pulse": 70,
                    "recorded_at": datetime.utcnow().isoformat() + "Z"
                }
            ],
            "workouts": [
                {
                    "type": "Walking",
                    "start_time": datetime.utcnow().isoformat() + "Z",
                    "duration_seconds": 1800,
                    "distance_meters": 2500,
                    "calories": 150
                }
            ]
        },
        "metadata": {
            "export_date": datetime.utcnow().isoformat() + "Z",
            "version": "1.0",
            "source": "MQTT Test Client"
        }
    }

def test_mqtt_connection():
    """Test MQTT connection and publish test data"""
    print("=== MQTT Health Export Test Client ===\n")
    
    # Create MQTT client with WebSocket transport
    client = mqtt.Client(client_id=f"test_client_{uuid.uuid4().hex[:8]}", 
                        transport="websockets")
    
    # Set credentials
    client.username_pw_set(MQTT_USER, MQTT_PASS)
    
    # Set callbacks
    client.on_connect = on_connect
    client.on_message = on_message
    client.on_publish = on_publish
    
    print(f"Connecting to MQTT broker at ws://{MQTT_BROKER}:{MQTT_PORT}")
    
    try:
        # Connect to broker
        client.connect(MQTT_BROKER, MQTT_PORT, 60)
        
        # Start network loop in background
        client.loop_start()
        
        # Wait for connection
        time.sleep(2)
        
        # Publish test data
        print(f"\n📤 Publishing test health data to {TOPIC}")
        test_data = create_test_health_data()
        payload = json.dumps(test_data, indent=2)
        
        result = client.publish(TOPIC, payload, qos=1)
        if result.rc == mqtt.MQTT_ERR_SUCCESS:
            print("✓ Test data queued for publishing")
        else:
            print(f"✗ Failed to queue message: {result.rc}")
        
        # Wait for messages
        print("\n⏳ Listening for messages for 10 seconds...")
        time.sleep(10)
        
        # Disconnect
        client.loop_stop()
        client.disconnect()
        print("\n✓ Test completed")
        
    except Exception as e:
        print(f"\n✗ Error: {e}")
        print("\nTroubleshooting:")
        print("1. Ensure Mosquitto is installed: sudo apt install mosquitto mosquitto-clients")
        print("2. Ensure Mosquitto is running: sudo systemctl start mosquitto")
        print("3. Check Mosquitto config has WebSocket listener on port 9001")
        print("4. Check firewall allows port 9001")
        print("\nTo install and configure Mosquitto, run:")
        print("sudo bash /home/ladvien/self-sensored/setup_mqtt.sh")

if __name__ == "__main__":
    # Check if paho-mqtt is installed
    try:
        import paho.mqtt.client
    except ImportError:
        print("✗ paho-mqtt library not installed")
        print("Install with: pip install paho-mqtt")
        exit(1)
    
    test_mqtt_connection()