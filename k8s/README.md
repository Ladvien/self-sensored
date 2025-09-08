# Kubernetes Deployment Guide

This directory contains Kubernetes manifests for deploying the Health Export REST API infrastructure in production.

## Overview

The infrastructure consists of:
- **PostgreSQL 15** with PostGIS extension for geospatial data storage
- **Redis 7.x** for caching and rate limiting
- **Production-ready configurations** with proper security, monitoring, and persistence

## Prerequisites

1. **Kubernetes Cluster** (v1.24+)
2. **kubectl** configured for your cluster
3. **Storage Classes** - Update `storageClassName` in PVCs based on your cluster
4. **RBAC permissions** to create namespaces, secrets, and resources

## Quick Start

### 1. Create Namespace and Secrets

```bash
# Create the namespace
kubectl apply -f namespace.yaml

# IMPORTANT: Update passwords in secret files before applying
# Edit postgresql-secret.yaml and redis-secret.yaml with production passwords

# Apply secrets
kubectl apply -f database/postgresql-secret.yaml
kubectl apply -f redis/redis-secret.yaml
```

### 2. Deploy PostgreSQL

```bash
# Apply ConfigMaps and PVC
kubectl apply -f database/postgresql-configmap.yaml

# Deploy PostgreSQL StatefulSet
kubectl apply -f database/postgresql-statefulset.yaml

# Create Services
kubectl apply -f database/postgresql-services.yaml

# Wait for PostgreSQL to be ready
kubectl wait --for=condition=ready pod -l app=postgresql -n health-export --timeout=300s
```

### 3. Deploy Redis

```bash
# Apply ConfigMap and PVC
kubectl apply -f redis/redis-configmap.yaml
kubectl apply -f redis/redis-pvc.yaml

# Deploy Redis
kubectl apply -f redis/redis-deployment.yaml
kubectl apply -f redis/redis-service.yaml

# Wait for Redis to be ready
kubectl wait --for=condition=ready pod -l app=redis -n health-export --timeout=300s
```

### 4. Verify Deployment

```bash
# Check all pods are running
kubectl get pods -n health-export

# Check services
kubectl get services -n health-export

# Check PVCs
kubectl get pvc -n health-export

# Test PostgreSQL connection
kubectl exec -it postgresql-0 -n health-export -- psql -U health_user -d health_export_prod -c "SELECT version();"

# Test Redis connection
kubectl exec -it deployment/redis -n health-export -- redis-cli ping
```

## Configuration Details

### PostgreSQL Configuration

- **Image**: `postgis/postgis:15-3.3`
- **Resources**: 512Mi-2Gi memory, 250m-1000m CPU
- **Storage**: 20Gi persistent volume (adjust as needed)
- **Extensions**: PostGIS, uuid-ossp, pgcrypto, pg_stat_statements
- **Security**: SCRAM-SHA-256 authentication

**Key Features:**
- Production-optimized PostgreSQL settings
- PostGIS for geospatial workout data
- Automated extension installation
- Performance monitoring with pg_stat_statements
- Proper backup directory mounts

### Redis Configuration

- **Image**: `redis:7.2-alpine`
- **Resources**: 128Mi-512Mi memory, 100m-500m CPU
- **Storage**: 10Gi persistent volume
- **Security**: Password authentication, restricted commands
- **Persistence**: Both RDB and AOF enabled

**Key Features:**
- Production-ready Redis configuration
- Memory optimization (384MB limit)
- Security hardening (dangerous commands disabled)
- Persistence for cache durability
- Performance monitoring

## Security Considerations

### Secrets Management

**CRITICAL**: Change all default passwords before deploying to production:

1. **PostgreSQL Secret** (`database/postgresql-secret.yaml`):
   - `password`: Main database password
   - `database-url`: Full connection string

2. **Redis Secret** (`redis/redis-secret.yaml`):
   - `password`: Redis authentication password
   - `redis-url`: Connection URL

### Password Updates

```bash
# Generate new base64 encoded passwords
echo -n "your_secure_password" | base64

# Update the secret files with new values
# Then reapply:
kubectl apply -f database/postgresql-secret.yaml
kubectl apply -f redis/redis-secret.yaml

# Restart services to pick up new passwords
kubectl rollout restart statefulset/postgresql -n health-export
kubectl rollout restart deployment/redis -n health-export
```

## Storage Configuration

Update `storageClassName` in these files based on your cluster:

- `database/postgresql-statefulset.yaml` (line 114)
- `redis/redis-pvc.yaml` (line 15)

Common storage classes:
- `gp3` (AWS EBS)
- `fast-ssd` (GKE)
- `managed-premium` (Azure)
- `local-path` (k3s/development)

## Monitoring and Health Checks

### Health Check Endpoints

Both services include comprehensive health checks:

**PostgreSQL:**
- Liveness probe: `pg_isready` every 30s
- Readiness probe: `pg_isready` every 10s

**Redis:**
- Liveness probe: `redis-cli ping` every 30s  
- Readiness probe: `redis-cli ping` every 10s

### Monitoring Integration

The configuration includes:
- **PostgreSQL**: pg_stat_statements for query monitoring
- **Redis**: Slow log configuration and keyspace notifications
- **Logging**: Structured logging to stderr/syslog

## Backup and Recovery

### PostgreSQL Backup

```bash
# Manual backup
kubectl exec -it postgresql-0 -n health-export -- \
  pg_dump -U health_user health_export_prod > backup.sql

# Automated backup (recommended to set up)
# Consider tools like:
# - velero for cluster-wide backups
# - pg_basebackup for PostgreSQL-specific backups
# - External backup solutions (AWS RDS, GCP Cloud SQL)
```

### Redis Backup

Redis persistence is configured with both RDB and AOF:
- RDB snapshots every 15 minutes (if data changes)
- AOF for real-time durability
- Data stored in persistent volumes

## Troubleshooting

### Common Issues

1. **Pods stuck in Pending**:
   ```bash
   kubectl describe pod <pod-name> -n health-export
   # Check for storage/resource constraints
   ```

2. **PostgreSQL connection issues**:
   ```bash
   kubectl logs postgresql-0 -n health-export
   # Check authentication and network connectivity
   ```

3. **Redis authentication failures**:
   ```bash
   kubectl logs deployment/redis -n health-export
   # Verify password in secret matches configuration
   ```

### Useful Commands

```bash
# View logs
kubectl logs postgresql-0 -n health-export
kubectl logs deployment/redis -n health-export

# Describe resources
kubectl describe statefulset postgresql -n health-export
kubectl describe deployment redis -n health-export

# Port forwarding for local testing
kubectl port-forward postgresql-0 5432:5432 -n health-export
kubectl port-forward deployment/redis 6379:6379 -n health-export
```

## Development vs Production

This configuration is optimized for production. For development:

1. Use the `docker-compose.yml` in the project root
2. Reduce resource requests/limits
3. Use simpler storage (local-path or hostPath)
4. Consider using `NodePort` services for external access

## Next Steps

After successful deployment:

1. **Deploy the application** (Health Export REST API)
2. **Configure monitoring** (Prometheus/Grafana)
3. **Set up backups** (automated backup solution)
4. **Configure ingress** (for external API access)
5. **Implement CI/CD** (automated deployments)

## Support

For issues with this deployment:

1. Check the troubleshooting section above
2. Review pod logs and events
3. Verify storage and resource availability
4. Consult the main project README.md for application-specific guidance