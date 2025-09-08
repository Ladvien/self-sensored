# Database Infrastructure Setup Guide

This document provides comprehensive setup instructions for the Health Export REST API database infrastructure, supporting both development and production environments.

## Overview

The Health Export REST API uses a dual-database architecture:

- **PostgreSQL 15** with PostGIS extension for health data persistence and geospatial workout tracking
- **Redis 7.x** for caching API keys, rate limiting, and performance optimization

## Development Environment

### Quick Start with Docker Compose

```bash
# Clone the repository
git clone <repository-url>
cd health-export-api

# Start the database services
docker-compose up -d postgres redis

# Verify services are healthy
docker-compose ps

# Test PostgreSQL connectivity
docker exec -e PGPASSWORD=dev_password_123 health_export_postgres \
  psql -U health_user -d health_export_dev -c "SELECT version();"

# Test Redis connectivity
docker exec health_export_redis redis-cli ping
```

### Development Configuration Details

**PostgreSQL Development Settings:**
- **Host**: localhost
- **Port**: 5432
- **Database**: health_export_dev
- **Username**: health_user
- **Password**: dev_password_123
- **Connection String**: `postgresql://health_user:dev_password_123@localhost:5432/health_export_dev`

**Redis Development Settings:**
- **Host**: localhost
- **Port**: 6379
- **Password**: None (development only)
- **Connection String**: `redis://localhost:6379/0`

### Development Services

The docker-compose.yml includes additional development tools:

**pgAdmin (Web UI for PostgreSQL):**
- URL: http://localhost:8080
- Email: admin@healthexport.local
- Password: admin_password_123

**Redis Commander (Web UI for Redis):**
- URL: http://localhost:8081
- Available with: `docker-compose --profile debug up`

**Monitoring Stack:**
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (admin/grafana_admin_123)
- Available with: `docker-compose --profile monitoring up`

### Database Schema Initialization

The development environment automatically initializes with:

1. **PostgreSQL Extensions**:
   - PostGIS (geospatial data)
   - uuid-ossp (UUID generation)
   - pgcrypto (cryptographic functions)
   - pg_stat_statements (performance monitoring)

2. **Schema Creation**:
   - `apple_health` schema for organized data storage
   - Proper user permissions for health_user

3. **Performance Configuration**:
   - Optimized connection settings
   - Query performance monitoring
   - Development-friendly logging

## Production Environment

### Kubernetes Deployment

For production deployment on Kubernetes, see the comprehensive guide in `/k8s/README.md`.

**Quick Production Deployment:**

```bash
# Create namespace and secrets (update passwords first!)
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/database/postgresql-secret.yaml
kubectl apply -f k8s/redis/redis-secret.yaml

# Deploy PostgreSQL
kubectl apply -f k8s/database/
kubectl wait --for=condition=ready pod -l app=postgresql -n health-export --timeout=300s

# Deploy Redis
kubectl apply -f k8s/redis/
kubectl wait --for=condition=ready pod -l app=redis -n health-export --timeout=300s

# Verify deployment
kubectl get all -n health-export
```

### Production Configuration Details

**PostgreSQL Production Settings:**
- **Resources**: 512Mi-2Gi memory, 250m-1000m CPU
- **Storage**: 20Gi SSD persistent volume
- **Connections**: Up to 200 concurrent connections
- **Security**: SCRAM-SHA-256 authentication
- **Monitoring**: pg_stat_statements enabled

**Redis Production Settings:**
- **Resources**: 128Mi-512Mi memory, 100m-500m CPU
- **Storage**: 10Gi persistent volume
- **Security**: Password authentication, dangerous commands disabled
- **Persistence**: Both RDB and AOF enabled for durability

## Connection Configuration

### Environment Variables

The application should use these environment variables for database connections:

**Required Environment Variables:**

```bash
# PostgreSQL Configuration
DATABASE_URL="postgresql://username:password@host:port/database"
DATABASE_HOST="localhost"  # or k8s service name
DATABASE_PORT="5432"
DATABASE_NAME="health_export_dev"  # or health_export_prod
DATABASE_USER="health_user"
DATABASE_PASSWORD="secure_password"
DATABASE_SSL_MODE="prefer"  # or "require" for production

# Redis Configuration  
REDIS_URL="redis://[:password@]host:port/db"
REDIS_HOST="localhost"  # or k8s service name
REDIS_PORT="6379"
REDIS_PASSWORD=""  # empty for development
REDIS_DB="0"
REDIS_SSL="false"  # set to "true" for production with TLS

# Connection Pool Configuration
DATABASE_MIN_CONNECTIONS="5"
DATABASE_MAX_CONNECTIONS="20"
DATABASE_CONNECTION_TIMEOUT="30"
DATABASE_QUERY_TIMEOUT="60"

# Cache Configuration
REDIS_API_KEY_TTL="300"    # 5 minutes
REDIS_RATE_LIMIT_TTL="3600"  # 1 hour
REDIS_MAX_CONNECTIONS="10"
```

### Python/FastAPI Connection Examples

**PostgreSQL with asyncpg:**

```python
import asyncpg
from sqlalchemy.ext.asyncio import create_async_engine

# Using environment variables
DATABASE_URL = os.getenv("DATABASE_URL")
engine = create_async_engine(DATABASE_URL, pool_size=20, max_overflow=30)

# Direct asyncpg connection
async def connect_postgres():
    return await asyncpg.connect(
        host=os.getenv("DATABASE_HOST"),
        port=int(os.getenv("DATABASE_PORT")),
        database=os.getenv("DATABASE_NAME"),
        user=os.getenv("DATABASE_USER"),
        password=os.getenv("DATABASE_PASSWORD")
    )
```

**Redis with aioredis:**

```python
import aioredis
import os

# Using connection URL
async def connect_redis():
    redis_url = os.getenv("REDIS_URL")
    return aioredis.from_url(redis_url, decode_responses=True)

# Using individual parameters
async def connect_redis_params():
    return aioredis.Redis(
        host=os.getenv("REDIS_HOST"),
        port=int(os.getenv("REDIS_PORT")),
        password=os.getenv("REDIS_PASSWORD"),
        db=int(os.getenv("REDIS_DB")),
        decode_responses=True
    )
```

## Health Checks and Monitoring

### Application Health Checks

Implement these health check endpoints in your application:

```python
from fastapi import FastAPI, HTTPException
import asyncpg
import aioredis

app = FastAPI()

@app.get("/health")
async def health_check():
    """Comprehensive health check for both databases"""
    try:
        # Test PostgreSQL
        conn = await asyncpg.connect(os.getenv("DATABASE_URL"))
        await conn.fetchval("SELECT 1")
        await conn.close()
        postgres_status = "healthy"
    except Exception as e:
        postgres_status = f"unhealthy: {str(e)}"
    
    try:
        # Test Redis
        redis = aioredis.from_url(os.getenv("REDIS_URL"))
        await redis.ping()
        await redis.close()
        redis_status = "healthy"
    except Exception as e:
        redis_status = f"unhealthy: {str(e)}"
    
    if "unhealthy" in postgres_status or "unhealthy" in redis_status:
        raise HTTPException(status_code=503, detail={
            "postgres": postgres_status,
            "redis": redis_status
        })
    
    return {
        "status": "healthy",
        "postgres": postgres_status,
        "redis": redis_status,
        "timestamp": datetime.utcnow().isoformat()
    }

@app.get("/ready")
async def readiness_check():
    """Readiness check for Kubernetes"""
    return {"status": "ready"}
```

### Monitoring Queries

**PostgreSQL Performance Monitoring:**

```sql
-- Connection status
SELECT count(*), state FROM pg_stat_activity GROUP BY state;

-- Slow queries (from pg_stat_statements)
SELECT query, mean_exec_time, calls 
FROM pg_stat_statements 
ORDER BY mean_exec_time DESC 
LIMIT 10;

-- Database size
SELECT pg_database_size('health_export_prod') / 1024 / 1024 AS size_mb;
```

**Redis Performance Monitoring:**

```bash
# Redis info
redis-cli INFO memory
redis-cli INFO stats

# Slow queries
redis-cli SLOWLOG GET 10

# Memory usage
redis-cli MEMORY USAGE <key>
```

## Backup and Recovery

### PostgreSQL Backup

**Development Backup:**
```bash
# Dump development database
docker exec health_export_postgres pg_dump \
  -U health_user health_export_dev > dev_backup.sql

# Restore from backup
docker exec -i health_export_postgres psql \
  -U health_user health_export_dev < dev_backup.sql
```

**Production Backup (Kubernetes):**
```bash
# Create backup
kubectl exec -i postgresql-0 -n health-export -- \
  pg_dump -U health_user health_export_prod > prod_backup.sql

# Schedule regular backups with CronJob
# See k8s/database/backup-cronjob.yaml (create if needed)
```

### Redis Backup

Redis persistence is configured for both RDB snapshots and AOF logs:

- **RDB**: Point-in-time snapshots
- **AOF**: Real-time transaction log
- **Data**: Automatically persisted to Docker volumes or Kubernetes PVCs

## Troubleshooting

### Common Issues

**PostgreSQL Connection Issues:**
```bash
# Check if PostgreSQL is accepting connections
docker exec health_export_postgres pg_isready -U health_user

# View PostgreSQL logs
docker logs health_export_postgres

# Check authentication
docker exec -e PGPASSWORD=dev_password_123 health_export_postgres \
  psql -U health_user -d health_export_dev -c "SELECT current_user;"
```

**Redis Connection Issues:**
```bash
# Test Redis connectivity
docker exec health_export_redis redis-cli ping

# Check Redis logs
docker logs health_export_redis

# Test authentication (if configured)
docker exec health_export_redis redis-cli AUTH your_password
```

**Performance Issues:**
```bash
# PostgreSQL query performance
docker exec -e PGPASSWORD=dev_password_123 health_export_postgres \
  psql -U health_user -d health_export_dev -c \
  "SELECT query, mean_exec_time FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 5;"

# Redis slow queries
docker exec health_export_redis redis-cli SLOWLOG GET 5
```

### Debug Mode

Enable debug profiles for additional development tools:

```bash
# Start with debug tools
docker-compose --profile debug up -d

# Start with monitoring stack
docker-compose --profile monitoring up -d

# Start everything
docker-compose --profile debug --profile monitoring up -d
```

## Security Considerations

### Development Security

- Default passwords are insecure and intended for development only
- No SSL/TLS encryption configured
- Open access from localhost
- Admin interfaces exposed for debugging

### Production Security

- **Strong passwords**: Use secure, unique passwords for all accounts
- **Network isolation**: Deploy within private Kubernetes networks
- **Encrypted connections**: Use SSL/TLS for all database connections
- **Access control**: Implement proper RBAC and network policies
- **Secrets management**: Use Kubernetes secrets or external secret managers
- **Regular updates**: Keep database images updated for security patches

### Security Checklist

- [ ] Change all default passwords
- [ ] Enable SSL/TLS for database connections
- [ ] Implement network policies
- [ ] Set up monitoring and alerting
- [ ] Configure automated backups
- [ ] Review and audit access permissions
- [ ] Enable database audit logging
- [ ] Implement secret rotation policies

## Next Steps

After setting up the database infrastructure:

1. **Application Development**: Use the connection examples to integrate with your FastAPI application
2. **Schema Migration**: Implement Alembic for database schema migrations
3. **Performance Testing**: Load test the database configuration
4. **Monitoring Setup**: Implement comprehensive monitoring and alerting
5. **Production Deployment**: Deploy to Kubernetes following the production guide
6. **Backup Strategy**: Set up automated backup and recovery procedures

For additional support, refer to the main project documentation or raise issues in the project repository.