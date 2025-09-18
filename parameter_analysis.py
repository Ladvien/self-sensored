#!/usr/bin/env python3
"""
PostgreSQL Parameter Limit Analysis for Health Metrics Batch Processing
Identifies configurations that exceed PostgreSQL's 65,535 parameter limit.
"""

# Parameter counts per metric type (from batch_config.rs)
PARAMS_PER_RECORD = {
    'heart_rate': 10,
    'blood_pressure': 6,
    'sleep': 10,
    'activity': 19,  # Critical: highest parameter count
    'body_measurement': 16,
    'temperature': 8,
    'respiratory': 7,
    'workout': 10,
    'blood_glucose': 8,
    'metabolic': 6,
    'nutrition': 32,  # Highest parameter count
    'menstrual': 8,
    'fertility': 12,
    'environmental': 14,
    'audio_exposure': 7,
    'safety_event': 8,
    'mindfulness': 9,
    'mental_health': 10,
    'symptom': 9,
    'hygiene': 8,
}

# Current chunk sizes from ingest_async_simple.rs (line 185-209)
CURRENT_CHUNK_SIZES = {
    'heart_rate': 4200,
    'blood_pressure': 8000,
    'sleep': 5200,
    'activity': 2700,  # Fixed from 7000 to 2700
    'body_measurement': 3500,
    'temperature': 6500,  # Fixed from 8000 to 6500
    'respiratory': 7000,
    'workout': 5000,
    'blood_glucose': 6500,
    'metabolic': 8700,
    'nutrition': 1600,
    'menstrual': 6500,
    'fertility': 4300,
    'environmental': 3700,
    'audio_exposure': 7000,
    'safety_event': 6500,
    'mindfulness': 5800,
    'mental_health': 5200,
    'symptom': 5800,
    'hygiene': 6500,
}

# PostgreSQL limits
POSTGRESQL_MAX_PARAMS = 65535
SAFE_PARAM_LIMIT = 52428  # 80% of max for safety margin

def analyze_parameter_usage():
    print("PostgreSQL Parameter Limit Analysis")
    print("=" * 50)
    print(f"PostgreSQL Max Parameters: {POSTGRESQL_MAX_PARAMS:,}")
    print(f"Safe Parameter Limit (80%): {SAFE_PARAM_LIMIT:,}")
    print()
    
    critical_issues = []
    warnings = []
    safe_configs = []
    
    for metric_type in sorted(PARAMS_PER_RECORD.keys()):
        params_per_record = PARAMS_PER_RECORD[metric_type]
        chunk_size = CURRENT_CHUNK_SIZES.get(metric_type, 0)
        total_params = chunk_size * params_per_record
        usage_percent = (total_params / SAFE_PARAM_LIMIT) * 100
        
        # Calculate maximum safe chunk size
        max_safe_chunk = SAFE_PARAM_LIMIT // params_per_record
        
        status = ""
        if total_params > SAFE_PARAM_LIMIT:
            status = "🔴 CRITICAL"
            critical_issues.append((metric_type, chunk_size, total_params, max_safe_chunk))
        elif total_params > (SAFE_PARAM_LIMIT * 90 // 100):
            status = "🟠 WARNING"
            warnings.append((metric_type, chunk_size, total_params, max_safe_chunk))
        else:
            status = "✅ SAFE"
            safe_configs.append((metric_type, chunk_size, total_params, max_safe_chunk))
        
        print(f"{status:<12} {metric_type:<15} | Chunk: {chunk_size:>5} × {params_per_record:>2} params = {total_params:>6} ({usage_percent:>5.1f}%) | Max Safe: {max_safe_chunk:>5}")
    
    print("\n" + "=" * 80)
    
    if critical_issues:
        print("\n🔴 CRITICAL ISSUES (CAUSES DATA LOSS):")
        for metric_type, chunk_size, total_params, max_safe_chunk in critical_issues:
            excess = total_params - SAFE_PARAM_LIMIT
            print(f"  • {metric_type}: {chunk_size} → {max_safe_chunk} (excess: {excess:,} params)")
    
    if warnings:
        print("\n🟠 HIGH USAGE WARNINGS (>90% of safe limit):")
        for metric_type, chunk_size, total_params, max_safe_chunk in warnings:
            print(f"  • {metric_type}: {chunk_size} chunks ({total_params:,} params)")
    
    print(f"\n✅ SAFE CONFIGURATIONS: {len(safe_configs)}")
    print(f"🟠 HIGH USAGE WARNINGS: {len(warnings)}")
    print(f"🔴 CRITICAL ISSUES: {len(critical_issues)}")
    
    if critical_issues:
        print("\n⚠️  CRITICAL: These configurations will cause silent data loss!")
        print("   PostgreSQL will reject queries exceeding parameter limits.")
        
    return critical_issues, warnings

if __name__ == "__main__":
    analyze_parameter_usage()
