use anyhow::{Context, Result};
use google_cloud_storage::client::{Client as GcsClient, ClientConfig};
use google_cloud_storage::http::objects::delete::DeleteObjectRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use google_cloud_storage::sign::SignedURLMethod;
use google_cloud_storage::sign::SignedURLOptions;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tracing::{info, instrument};
use uuid::Uuid;

const BUCKET: &str = "mcxtest-attachments";
const SIGNED_URL_EXPIRY: Duration = Duration::from_secs(15 * 60); // 15 minutes

/// Rendered artifact ready for upload.
#[derive(Debug, Clone)]
pub struct RenderedFile {
    pub format: String,
    pub content_type: String,
    pub file_name: String,
    pub data: Vec<u8>,
    pub rendering_duration_ms: i32,
    pub page_count: Option<i32>,
}

/// Result of a successful GCS upload.
#[derive(Debug, Clone)]
pub struct UploadResult {
    pub gcs_path: String,
    pub file_size: i64,
    pub sha256_checksum: String,
    pub format: String,
    pub content_type: String,
    pub file_name: String,
    pub rendering_duration_ms: i32,
    pub page_count: Option<i32>,
}

#[derive(Clone)]
pub struct DocumentStorage {
    client: GcsClient,
    bucket: String,
}

impl DocumentStorage {
    /// Initialise from the mounted GCS service account key.
    pub async fn new() -> Result<Self> {
        let config = ClientConfig::default()
            .with_auth()
            .await
            .context("Failed to initialise GCS client with service account")?;

        let client = GcsClient::new(config);

        Ok(Self {
            client,
            bucket: BUCKET.to_string(),
        })
    }

    /// Build the object path: `{tenant_id}/documents/{project_id}/{document_id}/{filename}`
    fn object_path(
        tenant_id: Uuid,
        project_id: i64,
        document_id: i64,
        file_name: &str,
    ) -> String {
        format!(
            "{}/documents/{}/{}/{}",
            tenant_id, project_id, document_id, file_name
        )
    }

    /// Upload a rendered file to GCS and return metadata including SHA-256 checksum.
    #[instrument(skip(self, file), fields(bucket = %self.bucket, format = %file.format))]
    pub async fn upload_artifact(
        &self,
        tenant_id: Uuid,
        project_id: i64,
        document_id: i64,
        file: &RenderedFile,
    ) -> Result<UploadResult> {
        let gcs_path = Self::object_path(tenant_id, project_id, document_id, &file.file_name);
        let file_size = file.data.len() as i64;

        // Compute SHA-256 checksum
        let mut hasher = Sha256::new();
        hasher.update(&file.data);
        let sha256_checksum = hex::encode(hasher.finalize());

        let upload_type = UploadType::Simple(Media {
            name: gcs_path.clone().into(),
            content_type: file.content_type.clone().into(),
            content_length: Some(file_size as u64),
        });

        self.client
            .upload_object(
                &UploadObjectRequest {
                    bucket: self.bucket.clone(),
                    ..Default::default()
                },
                file.data.clone(),
                &upload_type,
            )
            .await
            .with_context(|| format!("Failed to upload {} to GCS path {}", file.file_name, gcs_path))?;

        info!(
            gcs_path = %gcs_path,
            file_size = file_size,
            sha256 = %sha256_checksum,
            "Uploaded document artifact to GCS"
        );

        Ok(UploadResult {
            gcs_path,
            file_size,
            sha256_checksum,
            format: file.format.clone(),
            content_type: file.content_type.clone(),
            file_name: file.file_name.clone(),
            rendering_duration_ms: file.rendering_duration_ms,
            page_count: file.page_count,
        })
    }

    /// Upload all rendered formats for a document.
    pub async fn upload_all_artifacts(
        &self,
        tenant_id: Uuid,
        project_id: i64,
        document_id: i64,
        files: &[RenderedFile],
    ) -> Result<Vec<UploadResult>> {
        let mut results = Vec::with_capacity(files.len());
        for file in files {
            let result = self
                .upload_artifact(tenant_id, project_id, document_id, file)
                .await?;
            results.push(result);
        }
        Ok(results)
    }

    /// Generate a signed URL for downloading an artifact.
    #[instrument(skip(self), fields(bucket = %self.bucket))]
    pub async fn generate_signed_url(
        &self,
        gcs_path: &str,
        file_name: &str,
    ) -> Result<String> {
        let disposition = format!("attachment; filename=\"{}\"", file_name);

        let url = self
            .client
            .signed_url(
                &self.bucket,
                gcs_path,
                None,
                None,
                SignedURLOptions {
                    method: SignedURLMethod::GET,
                    expires: SIGNED_URL_EXPIRY,
                    query_parameters: Some(
                        vec![("response-content-disposition".to_string(), disposition)]
                            .into_iter()
                            .collect(),
                    ),
                    ..Default::default()
                },
            )
            .await
            .with_context(|| format!("Failed to generate signed URL for {}", gcs_path))?;

        Ok(url)
    }

    /// Delete a single object from GCS.
    #[instrument(skip(self), fields(bucket = %self.bucket))]
    pub async fn delete_object(&self, gcs_path: &str) -> Result<()> {
        self.client
            .delete_object(&DeleteObjectRequest {
                bucket: self.bucket.clone(),
                object: gcs_path.to_string(),
                ..Default::default()
            })
            .await
            .with_context(|| format!("Failed to delete GCS object {}", gcs_path))?;

        info!(gcs_path = %gcs_path, "Deleted GCS object");
        Ok(())
    }

    /// Delete all GCS objects for a document given their paths.
    pub async fn delete_objects(&self, gcs_paths: &[String]) -> Result<()> {
        for path in gcs_paths {
            if let Err(e) = self.delete_object(path).await {
                tracing::warn!(error = %e, path = %path, "Failed to delete GCS object, continuing");
            }
        }
        Ok(())
    }
}
