# MQTT Deployment Guide for Multi-Host Network

## Network Topology
- **192.168.1.102**: Cloudflare Tunnel Host (Gateway)
- **192.168.1.110**: MQTT/Application Host (Your current machine)
- **Domain**: lolzlab.com

---

## 🖥️ ON 192.168.1.102 (Cloudflare Tunnel Host)

This machine acts as the gateway between Cloudflare and your internal network.

### What Goes Here:
1. **cloudflared tunnel client**
2. **Tunnel configuration pointing to 192.168.1.110**

### Setup Instructions for 192.168.1.102:

#### Step 1: Install cloudflared

```bash
# SSH into 192.168.1.102
ssh user@192.168.1.102

# Download and install cloudflared
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb -o cloudflared.deb
sudo dpkg -i cloudflared.deb

# Verify installation
cloudflared --version
```

#### Step 2: Configure Tunnel (if not already configured)

If you already have a tunnel running for other services, just add the MQTT route. If not, create one:

```bash
# Login to Cloudflare (only if not already done)
cloudflared tunnel login

# Create tunnel (only if not already exists)
cloudflared tunnel create lolzlab-tunnel
```

#### Step 3: Update Tunnel Configuration

Edit the tunnel configuration file on 192.168.1.102:

```bash
# Edit existing config or create new
sudo nano /etc/cloudflared/config.yml
```

Add or update with MQTT routing:

```yaml
tunnel: YOUR_EXISTING_TUNNEL_ID
credentials-file: /etc/cloudflared/YOUR_TUNNEL_ID.json

ingress:
  # MQTT WebSocket endpoint - routes to 192.168.1.110
  - hostname: mqtt.lolzlab.com
    service: ws://192.168.1.110:9001
    originRequest:
      noTLSVerify: true
  
  # Your existing Health Export API (if applicable)
  - hostname: api.lolzlab.com
    service: http://192.168.1.110:8080
    
  # Add any other existing services here...
  
  # Catch-all rule (required)
  - service: http_status:404
```

#### Step 4: Add DNS Record (if not exists)

```bash
# Add DNS for MQTT subdomain
cloudflared tunnel route dns lolzlab-tunnel mqtt.lolzlab.com
```

#### Step 5: Restart cloudflared Service

```bash
# Restart to apply new configuration
sudo systemctl restart cloudflared

# Verify it's running
sudo systemctl status cloudflared

# Check logs
sudo journalctl -u cloudflared -f
```

---

## 🖥️ ON 192.168.1.110 (MQTT/Application Host)

This is your current machine where the Health Export API runs.

### What Goes Here:
1. **Mosquitto MQTT Broker**
2. **MQTT-to-Database Bridge (Rust service)**
3. **Your existing Health Export REST API**

### Setup Instructions for 192.168.1.110:

#### Step 1: Install and Configure Mosquitto

```bash
# Update and install Mosquitto
sudo apt update
sudo apt install -y mosquitto mosquitto-clients

# Stop Mosquitto to configure
sudo systemctl stop mosquitto
```

#### Step 2: Configure Mosquitto for WebSocket

```bash
sudo nano /etc/mosquitto/mosquitto.conf
```

Configure to accept connections from 192.168.1.102:

```conf
# Basic Configuration
pid_file /var/run/mosquitto/mosquitto.pid
persistence true
persistence_location /var/lib/mosquitto/
log_dest file /var/log/mosquitto/mosquitto.log
log_dest stdout

# Standard MQTT Listener (local testing only)
listener 1883 localhost
protocol mqtt

# WebSocket Listener - IMPORTANT: Listen on all interfaces
listener 9001 0.0.0.0
protocol websockets
http_dir /usr/share/mosquitto

# Security Configuration
allow_anonymous false
password_file /etc/mosquitto/passwd

# Message Settings
max_queued_messages 1000
max_inflight_messages 20
max_keepalive 60
```

#### Step 3: Create MQTT Credentials

```bash
# Create password file with user for Auto Health Export
sudo mosquitto_passwd -c /etc/mosquitto/passwd health_export
# Enter password when prompted (e.g., 'HealthData2024!')

# Set permissions
sudo chown mosquitto:mosquitto /etc/mosquitto/passwd
sudo chmod 600 /etc/mosquitto/passwd
```

#### Step 4: Configure Firewall

```bash
# Allow WebSocket port from tunnel host
sudo ufw allow from 192.168.1.102 to any port 9001

# Or if ufw is not enabled, use iptables
sudo iptables -A INPUT -p tcp -s 192.168.1.102 --dport 9001 -j ACCEPT
```

#### Step 5: Start Mosquitto

```bash
# Enable and start Mosquitto
sudo systemctl enable mosquitto
sudo systemctl start mosquitto

# Verify it's running
sudo systemctl status mosquitto

# Check that it's listening on port 9001
sudo netstat -tlnp | grep 9001
# Should show: tcp6  0  0 :::9001  :::*  LISTEN  -/mosquitto
```

#### Step 6: Test Connection from Tunnel Host

```bash
# From 192.168.1.102, test connection to MQTT
telnet 192.168.1.110 9001
# Should connect successfully

# Or use curl to test WebSocket
curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" \
     -H "Sec-WebSocket-Version: 13" \
     -H "Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" \
     http://192.168.1.110:9001/
```

#### Step 7: Add MQTT Subscriber to Your Rust Application

In your Health Export API project on 192.168.1.110:

```bash
cd /home/ladvien/self-sensored

# Add MQTT dependencies to Cargo.toml
echo '
rumqttc = "0.24"
paho-mqtt = "0.12"' >> Cargo.toml
```

Create the MQTT subscriber service (same as in previous instructions):

```rust
// src/services/mqtt_subscriber.rs
// [Previous MQTT subscriber code from earlier instructions]
```

#### Step 8: Configure MQTT Subscriber

Update the MQTT connection to use WebSocket through localhost (since MQTT is on same machine):

```rust
// In main.rs or mqtt_subscriber.rs
let subscriber = MqttSubscriber::new(
    pool.clone(),
    "ws://localhost:9001",  // Local WebSocket connection
    "health_export",
    "HealthData2024!"
);
```

---

## 📱 Auto Health Export App Configuration

Configure the iOS app with these settings:

- **Server**: `wss://mqtt.lolzlab.com`
- **Port**: `443`
- **Client ID**: `health_export_iphone`
- **Username**: `health_export`
- **Password**: `HealthData2024!` (or whatever you set)
- **Topic**: `health/data/[user_id]`
- **QoS**: `1`
- **Clean Session**: `true`

---

## 🔍 Testing the Complete Flow

### From 192.168.1.110 (MQTT Host):

```bash
# Monitor Mosquitto logs
sudo tail -f /var/log/mosquitto/mosquitto.log

# Subscribe to test topic
mosquitto_sub -h localhost -p 9001 -u health_export -P 'HealthData2024!' -t 'health/data/#' -v
```

### From 192.168.1.102 (Tunnel Host):

```bash
# Check tunnel status
sudo systemctl status cloudflared
sudo journalctl -u cloudflared -f

# Test WebSocket connection to MQTT host
curl http://192.168.1.110:9001
```

### From External (Internet):

```bash
# Test with MQTT client (like MQTT Explorer)
# Server: wss://mqtt.lolzlab.com
# Port: 443
# Should connect through Cloudflare → 192.168.1.102 → 192.168.1.110
```

---

## 🔒 Security Considerations

### On 192.168.1.102:
- Only cloudflared should be exposed
- Firewall should block all other incoming connections
- Use internal network for backend communication

### On 192.168.1.110:
- Mosquitto should only accept connections from 192.168.1.102 on port 9001
- Use firewall rules to restrict access
- Keep MQTT credentials secure
- Monitor logs for unauthorized access attempts

---

## 📊 Network Flow Diagram

```
[iPhone with Auto Health Export]
            ↓
    wss://mqtt.lolzlab.com:443
            ↓
    [Cloudflare Network]
            ↓
    [192.168.1.102:cloudflared]
            ↓
    ws://192.168.1.110:9001
            ↓
    [192.168.1.110:Mosquitto]
            ↓
    [MQTT Subscriber Service]
            ↓
    [PostgreSQL Database]
```

---

## ⚠️ Important Notes

1. **Existing Services**: This setup assumes you might already have cloudflared running on 192.168.1.102 for other services. The configuration just adds MQTT routing.

2. **Port 9001**: Must be open between 192.168.1.102 and 192.168.1.110 internally.

3. **WebSocket Protocol**: The tunnel uses `ws://` internally (not `wss://`) since TLS is handled by Cloudflare.

4. **DNS Setup**: mqtt.lolzlab.com must point to your Cloudflare tunnel.

5. **Testing**: Always test internally first (192.168.1.102 → 192.168.1.110) before testing through Cloudflare.

---

*Created: 2025-09-09*
*Network: 192.168.1.102 (Tunnel) → 192.168.1.110 (MQTT/App)*