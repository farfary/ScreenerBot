//! Metaplex Token Metadata fetching and parsing
//!
//! Derives the metadata PDA, fetches the account data, and deserializes it.
//! Also fetches the off-chain JSON metadata to get the image URL.

use crate::constants::METAPLEX_PROGRAM_ID;
use crate::logger::{self, LogTag};
use crate::rpc::{get_new_rpc_client, RpcClientMethods};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

// ============================================================================
// TYPES
// ============================================================================

/// NFT metadata fetched from chain and JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMetadata {
    pub mint: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub uri: Option<String>,
    pub image_url: Option<String>,
}

/// Errors that can occur during metadata fetching
#[derive(Debug)]
pub enum NftMetadataError {
    InvalidMint(String),
    PdaDerivationFailed(String),
    AccountNotFound(String),
    DeserializationFailed(String),
    JsonFetchFailed(String),
    RpcError(String),
}

impl std::fmt::Display for NftMetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMint(s) => write!(f, "Invalid mint: {}", s),
            Self::PdaDerivationFailed(s) => write!(f, "PDA derivation failed: {}", s),
            Self::AccountNotFound(s) => write!(f, "Account not found: {}", s),
            Self::DeserializationFailed(s) => write!(f, "Deserialization failed: {}", s),
            Self::JsonFetchFailed(s) => write!(f, "JSON fetch failed: {}", s),
            Self::RpcError(s) => write!(f, "RPC error: {}", s),
        }
    }
}

impl std::error::Error for NftMetadataError {}

// ============================================================================
// METAPLEX METADATA ACCOUNT STRUCTURE
// ============================================================================

/// Key discriminator for Metaplex accounts
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Key {
    Uninitialized = 0,
    EditionV1 = 1,
    MasterEditionV1 = 2,
    ReservationListV1 = 3,
    MetadataV1 = 4,
    ReservationListV2 = 5,
    MasterEditionV2 = 6,
    EditionMarker = 7,
    UseAuthorityRecord = 8,
    CollectionAuthorityRecord = 9,
    TokenOwnedEscrow = 10,
    TokenRecord = 11,
    MetadataDelegate = 12,
    EditionMarkerV2 = 13,
    HolderDelegate = 14,
}

/// On-chain metadata account data (simplified for reading)
#[derive(Debug, Clone)]
struct OnChainMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

/// Off-chain JSON metadata structure
#[derive(Debug, Clone, Deserialize)]
struct JsonMetadata {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub animation_url: Option<String>,
}

// ============================================================================
// CONSTANTS
// ============================================================================

const METADATA_PREFIX: &[u8] = b"metadata";
const MAX_NAME_LENGTH: usize = 32;
const MAX_SYMBOL_LENGTH: usize = 10;
const MAX_URI_LENGTH: usize = 200;

// ============================================================================
// PDA DERIVATION
// ============================================================================

/// Derives the metadata PDA for a given mint
fn derive_metadata_pda(mint: &Pubkey) -> Result<Pubkey, NftMetadataError> {
    let program_id = Pubkey::from_str(METAPLEX_PROGRAM_ID)
        .map_err(|e| NftMetadataError::PdaDerivationFailed(e.to_string()))?;

    let seeds = &[METADATA_PREFIX, program_id.as_ref(), mint.as_ref()];

    let (pda, _bump) = Pubkey::find_program_address(seeds, &program_id);
    Ok(pda)
}

// ============================================================================
// BORSH DESERIALIZATION (Manual - no mpl-token-metadata dependency)
// ============================================================================

/// Reads a u8 from the buffer
fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, NftMetadataError> {
    if *offset >= data.len() {
        return Err(NftMetadataError::DeserializationFailed(
            "Buffer underflow reading u8".to_string(),
        ));
    }
    let value = data[*offset];
    *offset += 1;
    Ok(value)
}

/// Reads a u32 (little-endian) from the buffer
fn read_u32_le(data: &[u8], offset: &mut usize) -> Result<u32, NftMetadataError> {
    if *offset + 4 > data.len() {
        return Err(NftMetadataError::DeserializationFailed(
            "Buffer underflow reading u32".to_string(),
        ));
    }
    let value = u32::from_le_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
    ]);
    *offset += 4;
    Ok(value)
}

/// Reads a Borsh string (length-prefixed)
fn read_string(data: &[u8], offset: &mut usize) -> Result<String, NftMetadataError> {
    let len = read_u32_le(data, offset)? as usize;
    if *offset + len > data.len() {
        return Err(NftMetadataError::DeserializationFailed(format!(
            "Buffer underflow reading string of len {}",
            len
        )));
    }
    let bytes = &data[*offset..*offset + len];
    *offset += len;

    // Metaplex pads strings with null bytes, trim them
    let s = String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string();
    Ok(s)
}

/// Skips n bytes in the buffer
fn skip_bytes(data: &[u8], offset: &mut usize, n: usize) -> Result<(), NftMetadataError> {
    if *offset + n > data.len() {
        return Err(NftMetadataError::DeserializationFailed(format!(
            "Buffer underflow skipping {} bytes",
            n
        )));
    }
    *offset += n;
    Ok(())
}

/// Deserializes the on-chain metadata account
fn deserialize_metadata(data: &[u8]) -> Result<OnChainMetadata, NftMetadataError> {
    let mut offset = 0;

    // Read key discriminator
    let key = read_u8(data, &mut offset)?;
    if key != Key::MetadataV1 as u8 {
        return Err(NftMetadataError::DeserializationFailed(format!(
            "Invalid metadata key: {}, expected {}",
            key,
            Key::MetadataV1 as u8
        )));
    }

    // Skip update_authority (32 bytes pubkey)
    skip_bytes(data, &mut offset, 32)?;

    // Skip mint (32 bytes pubkey)
    skip_bytes(data, &mut offset, 32)?;

    // Read Data struct
    let name = read_string(data, &mut offset)?;
    let symbol = read_string(data, &mut offset)?;
    let uri = read_string(data, &mut offset)?;

    // We don't need the rest (seller_fee_basis_points, creators, etc.)

    Ok(OnChainMetadata { name, symbol, uri })
}

// ============================================================================
// JSON METADATA FETCHING
// ============================================================================

/// Fetches the JSON metadata from the URI
async fn fetch_json_metadata(uri: &str) -> Result<JsonMetadata, NftMetadataError> {
    // Skip empty URIs
    if uri.is_empty() {
        return Err(NftMetadataError::JsonFetchFailed("Empty URI".to_string()));
    }

    // Handle IPFS URIs
    let url = if uri.starts_with("ipfs://") {
        format!("https://ipfs.io/ipfs/{}", &uri[7..])
    } else if uri.starts_with("ar://") {
        format!("https://arweave.net/{}", &uri[5..])
    } else {
        uri.to_string()
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| NftMetadataError::JsonFetchFailed(e.to_string()))?;

    let response = client
        .get(&url)
        .header("User-Agent", "ScreenerBot/1.0")
        .send()
        .await
        .map_err(|e| NftMetadataError::JsonFetchFailed(e.to_string()))?;

    if !response.status().is_success() {
        return Err(NftMetadataError::JsonFetchFailed(format!(
            "HTTP {} for {}",
            response.status(),
            url
        )));
    }

    let json: JsonMetadata = response
        .json()
        .await
        .map_err(|e| NftMetadataError::JsonFetchFailed(e.to_string()))?;

    Ok(json)
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Fetches NFT metadata for a single mint
pub async fn fetch_nft_metadata(mint: &str) -> Result<NftMetadata, NftMetadataError> {
    let mint_pubkey =
        Pubkey::from_str(mint).map_err(|e| NftMetadataError::InvalidMint(e.to_string()))?;

    // Derive metadata PDA
    let metadata_pda = derive_metadata_pda(&mint_pubkey)?;

    // Fetch account data from RPC
    let rpc_client = get_new_rpc_client();
    let account = rpc_client
        .get_account(&metadata_pda)
        .await
        .map_err(|e| NftMetadataError::RpcError(e.to_string()))?
        .ok_or_else(|| NftMetadataError::AccountNotFound(metadata_pda.to_string()))?;

    // Deserialize on-chain metadata
    let on_chain = deserialize_metadata(&account.data)?;

    // Fetch JSON metadata for image URL
    let image_url = if !on_chain.uri.is_empty() {
        match fetch_json_metadata(&on_chain.uri).await {
            Ok(json) => json.image,
            Err(e) => {
                logger::debug(
                    LogTag::Wallet,
                    &format!("Failed to fetch JSON metadata for {}: {}", mint, e),
                );
                None
            }
        }
    } else {
        None
    };

    Ok(NftMetadata {
        mint: mint.to_string(),
        name: if on_chain.name.is_empty() {
            None
        } else {
            Some(on_chain.name)
        },
        symbol: if on_chain.symbol.is_empty() {
            None
        } else {
            Some(on_chain.symbol)
        },
        uri: if on_chain.uri.is_empty() {
            None
        } else {
            Some(on_chain.uri)
        },
        image_url,
    })
}

/// Fetches NFT metadata for multiple mints in batch
/// Returns a map of mint -> metadata
pub async fn fetch_nft_metadata_batch(
    mints: &[String],
) -> HashMap<String, Result<NftMetadata, NftMetadataError>> {
    let mut results = HashMap::new();

    if mints.is_empty() {
        return results;
    }

    logger::debug(
        LogTag::Wallet,
        &format!("Fetching metadata for {} NFTs", mints.len()),
    );

    // Parse all mints and derive PDAs
    let mut pda_to_mint: HashMap<Pubkey, String> = HashMap::new();
    let mut failed_mints: Vec<(String, NftMetadataError)> = Vec::new();

    for mint in mints {
        match Pubkey::from_str(mint) {
            Ok(mint_pubkey) => match derive_metadata_pda(&mint_pubkey) {
                Ok(pda) => {
                    pda_to_mint.insert(pda, mint.clone());
                }
                Err(e) => {
                    failed_mints.push((mint.clone(), e));
                }
            },
            Err(e) => {
                failed_mints.push((mint.clone(), NftMetadataError::InvalidMint(e.to_string())));
            }
        }
    }

    // Add failed mints to results
    for (mint, err) in failed_mints {
        results.insert(mint, Err(err));
    }

    if pda_to_mint.is_empty() {
        return results;
    }

    // Batch fetch accounts (max 100 at a time for RPC limits)
    let rpc_client = get_new_rpc_client();
    let pdas: Vec<Pubkey> = pda_to_mint.keys().cloned().collect();

    for chunk in pdas.chunks(50) {
        match rpc_client.get_multiple_accounts(chunk).await {
            Ok(accounts) => {
                for (i, account_opt) in accounts.iter().enumerate() {
                    let pda = &chunk[i];
                    let mint = pda_to_mint.get(pda).unwrap().clone();

                    match account_opt {
                        Some(account) => {
                            // Deserialize on-chain metadata
                            match deserialize_metadata(&account.data) {
                                Ok(on_chain) => {
                                    // Fetch JSON metadata (async, but we'll do it inline for simplicity)
                                    let image_url = if !on_chain.uri.is_empty() {
                                        match fetch_json_metadata(&on_chain.uri).await {
                                            Ok(json) => json.image,
                                            Err(_) => None,
                                        }
                                    } else {
                                        None
                                    };

                                    results.insert(
                                        mint,
                                        Ok(NftMetadata {
                                            mint: pda_to_mint.get(pda).unwrap().clone(),
                                            name: if on_chain.name.is_empty() {
                                                None
                                            } else {
                                                Some(on_chain.name)
                                            },
                                            symbol: if on_chain.symbol.is_empty() {
                                                None
                                            } else {
                                                Some(on_chain.symbol)
                                            },
                                            uri: if on_chain.uri.is_empty() {
                                                None
                                            } else {
                                                Some(on_chain.uri)
                                            },
                                            image_url,
                                        }),
                                    );
                                }
                                Err(e) => {
                                    results.insert(mint, Err(e));
                                }
                            }
                        }
                        None => {
                            results.insert(
                                mint,
                                Err(NftMetadataError::AccountNotFound(
                                    "Metadata account not found".to_string(),
                                )),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                // Mark all mints in this chunk as failed
                for pda in chunk {
                    let mint = pda_to_mint.get(pda).unwrap().clone();
                    results.insert(mint, Err(NftMetadataError::RpcError(e.to_string())));
                }
            }
        }
    }

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Fetched metadata: {} success, {} failed",
            results.values().filter(|r| r.is_ok()).count(),
            results.values().filter(|r| r.is_err()).count()
        ),
    );

    results
}
