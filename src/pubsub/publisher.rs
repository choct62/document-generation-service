// document-generation-service/src/pubsub/publisher.rs

use crate::models::DocumentGenerationResponse;
use google_cloud_pubsub::client::{Client, ClientConfig};
use google_cloud_pubsub::publisher::Publisher as PubSubPublisher;
use tracing::{error, info};

pub struct Publisher {
    publisher: PubSubPublisher,
    topic_name: String,
}

impl Publisher {
    pub async fn new(project_id: &str, topic_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = ClientConfig::default().with_auth().await?;
        let client = Client::new(config).await?;

        let topic = client.topic(topic_name);
        let publisher = topic.new_publisher(None);

        info!(
            project_id = %project_id,
            topic = %topic_name,
            "Publisher initialized"
        );

        Ok(Self {
            publisher,
            topic_name: topic_name.to_string(),
        })
    }

    pub async fn publish_response(&self, response: &DocumentGenerationResponse) {
        let json_data = match serde_json::to_vec(response) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to serialize response: {}", e);
                return;
            }
        };

        // Create PubsubMessage using googleapis
        let message = google_cloud_googleapis::pubsub::v1::PubsubMessage {
            data: json_data,
            attributes: vec![
                ("request_id".to_string(), response.request_id.clone()),
                ("status".to_string(), response.status.clone()),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let awaiter = self.publisher.publish(message).await;

        match awaiter.get().await {
            Ok(message_id) => {
                info!(
                    request_id = %response.request_id,
                    message_id = %message_id,
                    topic = %self.topic_name,
                    "Response published successfully"
                );
            }
            Err(e) => {
                error!(
                    request_id = %response.request_id,
                    error = %e,
                    "Failed to publish response"
                );
            }
        }
    }
}
