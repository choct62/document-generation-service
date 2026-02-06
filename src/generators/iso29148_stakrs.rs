// document-generation-service/src/generators/iso29148_stakrs.rs

use crate::error::Result;
use crate::generators::Generator;
use crate::models::DocumentMetadata;
use async_trait::async_trait;
use handlebars::Handlebars;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct ISO29148StakRSGenerator {
    handlebars: Arc<RwLock<Handlebars<'static>>>,
}

impl ISO29148StakRSGenerator {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);

        Self {
            handlebars: Arc::new(RwLock::new(handlebars)),
        }
    }

    async fn load_template(&self, template_name: &str) -> Result<()> {
        let template_path = format!("./templates/{}.md.hbs", template_name);
        let mut hb = self.handlebars.write().await;

        match hb.register_template_file(template_name, &template_path) {
            Ok(_) => Ok(()),
            Err(e) => Err(crate::error::DocumentError::TemplateError(e)),
        }
    }
}

#[async_trait]
impl Generator for ISO29148StakRSGenerator {
    async fn generate(&self, data: &Value, metadata: &DocumentMetadata) -> Result<String> {
        info!(
            title = %metadata.title,
            "Generating ISO/IEC/IEEE 29148:2018 Stakeholder Requirements Specification"
        );

        // Load template
        self.load_template("iso29148_stakrs").await?;

        // Combine metadata and data for template context
        let mut context = serde_json::json!({
            "metadata": metadata,
            "data": data,
        });

        // Merge if data is an object
        if let Value::Object(map) = data {
            if let Value::Object(ref mut ctx_map) = context {
                for (key, value) in map {
                    ctx_map.insert(key.clone(), value.clone());
                }
            }
        }

        let hb = self.handlebars.read().await;
        let rendered = hb.render("iso29148_stakrs", &context)?;

        info!(
            title = %metadata.title,
            size_bytes = rendered.len(),
            "ISO 29148 StakRS document generated"
        );

        Ok(rendered)
    }
}
