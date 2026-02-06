// document-generation-service/src/error.rs

use thiserror::Error;

pub type Result<T> = std::result::Result<T, DocumentError>;

#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("Template error: {0}")]
    TemplateError(#[from] handlebars::TemplateError),

    #[error("Rendering error: {0}")]
    RenderError(#[from] handlebars::RenderError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Pandoc error: {0}")]
    PandocError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid document format: {0}")]
    InvalidFormat(String),

    #[error("Invalid specification type: {0}")]
    InvalidSpecificationType(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Pub/Sub error: {0}")]
    PubSubError(String),

    #[error("Base64 encoding error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),
}

impl DocumentError {
    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.to_string(),
            error_type: match self {
                DocumentError::TemplateError(_) => "template_error",
                DocumentError::RenderError(_) => "render_error",
                DocumentError::IoError(_) => "io_error",
                DocumentError::PandocError(_) => "pandoc_error",
                DocumentError::SerializationError(_) => "serialization_error",
                DocumentError::InvalidFormat(_) => "invalid_format",
                DocumentError::InvalidSpecificationType(_) => "invalid_specification_type",
                DocumentError::MissingField(_) => "missing_field",
                DocumentError::TemplateNotFound(_) => "template_not_found",
                DocumentError::PubSubError(_) => "pubsub_error",
                DocumentError::Base64Error(_) => "base64_error",
                DocumentError::InvalidData(_) => "invalid_data",
                DocumentError::GenerationFailed(_) => "generation_failed",
            }
            .to_string(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub error_type: String,
}
