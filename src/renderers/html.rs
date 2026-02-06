// document-generation-service/src/renderers/html.rs

use crate::error::Result;
use crate::models::DocumentMetadata;
use std::process::Command;
use tempfile::NamedTempFile;
use tokio::fs;
use tracing::{debug, info};

pub struct HtmlRenderer;

impl HtmlRenderer {
    pub fn new() -> Self {
        Self
    }

    pub async fn render(
        &self,
        markdown_content: &str,
        metadata: &DocumentMetadata,
    ) -> Result<Vec<u8>> {
        info!(title = %metadata.title, "Rendering HTML document");

        // Create temporary files
        let mut md_file = NamedTempFile::new()?;
        let html_file = NamedTempFile::new()?;

        // Write markdown to temp file
        use std::io::Write;
        md_file.write_all(markdown_content.as_bytes())?;
        md_file.flush()?;

        debug!("Markdown written to: {:?}", md_file.path());

        // Build Pandoc command for HTML
        let mut cmd = Command::new("pandoc");
        cmd.arg(md_file.path())
            .arg("-o")
            .arg(html_file.path())
            .arg("--from=markdown+yaml_metadata_block")
            .arg("--to=html5")
            .arg("--standalone")
            .arg("--toc")
            .arg("--toc-depth=3")
            .arg("--css=https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/5.1.0/github-markdown.min.css")
            .arg("--self-contained")
            .arg("-V")
            .arg(format!("title={}", metadata.title))
            .arg("-V")
            .arg(format!("author={}", metadata.author))
            .arg("-V")
            .arg(format!("date={}", metadata.generated_date.format("%B %d, %Y")));

        debug!("Running Pandoc: {:?}", cmd);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::error::DocumentError::PandocError(stderr.to_string()).into());
        }

        // Read HTML bytes
        let html_bytes = fs::read(html_file.path()).await?;

        info!(
            title = %metadata.title,
            size_kb = html_bytes.len() / 1024,
            "HTML generated successfully"
        );

        Ok(html_bytes)
    }
}
