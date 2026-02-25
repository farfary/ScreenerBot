use serde::Serialize;

use crate::features::FeatureStatus;

/// Response for checking a specific feature
#[derive(Serialize)]
pub struct FeatureCheckResponse {
    pub id: String,
    pub status: FeatureStatus,
    pub available: bool,
    pub visible: bool,
}
