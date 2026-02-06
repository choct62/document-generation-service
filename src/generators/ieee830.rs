// document-generation-service/src/generators/ieee830.rs

use crate::error::Result;
use crate::generators::Generator;
use crate::models::DocumentMetadata;
use async_trait::async_trait;
use handlebars::Handlebars;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct IEEE830Generator {
    handlebars: Arc<RwLock<Handlebars<'static>>>,
}

impl IEEE830Generator {
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
impl Generator for IEEE830Generator {
    async fn generate(&self, data: &Value, metadata: &DocumentMetadata) -> Result<String> {
        info!(
            title = %metadata.title,
            "Generating IEEE 830 document"
        );

        // Load template
        self.load_template("ieee830_srs").await?;

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
        let rendered = hb.render("ieee830_srs", &context)?;

        info!(
            title = %metadata.title,
            size_bytes = rendered.len(),
            "IEEE 830 document generated"
        );

        Ok(rendered)
    }
}
