use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::gcs::{DocumentStorage, RenderedFile, UploadResult};
use crate::persistence::{CreateArtifactInput, CreateDocumentInput, DocumentDb, GeneratedDocument};
use crate::renderer::DocumentRenderer; // existing Handlebars + Pandoc renderer

/// Inbound Pub/Sub message payload for document generation requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentGenerationRequest {
    pub tenant_id: Uuid,
    pub project_id: i64,
    pub template_id: Option<i64>,
    pub correlation_id: Option<Uuid>,
    pub title: String,
    pub document_type: String,
    pub requested_formats: Vec<String>,
    pub input_params: serde_json::Value,
    pub requested_by: i64,
}

/// Orchestrates: create record → render → upload → persist artifacts → mark complete.
pub struct DocumentPipeline {
    db: DocumentDb,
    storage: DocumentStorage,
    renderer: DocumentRenderer,
}

impl DocumentPipeline {
    pub fn new(db: DocumentDb, storage: DocumentStorage, renderer: DocumentRenderer) -> Self {
        Self { db, storage, renderer }
    }

    /// Main entry point called from the Pub/Sub message handler.
    #[instrument(skip(self, req), fields(
        tenant_id = %req.tenant_id,
        project_id = req.project_id,
        doc_type = %req.document_type
    ))]
    pub async fn process(&self, req: DocumentGenerationRequest) -> Result<GeneratedDocument> {
        // 1. Insert document record as 'queued'
        let doc = self
            .db
            .create_document(&CreateDocumentInput {
                tenant_id: req.tenant_id,
                project_id: req.project_id,
                template_id: req.template_id,
                correlation_id: req.correlation_id,
                title: req.title.clone(),
                document_type: req.document_type.clone(),
                requested_formats: req.requested_formats.clone(),
                input_params: req.input_params.clone(),
                requested_by: req.requested_by,
            })
            .await
            .context("Failed to create document record")?;

        info!(document_id = doc.id, "Created document record");

        // 2. Transition to 'processing'
        self.db
            .update_document_status(req.tenant_id, doc.id, "processing", None, None)
            .await?;

        // 3. Resolve template
        let template_content = if let Some(tid) = req.template_id {
            let tpl = self
                .db
                .get_template(req.tenant_id, tid)
                .await?
                .context("Requested template not found")?;
            tpl.template_content
        } else {
            // Fall back to default template for this document type
            let tpl = self
                .db
                .get_template_by_type(req.tenant_id, &req.document_type, "pdf")
                .await?
                .context("No default template found for document type")?;
            tpl.template_content
        };

        // 4. Render all requested formats
        self.db
            .update_document_status(req.tenant_id, doc.id, "rendering", None, None)
            .await?;

        let rendered_files = match self
            .render_all_formats(&template_content, &req.input_params, &req.requested_formats, &req.title)
            .await
        {
            Ok(files) => files,
            Err(e) => {
                let err_msg = format!("Rendering failed: {e:#}");
                error!(document_id = doc.id, error = %err_msg, "Render failure");
                let failed = self
                    .db
                    .update_document_status(req.tenant_id, doc.id, "failed", Some(&err_msg), None)
                    .await?;
                return Ok(failed);
            }
        };

        // 5. Upload to GCS
        self.db
            .update_document_status(req.tenant_id, doc.id, "uploading", None, None)
            .await?;

        let upload_results = match self
            .storage
            .upload_all_artifacts(req.tenant_id, req.project_id, doc.id, &rendered_files)
            .await
        {
            Ok(results) => results,
            Err(e) => {
                let err_msg = format!("GCS upload failed: {e:#}");
                error!(document_id = doc.id, error = %err_msg, "Upload failure");
                let failed = self
                    .db
                    .update_document_status(req.tenant_id, doc.id, "failed", Some(&err_msg), None)
                    .await?;
                return Ok(failed);
            }
        };

        // 6. Persist artifact metadata rows
        for result in &upload_results {
            self.db
                .create_artifact(&CreateArtifactInput {
                    tenant_id: req.tenant_id,
                    document_id: doc.id,
                    format: result.format.clone(),
                    file_name: result.file_name.clone(),
                    gcs_path: result.gcs_path.clone(),
                    file_size: result.file_size,
                    content_type: result.content_type.clone(),
                    sha256_checksum: result.sha256_checksum.clone(),
                    page_count: result.page_count,
                    rendering_duration_ms: Some(result.rendering_duration_ms),
                })
                .await
                .with_context(|| {
                    format!("Failed to persist artifact metadata for {}", result.format)
                })?;
        }

        // 7. Build generation metadata
        let gen_metadata = serde_json::json!({
            "rendering_engine": "pandoc-xelatex",
            "template_engine": "handlebars",
            "formats_generated": upload_results.iter().map(|r| &r.format).collect::<Vec<_>>(),
            "total_size_bytes": upload_results.iter().map(|r| r.file_size).sum::<i64>(),
            "completed_at": Utc::now().to_rfc3339(),
        });

        // 8. Mark completed
        let completed = self
            .db
            .update_document_status(
                req.tenant_id,
                doc.id,
                "completed",
                None,
                Some(&gen_metadata),
            )
            .await?;

        info!(
            document_id = doc.id,
            artifacts = upload_results.len(),
            "Document generation completed successfully"
        );

        Ok(completed)
    }

    /// Render the template into each requested format via the existing renderer.
    async fn render_all_formats(
        &self,
        template_content: &str,
        input_params: &serde_json::Value,
        formats: &[String],
        title: &str,
    ) -> Result<Vec<RenderedFile>> {
        let mut files = Vec::with_capacity(formats.len());

        for fmt in formats {
            let start = std::time::Instant::now();

            let (data, content_type, extension) = match fmt.as_str() {
                "pdf" => {
                    let html = self.renderer.render_handlebars(template_content, input_params)?;
                    let pdf = self.renderer.html_to_pdf(&html).await?;
                    (pdf, "application/pdf".to_string(), "pdf")
                }
                "html" => {
                    let html = self.renderer.render_handlebars(template_content, input_params)?;
                    (html.into_bytes(), "text/html; charset=utf-8".to_string(), "html")
                }
                "markdown" => {
                    let md = self.renderer.render_handlebars_markdown(template_content, input_params)?;
                    (md.into_bytes(), "text/markdown; charset=utf-8".to_string(), "md")
                }
                other => anyhow::bail!("Unsupported format: {}", other),
            };

            let duration_ms = start.elapsed().as_millis() as i32;
            let sanitized_title = title
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                .collect::<String>();

            files.push(RenderedFile {
                format: fmt.clone(),
                content_type,
                file_name: format!("{}_{}.{}", sanitized_title, Utc::now().format("%Y%m%d_%H%M%S"), extension),
                data,
                rendering_duration_ms: duration_ms,
                page_count: None, // Could be extracted from PDF metadata if needed
            });
        }

        Ok(files)
    }
}
