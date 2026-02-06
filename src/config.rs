// document-generation-service/src/config.rs

use config::{Config as ConfigLoader, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub service: ServiceConfig,
    pub pubsub: PubSubConfig,
    pub templates: TemplateConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PubSubConfig {
    pub project_id: String,
    pub request_subscription: String,
    pub response_topic: String,
    pub max_concurrent_messages: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateConfig {
    pub path: String,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config = ConfigLoader::builder()
            // Start with default values
            .set_default("service.name", "document-generation-service")?
            .set_default("service.log_level", "info")?
            .set_default("pubsub.project_id", "mcxtest")?
            .set_default("pubsub.request_subscription", "document-generation-requests-sub")?
            .set_default("pubsub.response_topic", "document-generation-results")?
            .set_default("pubsub.max_concurrent_messages", "10")?
            .set_default("templates.path", "./templates")?
            // Load from config file if it exists
            .add_source(File::with_name("config").required(false))
            // Override with environment variables (e.g., SERVICE__NAME)
            .add_source(Environment::with_prefix("SERVICE").separator("__"))
            .build()?;

        config.try_deserialize()
    }
}
