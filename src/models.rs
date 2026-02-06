// document-generation-service/src/models.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentFormat {
    PDF,
    Markdown,
    HTML,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecificationType {
    // Legacy IEEE 830
    #[serde(rename = "ieee830_drd")]
    IEEE830DRD,
    #[serde(rename = "ieee830_srs")]
    IEEE830SRS,

    // Legacy MIL-STD-498
    #[serde(rename = "milstd498_srs")]
    MilStd498SRS,

    // ISO/IEC/IEEE 29148:2018 (Modern standards)
    #[serde(rename = "iso29148_stakeholder_requirements")]
    ISO29148StakeholderRequirements,
    #[serde(rename = "iso29148_system_requirements")]
    ISO29148SystemRequirements,
    #[serde(rename = "iso29148_software_requirements")]
    ISO29148SoftwareRequirements,
    #[serde(rename = "iso29148_concept_of_operations")]
    ISO29148ConceptOfOperations,

    // Reports
    #[serde(rename = "security_scan_report")]
    SecurityScanReport,
    #[serde(rename = "compliance_audit_report")]
    ComplianceAuditReport,
    #[serde(rename = "test_execution_report")]
    TestExecutionReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentGenerationRequest {
    pub specification_type: SpecificationType,
    pub output_formats: Vec<DocumentFormat>,
    pub data: serde_json::Value,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: String,
    pub project_name: String,
    pub version: String,
    pub author: String,
    pub organization: String,
    pub classification: Option<String>,
    pub distribution_statement: Option<String>,
    #[serde(default = "Utc::now")]
    pub generated_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedDocument {
    pub format: DocumentFormat,
    pub content_base64: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentGenerationResponse {
    pub request_id: String,
    pub status: String,
    pub documents: Vec<GeneratedDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub generated_at: DateTime<Utc>,
}

impl DocumentGenerationResponse {
    pub fn success(request_id: String, documents: Vec<GeneratedDocument>) -> Self {
        Self {
            request_id,
            status: "success".to_string(),
            documents,
            error: None,
            generated_at: Utc::now(),
        }
    }

    pub fn error(request_id: String, error: String) -> Self {
        Self {
            request_id,
            status: "error".to_string(),
            documents: vec![],
            error: Some(error),
            generated_at: Utc::now(),
        }
    }
}
