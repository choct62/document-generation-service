// document-generation-service/src/renderers/markdown.rs

use crate::error::Result;
use crate::models::DocumentMetadata;
use tracing::info;

pub struct MarkdownRenderer;

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self
    }

    pub async fn render(
        &self,
        markdown_content: &str,
        metadata: &DocumentMetadata,
    ) -> Result<Vec<u8>> {
        info!(title = %metadata.title, "Rendering Markdown document");

        // Add front matter to markdown
        let front_matter = format!(
            "---\ntitle: {}\nauthor: {}\nversion: {}\nproject: {}\norganization: {}\ndate: {}\n---\n\n",
            metadata.title,
            metadata.author,
            metadata.version,
            metadata.project_name,
            metadata.organization,
            metadata.generated_date.format("%Y-%m-%d")
        );

        let full_content = format!("{}{}", front_matter, markdown_content);

        info!(
            title = %metadata.title,
            size_kb = full_content.len() / 1024,
            "Markdown generated successfully"
        );

        Ok(full_content.into_bytes())
    }
}
