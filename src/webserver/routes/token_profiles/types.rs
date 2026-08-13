use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedTokenProfile {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    #[serde(default)]
    pub links: HashMap<String, String>,
    pub published_at: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct WebsiteProfileResponse {
    #[serde(default)]
    pub profiles: Vec<PublishedTokenProfile>,
}
