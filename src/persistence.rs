use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Row};
use uuid::Uuid;

// ============================================================
// Models
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GeneratedDocument {
    pub id: i64,
    pub tenant_id: Uuid,
    pub project_id: i64,
    pub template_id: Option<i64>,
    pub correlation_id: Option<Uuid>,
    pub title: String,
    pub document_type: String,
    pub status: String,
    pub requested_formats: Vec<String>,
    pub input_params: serde_json::Value,
    pub generation_metadata: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub requested_by: i64,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentArtifact {
    pub id: i64,
    pub tenant_id: Uuid,
    pub document_id: i64,
    pub format: String,
    pub file_name: String,
    pub gcs_path: String,
    pub file_size: i64,
    pub content_type: String,
    pub sha256_checksum: String,
    pub page_count: Option<i32>,
    pub rendering_duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentTemplate {
    pub id: i64,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub template_type: String,
    pub format: String,
    pub template_content: String,
    pub schema_version: String,
    pub is_system: bool,
    pub is_active: bool,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================
// Input structs
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentInput {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArtifactInput {
    pub tenant_id: Uuid,
    pub document_id: i64,
    pub format: String,
    pub file_name: String,
    pub gcs_path: String,
    pub file_size: i64,
    pub content_type: String,
    pub sha256_checksum: String,
    pub page_count: Option<i32>,
    pub rendering_duration_ms: Option<i32>,
}

// ============================================================
// Database client
// ============================================================

#[derive(Clone)]
pub struct DocumentDb {
    pool: PgPool,
}

impl DocumentDb {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Sets the tenant context for RLS on the current connection.
    async fn set_tenant_context(
        conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
        tenant_id: Uuid,
    ) -> Result<()> {
        sqlx::query("SELECT set_config('app.current_tenant', $1::text, true)")
            .bind(tenant_id.to_string())
            .execute(&mut **conn)
            .await
            .context("Failed to set tenant context")?;
        Ok(())
    }

    // --------------------------------------------------------
    // generated_documents CRUD
    // --------------------------------------------------------

    pub async fn create_document(&self, input: &CreateDocumentInput) -> Result<GeneratedDocument> {
        let mut conn = self.pool.acquire().await.context("Failed to acquire connection")?;
        Self::set_tenant_context(&mut conn, input.tenant_id).await?;

        let doc = sqlx::query_as::<_, GeneratedDocument>(
            r#"
            INSERT INTO storage.generated_documents (
                tenant_id, project_id, template_id, correlation_id,
                title, document_type, status, requested_formats,
                input_params, requested_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'queued', $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(input.tenant_id)
        .bind(input.project_id)
        .bind(input.template_id)
        .bind(input.correlation_id)
        .bind(&input.title)
        .bind(&input.document_type)
        .bind(&input.requested_formats)
        .bind(&input.input_params)
        .bind(input.requested_by)
        .fetch_one(&mut *conn)
        .await
        .context("Failed to insert generated_document")?;

        Ok(doc)
    }

    pub async fn get_document(&self, tenant_id: Uuid, id: i64) -> Result<Option<GeneratedDocument>> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, tenant_id).await?;

        let doc = sqlx::query_as::<_, GeneratedDocument>(
            "SELECT * FROM storage.generated_documents WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to fetch generated_document")?;

        Ok(doc)
    }

    pub async fn update_document_status(
        &self,
        tenant_id: Uuid,
        id: i64,
        status: &str,
        error_message: Option<&str>,
        generation_metadata: Option<&serde_json::Value>,
    ) -> Result<GeneratedDocument> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, tenant_id).await?;

        let now = Utc::now();
        let started_at = if status == "processing" { Some(now) } else { None };
        let completed_at = if status == "completed" || status == "failed" {
            Some(now)
        } else {
            None
        };

        let doc = sqlx::query_as::<_, GeneratedDocument>(
            r#"
            UPDATE storage.generated_documents
            SET status = $1,
                error_message = COALESCE($2, error_message),
                generation_metadata = COALESCE($3, generation_metadata),
                started_at = COALESCE($4, started_at),
                completed_at = COALESCE($5, completed_at)
            WHERE id = $6
            RETURNING *
            "#,
        )
        .bind(status)
        .bind(error_message)
        .bind(generation_metadata)
        .bind(started_at)
        .bind(completed_at)
        .bind(id)
        .fetch_one(&mut *conn)
        .await
        .context("Failed to update document status")?;

        Ok(doc)
    }

    pub async fn delete_document(&self, tenant_id: Uuid, id: i64) -> Result<bool> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, tenant_id).await?;

        let result = sqlx::query("DELETE FROM storage.generated_documents WHERE id = $1")
            .bind(id)
            .execute(&mut *conn)
            .await
            .context("Failed to delete generated_document")?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_documents(
        &self,
        tenant_id: Uuid,
        project_id: Option<i64>,
        document_type: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<GeneratedDocument>, i64)> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, tenant_id).await?;

        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM storage.generated_documents
            WHERE ($1::bigint IS NULL OR project_id = $1)
              AND ($2::text IS NULL OR document_type = $2)
              AND ($3::text IS NULL OR status = $3)
            "#,
        )
        .bind(project_id)
        .bind(document_type)
        .bind(status)
        .fetch_one(&mut *conn)
        .await
        .context("Failed to count documents")?;

        let docs = sqlx::query_as::<_, GeneratedDocument>(
            r#"
            SELECT * FROM storage.generated_documents
            WHERE ($1::bigint IS NULL OR project_id = $1)
              AND ($2::text IS NULL OR document_type = $2)
              AND ($3::text IS NULL OR status = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(project_id)
        .bind(document_type)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *conn)
        .await
        .context("Failed to list documents")?;

        Ok((docs, count))
    }

    // --------------------------------------------------------
    // generated_document_artifacts CRUD
    // --------------------------------------------------------

    pub async fn create_artifact(&self, input: &CreateArtifactInput) -> Result<DocumentArtifact> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, input.tenant_id).await?;

        let artifact = sqlx::query_as::<_, DocumentArtifact>(
            r#"
            INSERT INTO storage.generated_document_artifacts (
                tenant_id, document_id, format, file_name, gcs_path,
                file_size, content_type, sha256_checksum, page_count,
                rendering_duration_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(input.tenant_id)
        .bind(input.document_id)
        .bind(&input.format)
        .bind(&input.file_name)
        .bind(&input.gcs_path)
        .bind(input.file_size)
        .bind(&input.content_type)
        .bind(&input.sha256_checksum)
        .bind(input.page_count)
        .bind(input.rendering_duration_ms)
        .fetch_one(&mut *conn)
        .await
        .context("Failed to insert document artifact")?;

        Ok(artifact)
    }

    pub async fn list_artifacts(
        &self,
        tenant_id: Uuid,
        document_id: i64,
    ) -> Result<Vec<DocumentArtifact>> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, tenant_id).await?;

        let artifacts = sqlx::query_as::<_, DocumentArtifact>(
            "SELECT * FROM storage.generated_document_artifacts WHERE document_id = $1 ORDER BY format",
        )
        .bind(document_id)
        .fetch_all(&mut *conn)
        .await
        .context("Failed to list artifacts")?;

        Ok(artifacts)
    }

    pub async fn get_artifact(
        &self,
        tenant_id: Uuid,
        artifact_id: i64,
    ) -> Result<Option<DocumentArtifact>> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, tenant_id).await?;

        let artifact = sqlx::query_as::<_, DocumentArtifact>(
            "SELECT * FROM storage.generated_document_artifacts WHERE id = $1",
        )
        .bind(artifact_id)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to fetch artifact")?;

        Ok(artifact)
    }

    pub async fn delete_artifacts_for_document(
        &self,
        tenant_id: Uuid,
        document_id: i64,
    ) -> Result<Vec<String>> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, tenant_id).await?;

        let paths: Vec<String> = sqlx::query_scalar(
            "DELETE FROM storage.generated_document_artifacts WHERE document_id = $1 RETURNING gcs_path",
        )
        .bind(document_id)
        .fetch_all(&mut *conn)
        .await
        .context("Failed to delete artifacts")?;

        Ok(paths)
    }

    // --------------------------------------------------------
    // document_templates
    // --------------------------------------------------------

    pub async fn get_template(
        &self,
        tenant_id: Uuid,
        template_id: i64,
    ) -> Result<Option<DocumentTemplate>> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, tenant_id).await?;

        let tpl = sqlx::query_as::<_, DocumentTemplate>(
            "SELECT * FROM storage.document_templates WHERE id = $1 AND is_active = true",
        )
        .bind(template_id)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to fetch template")?;

        Ok(tpl)
    }

    pub async fn get_template_by_type(
        &self,
        tenant_id: Uuid,
        template_type: &str,
        format: &str,
    ) -> Result<Option<DocumentTemplate>> {
        let mut conn = self.pool.acquire().await?;
        Self::set_tenant_context(&mut conn, tenant_id).await?;

        let tpl = sqlx::query_as::<_, DocumentTemplate>(
            r#"
            SELECT * FROM storage.document_templates
            WHERE template_type = $1 AND format = $2 AND is_active = true
            ORDER BY is_system ASC, updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(template_type)
        .bind(format)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to fetch template by type")?;

        Ok(tpl)
    }
}
