#!/usr/bin/env python3
"""
Payload Analysis Script for Health Export API
This script analyzes raw_ingestions payloads and cross-checks against database entries.
"""

import psycopg2
import json
from collections import defaultdict, Counter
import sys
from datetime import datetime

# Database connection
DB_URL = "postgresql://self_sensored:37om3i*t3XfSZ0@192.168.1.104:5432/self_sensored"

def get_db_connection():
    """Get database connection"""
    return psycopg2.connect(DB_URL)

def analyze_payloads():
    """Analyze all payloads and cross-check against database"""

    conn = get_db_connection()
    cur = conn.cursor()

    print("=== PAYLOAD ANALYSIS REPORT ===")
    print(f"Generated: {datetime.now()}")
    print()

    # Get all raw ingestions
    cur.execute("""
        SELECT id, user_id, payload_hash, raw_payload, processing_status, created_at
        FROM raw_ingestions
        ORDER BY created_at
    """)

    ingestions = cur.fetchall()

    # Analyze payload structure
    payload_analysis = {}
    metric_counts = defaultdict(int)
    missing_data = []
    processing_summary = Counter()

    print(f"Total Raw Ingestions: {len(ingestions)}")

    unique_payloads = {}
    duplicate_payloads = defaultdict(list)

    for row in ingestions:
        ingestion_id, user_id, payload_hash, raw_payload, processing_status, created_at = row
        processing_summary[processing_status] += 1

        # Track duplicates
        if payload_hash in unique_payloads:
            duplicate_payloads[payload_hash].append({
                'id': ingestion_id,
                'created_at': created_at
            })
        else:
            unique_payloads[payload_hash] = {
                'id': ingestion_id,
                'user_id': user_id,
                'payload': raw_payload,
                'created_at': created_at
            }

    print(f"Unique Payloads: {len(unique_payloads)}")
    print(f"Duplicate Payloads: {len(duplicate_payloads)}")
    print(f"Processing Status Summary: {dict(processing_summary)}")
    print()

    # Analyze each unique payload
    total_expected_metrics = 0

    for payload_hash, payload_info in unique_payloads.items():
        try:
            payload_data = json.loads(payload_info['payload'])
            user_id = payload_info['user_id']

            print(f"--- Payload Hash: {payload_hash[:16]}... ---")
            print(f"User ID: {user_id}")
            print(f"Created: {payload_info['created_at']}")

            if 'data' in payload_data and 'metrics' in payload_data['data']:
                metrics = payload_data['data']['metrics']
                print(f"Metrics in payload: {len(metrics)}")
                total_expected_metrics += len(metrics)

                # Count by type
                type_counts = Counter()
                for metric in metrics:
                    metric_type = metric.get('type', 'Unknown')
                    type_counts[metric_type] += 1

                print(f"Metric types: {dict(type_counts)}")

                # Cross-check against database
                for metric_type, count in type_counts.items():
                    table_name = get_table_name_for_type(metric_type)
                    if table_name:
                        # Check if metrics exist in database
                        cur.execute(f"""
                            SELECT COUNT(*) FROM {table_name}
                            WHERE user_id = %s
                            AND created_at >= %s::timestamp - interval '1 hour'
                            AND created_at <= %s::timestamp + interval '1 hour'
                        """, (user_id, payload_info['created_at'], payload_info['created_at']))

                        db_count = cur.fetchone()[0]
                        print(f"  {metric_type} -> {table_name}: Expected {count}, Found {db_count}")

                        if db_count < count:
                            missing_data.append({
                                'payload_hash': payload_hash,
                                'metric_type': metric_type,
                                'expected': count,
                                'found': db_count,
                                'missing': count - db_count
                            })
                    else:
                        print(f"  {metric_type}: NO TABLE MAPPING FOUND")
                        missing_data.append({
                            'payload_hash': payload_hash,
                            'metric_type': metric_type,
                            'expected': count,
                            'found': 0,
                            'missing': count,
                            'reason': 'No table mapping'
                        })

            print()

        except json.JSONDecodeError as e:
            print(f"ERROR: Invalid JSON in payload {payload_hash}: {e}")
            missing_data.append({
                'payload_hash': payload_hash,
                'error': f'Invalid JSON: {e}'
            })
        except Exception as e:
            print(f"ERROR analyzing payload {payload_hash}: {e}")
            missing_data.append({
                'payload_hash': payload_hash,
                'error': str(e)
            })

    # Summary
    print("=== SUMMARY ===")
    print(f"Total Expected Metrics: {total_expected_metrics}")
    print(f"Missing/Problematic Data Points: {len(missing_data)}")

    if missing_data:
        print("\n=== MISSING DATA DETAILS ===")
        for item in missing_data:
            print(f"Payload: {item.get('payload_hash', 'Unknown')[:16]}...")
            if 'error' in item:
                print(f"  Error: {item['error']}")
            else:
                print(f"  Type: {item.get('metric_type')}")
                print(f"  Expected: {item.get('expected')}, Found: {item.get('found')}, Missing: {item.get('missing')}")
                if 'reason' in item:
                    print(f"  Reason: {item['reason']}")
            print()

    # Check for duplicate payload hashes
    if duplicate_payloads:
        print("=== DUPLICATE PAYLOADS ===")
        for payload_hash, duplicates in duplicate_payloads.items():
            print(f"Hash: {payload_hash[:16]}...")
            print(f"  Duplicated {len(duplicates)} times:")
            for dup in duplicates:
                print(f"    ID: {dup['id']}, Created: {dup['created_at']}")
        print()

    cur.close()
    conn.close()

    return {
        'total_ingestions': len(ingestions),
        'unique_payloads': len(unique_payloads),
        'duplicate_count': len(duplicate_payloads),
        'total_expected_metrics': total_expected_metrics,
        'missing_data': missing_data,
        'processing_summary': dict(processing_summary)
    }

def get_table_name_for_type(metric_type):
    """Map metric type to database table name"""
    mapping = {
        'HeartRate': 'heart_rate_metrics',
        'BloodPressure': 'blood_pressure_metrics',
        'Sleep': 'sleep_metrics',
        'Activity': 'activity_metrics',
        'Workout': 'workout_metrics',
        'BodyMeasurement': 'body_measurements',
        'EnvironmentalAudio': 'environmental_metrics'
    }
    return mapping.get(metric_type)

if __name__ == "__main__":
    try:
        results = analyze_payloads()
        print("Analysis completed successfully!")
    except Exception as e:
        print(f"Analysis failed: {e}")
        sys.exit(1)