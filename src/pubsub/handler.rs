// document-generation-service/src/pubsub/handler.rs

use crate::error::Result;
use crate::models::{
    DocumentFormat, DocumentGenerationRequest, DocumentGenerationResponse, GeneratedDocument,
};
use crate::renderers::{HtmlRenderer, MarkdownRenderer, PdfRenderer};
use base64::{engine::general_purpose, Engine as _};
use tracing::{error, info, warn};

pub struct MessageHandler {
    pdf_renderer: PdfRenderer,
    markdown_renderer: MarkdownRenderer,
    html_renderer: HtmlRenderer,
}

impl MessageHandler {
    pub fn new() -> Self {
        Self {
            pdf_renderer: PdfRenderer::new(),
            markdown_renderer: MarkdownRenderer::new(),
            html_renderer: HtmlRenderer::new(),
        }
    }

    pub async fn handle_message(&self, data: &[u8]) -> DocumentGenerationResponse {
        // Parse the request
        let request: DocumentGenerationRequest = match serde_json::from_slice(data) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse request: {}", e);
                return DocumentGenerationResponse::error(
                    "unknown".to_string(),
                    format!("Invalid request format: {}", e),
                );
            }
        };

        let request_id = uuid::Uuid::new_v4().to_string();

        info!(
            request_id = %request_id,
            spec_type = ?request.specification_type,
            formats = ?request.output_formats,
            "Processing document generation request"
        );

        // Generate the document content
        let generator = match crate::generators::create_generator(&request.specification_type) {
            Ok(gen) => gen,
            Err(e) => {
                error!("Failed to create generator: {}", e);
                return DocumentGenerationResponse::error(request_id, e.to_string());
            }
        };

        let markdown_content = match generator.generate(&request.data, &request.metadata).await {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to generate content: {}", e);
                return DocumentGenerationResponse::error(request_id, e.to_string());
            }
        };

        // Render in requested formats
        let mut documents = Vec::new();

        for format in &request.output_formats {
            match self
                .render_document(format, &markdown_content, &request.metadata)
                .await
            {
                Ok(doc) => documents.push(doc),
                Err(e) => {
                    warn!("Failed to render {} format: {}", format_name(format), e);
                    // Continue with other formats instead of failing completely
                }
            }
        }

        if documents.is_empty() {
            error!("Failed to generate any documents");
            return DocumentGenerationResponse::error(
                request_id,
                "Failed to generate documents in any requested format".to_string(),
            );
        }

        info!(
            request_id = %request_id,
            document_count = documents.len(),
            "Successfully generated documents"
        );

        DocumentGenerationResponse::success(request_id, documents)
    }

    async fn render_document(
        &self,
        format: &DocumentFormat,
        markdown_content: &str,
        metadata: &crate::models::DocumentMetadata,
    ) -> Result<GeneratedDocument> {
        let (content_bytes, mime_type, extension) = match format {
            DocumentFormat::PDF => {
                let bytes = self.pdf_renderer.render(markdown_content, metadata).await?;
                (bytes, "application/pdf", "pdf")
            }
            DocumentFormat::Markdown => {
                let bytes = self
                    .markdown_renderer
                    .render(markdown_content, metadata)
                    .await?;
                (bytes, "text/markdown", "md")
            }
            DocumentFormat::HTML => {
                let bytes = self
                    .html_renderer
                    .render(markdown_content, metadata)
                    .await?;
                (bytes, "text/html", "html")
            }
        };

        let content_base64 = general_purpose::STANDARD.encode(&content_bytes);
        let size_bytes = content_bytes.len();

        let filename = format!(
            "{}-v{}.{}",
            metadata.title.to_lowercase().replace(' ', "-"),
            metadata.version,
            extension
        );

        Ok(GeneratedDocument {
            format: format.clone(),
            content_base64,
            filename,
            mime_type: mime_type.to_string(),
            size_bytes,
        })
    }
}

fn format_name(format: &DocumentFormat) -> &str {
    match format {
        DocumentFormat::PDF => "PDF",
        DocumentFormat::Markdown => "Markdown",
        DocumentFormat::HTML => "HTML",
    }
}
