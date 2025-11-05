use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub valid: bool,
    pub tier: Option<String>,
    pub start_ts: Option<u64>,
    pub expiry_ts: Option<u64>,
    pub mint: Option<String>,
    pub reason: Option<String>,
}

impl LicenseStatus {
    pub fn valid(tier: String, start_ts: u64, expiry_ts: u64, mint: String) -> Self {
        Self {
            valid: true,
            tier: Some(tier),
            start_ts: Some(start_ts),
            expiry_ts: Some(expiry_ts),
            mint: Some(mint),
            reason: None,
        }
    }

    pub fn invalid(reason: &str) -> Self {
        Self {
            valid: false,
            tier: None,
            start_ts: None,
            expiry_ts: None,
            mint: None,
            reason: Some(reason.to_string()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataJson {
    pub name: String,
    pub description: Option<String>,
    pub image: Option<String>,
    pub attributes: Vec<MetadataAttribute>,
    pub properties: Option<MetadataProperties>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataProperties {
    pub creators: Option<Vec<MetadataCreator>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataCreator {
    pub address: String,
    pub share: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataAttribute {
    pub trait_type: String,
    pub value: String,
}

impl MetadataJson {
    pub fn get_attribute(&self, trait_type: &str) -> Option<String> {
        self.attributes
            .iter()
            .find(|a| a.trait_type == trait_type)
            .map(|a| a.value.clone())
    }

    pub fn is_screenerbot_license(&self) -> bool {
        self.name.contains("ScreenerBot") && self.name.contains("License")
    }

    pub fn get_issuer_address(&self) -> Option<String> {
        self.properties
            .as_ref()?
            .creators
            .as_ref()?
            .first()
            .map(|c| c.address.clone())
    }
}

// Simplified Metaplex metadata structure (we only need the URI)
#[derive(Debug)]
pub struct MetaplexMetadata {
    pub uri: String,
    pub creators: Vec<MetaplexCreator>,
}

#[derive(Debug)]
pub struct MetaplexCreator {
    pub address: Pubkey,
    pub verified: bool,
}
