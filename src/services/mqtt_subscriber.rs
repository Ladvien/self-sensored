use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, Transport};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct MqttSubscriber {
    pool: PgPool,
}

impl MqttSubscriber {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting MQTT subscriber service");

        // Configure MQTT connection for WebSocket
        let mut mqtt_options = MqttOptions::new("health_export_subscriber", "localhost", 9001);

        mqtt_options.set_keep_alive(Duration::from_secs(60));
        mqtt_options.set_credentials("health_export", "HealthData2024!");
        mqtt_options.set_clean_session(false);

        // Set WebSocket transport
        mqtt_options.set_transport(Transport::Ws);

        let (client, mut eventloop) = AsyncClient::new(mqtt_options, 100);

        // Subscribe to health data topics
        let topic = "health/data/+";
        client.subscribe(topic, QoS::AtLeastOnce).await?;
        info!("Subscribed to MQTT topic: {}", topic);

        // Process incoming messages
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    let topic = &publish.topic;
                    let payload = std::str::from_utf8(&publish.payload).unwrap_or("{}");

                    info!(
                        "Received MQTT message on topic {}: {} bytes",
                        topic,
                        payload.len()
                    );

                    // Process the health data
                    if let Err(e) = self.process_health_data(topic, payload).await {
                        error!("Failed to process health data: {}", e);
                    }
                }
                Ok(Event::Incoming(Packet::ConnAck(connack))) => {
                    info!("Connected to MQTT broker: {:?}", connack);
                }
                Ok(Event::Incoming(Packet::SubAck(suback))) => {
                    info!("Subscription confirmed: {:?}", suback);
                }
                Ok(Event::Incoming(Packet::Disconnect)) => {
                    warn!("Disconnected from MQTT broker");
                    sleep(Duration::from_secs(5)).await;
                }
                Ok(Event::Outgoing(_)) => {
                    // Outgoing events, can be ignored
                }
                Ok(Event::Incoming(_)) => {
                    // Other incoming packets
                }
                Err(e) => {
                    error!("MQTT error: {:?}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn process_health_data(
        &self,
        topic: &str,
        payload: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Parse JSON payload
        let data: Value = match serde_json::from_str(payload) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse JSON payload: {}", e);
                return Err(Box::new(e));
            }
        };

        // Extract user ID from topic (health/data/{user_id})
        let parts: Vec<&str> = topic.split('/').collect();
        let user_id_str = parts.get(2).ok_or("Invalid topic format")?;

        // Try to parse as UUID, or use a default/lookup
        let user_id = if let Ok(uuid) = Uuid::parse_str(user_id_str) {
            uuid
        } else {
            // For Auto Health Export, the user ID might be a device ID
            // You might want to map device IDs to user IDs in a separate table
            info!(
                "Non-UUID user ID received: {}, using default user",
                user_id_str
            );
            // Use a default user ID or look up from a mapping table
            Uuid::parse_str("b0d8f483-fadf-46bb-ad54-fa694238424a")? // Your test user ID
        };

        // Calculate data hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        let data_hash = format!("{:x}", hasher.finalize());

        // Use a dedicated MQTT API key ID (you should create this in your database)
        let api_key_id = Uuid::parse_str("2d56f485-85bc-4337-839d-9b08a6626baf")?; // Your existing test API key

        // Store raw payload in database
        let result = sqlx::query!(
            r#"
            INSERT INTO raw_ingestions (id, user_id, api_key_id, raw_data, data_hash, ingested_at, status) 
            VALUES ($1, $2, $3, $4, $5, NOW(), 'pending')
            ON CONFLICT (user_id, data_hash, ingested_at) DO NOTHING
            RETURNING id
            "#,
            Uuid::new_v4(),
            user_id,
            api_key_id,
            data,
            data_hash
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(record) = result {
            info!(
                "Stored MQTT health data with ID: {} for user: {}",
                record.id, user_id
            );

            // Mark as ready for processing
            sqlx::query!(
                "UPDATE raw_ingestions SET status = 'ready' WHERE id = $1",
                record.id
            )
            .execute(&self.pool)
            .await?;
        } else {
            warn!("Duplicate MQTT data received (already processed)");
        }

        Ok(())
    }
}
