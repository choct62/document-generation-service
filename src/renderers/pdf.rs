// document-generation-service/src/renderers/pdf.rs

use crate::error::Result;
use crate::models::DocumentMetadata;
use std::process::Command;
use tempfile::NamedTempFile;
use tokio::fs;
use tracing::{debug, info};

pub struct PdfRenderer;

impl PdfRenderer {
    pub fn new() -> Self {
        Self
    }

    pub async fn render(
        &self,
        markdown_content: &str,
        metadata: &DocumentMetadata,
    ) -> Result<Vec<u8>> {
        info!(title = %metadata.title, "Rendering PDF document");

        // Create temporary files
        let mut md_file = NamedTempFile::new()?;
        let pdf_file = NamedTempFile::new()?;

        // Write markdown to temp file
        use std::io::Write;
        md_file.write_all(markdown_content.as_bytes())?;
        md_file.flush()?;

        debug!("Markdown written to: {:?}", md_file.path());

        // Build Pandoc command
        let mut cmd = Command::new("pandoc");
        cmd.arg(md_file.path())
            .arg("-o")
            .arg(pdf_file.path())
            .arg("--from=markdown+yaml_metadata_block+hard_line_breaks")
            .arg("--to=pdf")
            .arg("--pdf-engine=xelatex")
            .arg("--toc")
            .arg("--toc-depth=3")
            .arg("--number-sections")
            .arg("-V")
            .arg("geometry:margin=1in")
            .arg("-V")
            .arg("fontsize=11pt")
            .arg("-V")
            .arg("documentclass=article")
            .arg("-V")
            .arg(format!("title={}", metadata.title))
            .arg("-V")
            .arg(format!("author={}", metadata.author))
            .arg("-V")
            .arg(format!("date={}", metadata.generated_date.format("%B %d, %Y")));

        // Add classification if present (for military docs)
        if let Some(classification) = &metadata.classification {
            cmd.arg("-V")
                .arg(format!("header-includes=\\markboth{{{}}}{{{}}}",
                    classification, classification));
        }

        debug!("Running Pandoc: {:?}", cmd);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::error::DocumentError::PandocError(stderr.to_string()).into());
        }

        // Read PDF bytes
        let pdf_bytes = fs::read(pdf_file.path()).await?;

        info!(
            title = %metadata.title,
            size_kb = pdf_bytes.len() / 1024,
            "PDF generated successfully"
        );

        Ok(pdf_bytes)
    }
}
