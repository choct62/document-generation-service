// document-generation-service/src/generators/mod.rs

mod ieee830;
mod iso29148_conops;
mod iso29148_srs;
mod iso29148_stakrs;
mod iso29148_syrs;
mod security_report;

use crate::error::Result;
use crate::models::{DocumentMetadata, SpecificationType};
use async_trait::async_trait;
use serde_json::Value;

pub use ieee830::IEEE830Generator;
pub use iso29148_conops::ISO29148ConOpsGenerator;
pub use iso29148_srs::ISO29148SRSGenerator;
pub use iso29148_stakrs::ISO29148StakRSGenerator;
pub use iso29148_syrs::ISO29148SyRSGenerator;
pub use security_report::SecurityReportGenerator;

#[async_trait]
pub trait Generator: Send + Sync {
    async fn generate(&self, data: &Value, metadata: &DocumentMetadata) -> Result<String>;
}

pub fn create_generator(spec_type: &SpecificationType) -> Result<Box<dyn Generator>> {
    match spec_type {
        SpecificationType::IEEE830DRD | SpecificationType::IEEE830SRS => {
            Ok(Box::new(IEEE830Generator::new()))
        }
        SpecificationType::ISO29148StakeholderRequirements => {
            Ok(Box::new(ISO29148StakRSGenerator::new()))
        }
        SpecificationType::ISO29148SystemRequirements => {
            Ok(Box::new(ISO29148SyRSGenerator::new()))
        }
        SpecificationType::ISO29148SoftwareRequirements => {
            Ok(Box::new(ISO29148SRSGenerator::new()))
        }
        SpecificationType::ISO29148ConceptOfOperations => {
            Ok(Box::new(ISO29148ConOpsGenerator::new()))
        }
        SpecificationType::SecurityScanReport => Ok(Box::new(SecurityReportGenerator::new())),
        _ => Err(crate::error::DocumentError::InvalidSpecificationType(
            format!("{:?}", spec_type),
        )),
    }
}
