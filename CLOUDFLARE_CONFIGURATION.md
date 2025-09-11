# Cloudflare Configuration for AUDIT-003 - Server Availability (520 Error Prevention)

This document provides comprehensive Cloudflare configuration requirements to prevent 520 "Web server is returning an unknown error" responses.

## 🚨 Critical Understanding: Cloudflare 520 Errors

**520 errors occur when:**
- Origin server returns an invalid or unexpected response
- Origin server takes too long to respond (>100 seconds by default)
- Origin server connections are unexpectedly dropped
- Origin server returns invalid HTTP headers
- SSL/TLS handshake fails between Cloudflare and origin

## 🔧 Origin Server Optimizations (Implemented)

### Enhanced Health Check Endpoints

1. **Basic Health Check** (`/health`)
   - Enhanced diagnostics with server information
   - Cloudflare-specific debug information
   - Connection keep-alive headers
   - Sub-50ms response time target

2. **Liveness Probe** (`/health/live`)
   - Ultra-fast response for container orchestration
   - Minimal resource usage
   - Docker/Kubernetes health check compatible

3. **Readiness Probe** (`/health/ready`)
   - Database connectivity verification
   - Service dependency checks
   - Returns 503 when not ready (appropriate for load balancer removal)

4. **Comprehensive Status** (`/api/v1/status`)
   - Full system diagnostics
   - Database performance metrics
   - Connection pool statistics
   - Resource utilization information

### Server Configuration Improvements

```rust
// Cloudflare-optimized timeout settings
REQUEST_TIMEOUT_SECONDS=90          // Under Cloudflare's 100s limit
CONNECTION_TIMEOUT_SECONDS=30       // Quick connection establishment
KEEP_ALIVE_TIMEOUT_SECONDS=75       // Maintain connections under CF limit
CLIENT_SHUTDOWN_TIMEOUT_SECONDS=30  // Graceful client disconnection
SERVER_SHUTDOWN_TIMEOUT_SECONDS=30  // Graceful server shutdown
```

### Response Headers for Cloudflare Compatibility

All health endpoints now include:
```
Cache-Control: no-cache, no-store, must-revalidate
Connection: keep-alive
X-Origin-Server: self-sensored-api
X-Health-Check-ID: <unique-id>
```

## ☁️ Cloudflare Dashboard Configuration

### SSL/TLS Settings

1. **SSL/TLS encryption mode**: `Full (strict)`
   ```
   Crypto > SSL/TLS > Overview > Full (strict)
   ```

2. **Origin Server Certificates**
   ```
   Crypto > SSL/TLS > Origin Server > Create Certificate
   ```
   - Generate Cloudflare Origin Certificate
   - Install on origin server
   - Configure in application environment:
     ```bash
     DATABASE_SSL_MODE=require
     DATABASE_SSL_CERT_PATH=/path/to/cloudflare-cert.pem
     DATABASE_SSL_KEY_PATH=/path/to/cloudflare-key.pem
     ```

3. **TLS Version**
   ```
   Crypto > SSL/TLS > Edge Certificates > Minimum TLS Version: 1.2
   ```

### Caching Configuration

1. **Browser Cache TTL**: `4 hours` (for static assets)
2. **Edge Cache TTL**: Custom rules for API endpoints

   **Page Rules:**
   ```
   Pattern: api.yourdomain.com/health*
   Settings: Cache Level = Bypass
   
   Pattern: api.yourdomain.com/api/v1/status
   Settings: Cache Level = Bypass
   
   Pattern: api.yourdomain.com/metrics
   Settings: Cache Level = Bypass
   
   Pattern: api.yourdomain.com/api/v1/*
   Settings: 
     - Cache Level = Bypass
     - Security Level = Medium
     - Browser Integrity Check = On
   ```

### Performance & Reliability

1. **Load Balancing** (Enterprise/Business)
   ```
   Traffic > Load Balancing
   - Health Check URL: https://api.yourdomain.com/health/ready
   - Health Check Interval: 15 seconds
   - Health Check Retries: 2
   - Health Check Timeout: 5 seconds
   - Health Check Expected Codes: 200
   ```

2. **Origin Rules**
   ```
   Rules > Origin Rules
   - Host Header Override: Preserve original
   - Resolve Override: Use origin IP
   ```

3. **Timeout Configuration**
   ```
   Network > Response Buffering: Off (for large payloads)
   Network > HTTP/2 to Origin: On
   Network > gRPC: Off (not applicable)
   ```

### Security Settings

1. **Security Level**: `Medium`
   ```
   Security > Settings > Security Level: Medium
   ```

2. **DDoS Protection**
   ```
   Security > DDoS > HTTP DDoS Attack Protection: On
   Security > DDoS > Network-layer DDoS Attack Protection: On
   ```

3. **WAF (Web Application Firewall)**
   ```
   Security > WAF > Managed Rules
   - Cloudflare Managed Ruleset: On
   - Skip specific rules for health checks:
     - Skip Rule ID 100XXX for /health* paths
   ```

4. **Rate Limiting**
   ```
   Security > Rate Limiting Rules
   Rule: API Protection
   - Match: api.yourdomain.com/api/v1/ingest*
   - Rate: 100 requests per 5 minutes per IP
   - Action: Block
   
   Rule: Health Check Allow
   - Match: api.yourdomain.com/health*
   - Rate: 1000 requests per minute
   - Action: Allow
   ```

### DNS Configuration

1. **DNS Records**
   ```
   DNS > Records
   Type: A
   Name: api (or your subdomain)
   IPv4 address: YOUR_ORIGIN_IP
   Proxy status: Proxied (orange cloud)
   TTL: Auto
   ```

2. **CNAME Flattening**: `Flatten at root` (if using root domain)

## 🔧 Advanced Cloudflare Configuration

### Custom Rules (Enterprise)

1. **Origin Request Headers**
   ```javascript
   // Add debugging headers to origin requests
   Rules > Transform Rules > Managed Transforms
   - Add: "CF-Connecting-IP" header
   - Add: "CF-Ray" header for tracking
   - Add: "CF-Request-ID" for debugging
   ```

2. **Response Headers**
   ```javascript
   // Add cache and security headers
   Rules > Transform Rules > Modify Response Header
   - Add: "X-Cloudflare-Cache": "MISS/HIT/DYNAMIC"
   - Add: "X-Response-Time": "${cf.response_time}"
   ```

### Health Check Monitoring Script

```bash
#!/bin/bash
# cloudflare-health-monitor.sh
# Run this script periodically to verify Cloudflare <-> Origin connectivity

ORIGIN_IP="YOUR_ORIGIN_IP"
CLOUDFLARE_DOMAIN="api.yourdomain.com"

echo "Testing direct origin connection..."
curl -H "Host: $CLOUDFLARE_DOMAIN" \
     -w "Status: %{http_code}, Time: %{time_total}s\n" \
     -s -o /dev/null \
     "http://$ORIGIN_IP/health/live"

echo "Testing through Cloudflare..."
curl -w "Status: %{http_code}, Time: %{time_total}s, CF-Ray: %{header_cf-ray}\n" \
     -s -o /dev/null \
     "https://$CLOUDFLARE_DOMAIN/health/live"

echo "Testing comprehensive status..."
curl -w "Status: %{http_code}, Time: %{time_total}s\n" \
     -s \
     "https://$CLOUDFLARE_DOMAIN/api/v1/status" | jq '.cloudflare_diagnostics'
```

## 🚨 Troubleshooting 520 Errors

### Diagnostic Steps

1. **Check Origin Response**
   ```bash
   curl -I http://YOUR_ORIGIN_IP:8080/health
   # Should return HTTP/1.1 200 OK with proper headers
   ```

2. **Verify SSL Certificate**
   ```bash
   openssl s_client -connect YOUR_ORIGIN_IP:443 -servername api.yourdomain.com
   # Check for certificate chain validity
   ```

3. **Test Health Endpoints**
   ```bash
   # Test each endpoint for proper response
   curl https://api.yourdomain.com/health
   curl https://api.yourdomain.com/health/live  
   curl https://api.yourdomain.com/health/ready
   curl https://api.yourdomain.com/api/v1/status
   ```

4. **Monitor Cloudflare Logs**
   ```
   Analytics > Logs > Cloudflare Logs (Enterprise)
   - Filter by Status Code: 520
   - Check Origin Response codes
   - Review Edge Response Times
   ```

### Common 520 Error Causes & Solutions

| Cause | Solution |
|-------|----------|
| Origin timeout > 100s | Reduce `REQUEST_TIMEOUT_SECONDS` to 90 |
| Invalid HTTP headers | Check response headers in health endpoints |
| SSL handshake failure | Configure Origin Server certificates |
| Connection drops | Implement proper graceful shutdown |
| High server load | Scale workers, optimize database queries |
| Network connectivity | Check firewall, security groups |

## 📊 Monitoring & Alerting

### Cloudflare Analytics

1. **Traffic Analytics**
   - Monitor 520 error rate
   - Track response times
   - Monitor cache hit ratios

2. **Security Analytics**
   - Review blocked requests
   - Monitor rate limiting triggers
   - Check WAF rule matches

### Prometheus Alerts Integration

Update your monitoring stack to include Cloudflare metrics:

```yaml
# prometheus-cloudflare-alerts.yml
groups:
- name: cloudflare_health
  rules:
  - alert: High520ErrorRate
    expr: rate(cloudflare_http_responses_total{status="520"}[5m]) > 0.01
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "High Cloudflare 520 error rate detected"
      description: "520 errors occurring at {{ $value }}/sec"
      
  - alert: OriginHealthCheckFailing
    expr: cloudflare_origin_health_status != 1
    for: 30s
    labels:
      severity: critical
    annotations:
      summary: "Origin server health check failing"
      description: "Cloudflare cannot reach origin server health endpoint"
```

## ✅ Verification Checklist

- [ ] Origin server responds to health checks within 5 seconds
- [ ] Health endpoints return proper HTTP headers
- [ ] SSL/TLS configuration is correct (Full Strict mode)
- [ ] Timeouts are configured below Cloudflare limits
- [ ] Cache bypass rules are set for API endpoints
- [ ] Rate limiting allows health checks
- [ ] Load balancer health checks are configured
- [ ] Monitoring alerts are active
- [ ] 520 error rate is below 0.1%
- [ ] Origin server logs show no connection errors

## 🔄 Regular Maintenance

1. **Weekly**
   - Review 520 error logs
   - Check origin server performance metrics
   - Verify health check response times

2. **Monthly**
   - Review and rotate SSL certificates
   - Analyze traffic patterns and adjust rate limits
   - Update timeout configurations based on performance data

3. **Quarterly**
   - Review Cloudflare plan features and upgrade if needed
   - Audit security rules and update WAF configuration
   - Performance test origin server under load

---

**Last Updated**: 2025-09-11  
**Version**: 1.0.0  
**Status**: Ready for Production