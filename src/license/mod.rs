mod cache;
mod types;

use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use types::{LicenseStatus, MetadataJson};
use types::{MetaplexCreator, MetaplexMetadata};

use crate::constants::{LICENSE_ISSUER_PUBKEY, METAPLEX_PROGRAM_ID};
use crate::logger::{self, LogTag};
use crate::rpc;
use crate::utils::parse_pubkey_safe;

// Global cache instance
static LICENSE_CACHE: Lazy<cache::LicenseCache> = Lazy::new(cache::LicenseCache::new);

const HTTP_TIMEOUT_SECS: u64 = 3;

/// Verify license for a wallet address
pub async fn verify_license_for_wallet(wallet: &Pubkey) -> Result<LicenseStatus, String> {
    let wallet_str = wallet.to_string();

    // 1. Check cache first
    if let Some(cached) = LICENSE_CACHE.get(&wallet_str) {
        logger::debug(
            LogTag::License,
            &format!("License cache hit for wallet={}", wallet_str),
        );
        return Ok(cached);
    }

    logger::debug(
        LogTag::License,
        &format!("Verifying license for wallet={}", wallet_str),
    );

    // 2. Get all token accounts (filter for NFTs: decimals=0, amount=1)
    let token_accounts = match get_nft_token_accounts(wallet).await {
        Ok(accounts) => accounts,
        Err(e) => {
            let status = LicenseStatus::invalid(&format!("Failed to get token accounts: {}", e));
            LICENSE_CACHE.set(&wallet_str, status.clone());
            return Ok(status);
        }
    };

    logger::info(
        LogTag::License,
        &format!(
            "Found {} NFT candidates for wallet={}",
            token_accounts.len(),
            wallet_str
        ),
    );

    // 3. Check each NFT for ScreenerBot license
    for mint in token_accounts {
        match check_license_nft(&mint).await {
            Ok(Some(status)) => {
                logger::info(
                    LogTag::License,
                    &format!(
                        "Valid license found: wallet={}, mint={}, tier={}, expiry={}",
                        wallet_str,
                        status.mint.as_ref().unwrap(),
                        status.tier.as_ref().unwrap(),
                        status.expiry_ts.unwrap()
                    ),
                );
                LICENSE_CACHE.set(&wallet_str, status.clone());
                return Ok(status);
            }
            Ok(None) => continue, // Not a ScreenerBot license, check next
            Err(e) => {
                logger::debug(
                    LogTag::License,
                    &format!("Error checking mint {}: {}", mint, e),
                );
                continue;
            }
        }
    }

    // No valid license found
    let status = LicenseStatus::invalid("No valid ScreenerBot license found");
    logger::warning(
        LogTag::License,
        &format!("No valid license: wallet={}", wallet_str),
    );
    LICENSE_CACHE.set(&wallet_str, status.clone());
    Ok(status)
}

/// Get cached license status without blocking on RPC calls
/// Returns (status, is_fresh) if cached, or None if no cache exists
/// Use this for dashboard APIs that should never block on license verification
pub fn get_cached_license(wallet: &Pubkey) -> Option<(LicenseStatus, bool)> {
    let wallet_str = wallet.to_string();
    LICENSE_CACHE.get_cached_or_stale(&wallet_str)
}

/// Check if license cache needs refresh for a wallet
pub fn license_needs_refresh(wallet: &Pubkey) -> bool {
    let wallet_str = wallet.to_string();
    LICENSE_CACHE.needs_refresh(&wallet_str)
}

/// Spawn a background task to refresh license if stale
/// This is fire-and-forget - does not block the caller
pub fn spawn_license_refresh_if_needed(wallet: Pubkey) {
    let wallet_str = wallet.to_string();
    if LICENSE_CACHE.needs_refresh(&wallet_str) {
        tokio::spawn(async move {
            logger::debug(
                LogTag::License,
                &format!("Background license refresh for wallet={}", wallet_str),
            );
            let _ = verify_license_for_wallet(&wallet).await;
        });
    }
}

/// Verify license for a wallet address using custom RPC endpoints (for pre-initialization)
/// This function creates an ephemeral RPC client and does NOT use the global RPC client
/// Does NOT cache the result (use this only during initialization validation)
pub async fn verify_license_for_wallet_with_endpoints(
    wallet: &Pubkey,
    rpc_urls: &[String],
) -> Result<LicenseStatus, String> {
    let wallet_str = wallet.to_string();

    if rpc_urls.is_empty() {
        return Ok(LicenseStatus::invalid("No RPC URLs provided"));
    }

    logger::info(
        LogTag::License,
        &format!(
            "Verifying license for wallet={} with {} custom RPC endpoint(s)",
            wallet_str,
            rpc_urls.len()
        ),
    );

    // Create ephemeral RPC client (does not use global state)
    let ephemeral_client = rpc::RpcClient::new_with_urls(rpc_urls.to_vec())
        .map_err(|e| format!("Failed to create ephemeral RPC client: {}", e))?;

    // Get all token accounts (filter for NFTs: decimals=0, amount=1)
    let token_accounts = match get_nft_token_accounts_with_client(&ephemeral_client, wallet).await {
        Ok(accounts) => accounts,
        Err(e) => {
            return Ok(LicenseStatus::invalid(&format!(
                "Failed to get token accounts: {}",
                e
            )));
        }
    };

    logger::info(
        LogTag::License,
        &format!(
            "Found {} NFT candidates for wallet={}",
            token_accounts.len(),
            wallet_str
        ),
    );

    // Check each NFT for ScreenerBot license
    for mint in token_accounts {
        match check_license_nft_with_client(&ephemeral_client, &mint).await {
            Ok(Some(status)) => {
                logger::info(
                    LogTag::License,
                    &format!(
                        "Valid license found: wallet={}, mint={}, tier={}, expiry={}",
                        wallet_str,
                        status.mint.as_ref().unwrap(),
                        status.tier.as_ref().unwrap(),
                        status.expiry_ts.unwrap()
                    ),
                );
                return Ok(status);
            }
            Ok(None) => continue, // Not a ScreenerBot license, check next
            Err(e) => {
                logger::debug(
                    LogTag::License,
                    &format!("Error checking mint {}: {}", mint, e),
                );
                continue;
            }
        }
    }

    // No valid license found
    let status = LicenseStatus::invalid("No valid ScreenerBot license found");
    logger::warning(
        LogTag::License,
        &format!("No valid license: wallet={}", wallet_str),
    );
    Ok(status)
}

/// Get all NFT token accounts (decimals=0, amount=1) for a wallet
async fn get_nft_token_accounts(wallet: &Pubkey) -> Result<Vec<Pubkey>, String> {
    let wallet_str = wallet.to_string();

    // Get NFT mints using the optimized RPC method (filters decimals=0, amount=1 server-side)
    let rpc_client = rpc::get_rpc_client();
    let nft_mint_strings = rpc_client
        .get_nft_mints_for_wallet(&wallet_str)
        .await
        .map_err(|e| format!("Failed to get NFT mints: {}", e))?;

    // Convert strings to Pubkeys
    let mut nft_mints = Vec::new();
    for mint_str in nft_mint_strings {
        if let Ok(mint) = Pubkey::from_str(&mint_str) {
            nft_mints.push(mint);
        }
    }

    Ok(nft_mints)
}

/// Get all NFT token accounts using a custom RPC client (for pre-initialization)
async fn get_nft_token_accounts_with_client(
    client: &rpc::RpcClient,
    wallet: &Pubkey,
) -> Result<Vec<Pubkey>, String> {
    let wallet_str = wallet.to_string();

    // Get NFT mints using the optimized RPC method (filters decimals=0, amount=1 server-side)
    let nft_mint_strings = client
        .get_nft_mints_for_wallet(&wallet_str)
        .await
        .map_err(|e| format!("Failed to get NFT mints: {}", e))?;

    // Convert strings to Pubkeys
    let mut nft_mints = Vec::new();
    for mint_str in nft_mint_strings {
        if let Ok(mint) = Pubkey::from_str(&mint_str) {
            nft_mints.push(mint);
        }
    }

    Ok(nft_mints)
}

/// Check if an NFT is a valid ScreenerBot license
async fn check_license_nft(mint: &Pubkey) -> Result<Option<LicenseStatus>, String> {
    logger::info(LogTag::License, &format!("Checking NFT: mint={}", mint));

    // 1. Derive Metaplex metadata PDA
    let metadata_pda = derive_metaplex_metadata_pda(mint)?;

    logger::info(LogTag::License, &format!("Metadata PDA: {}", metadata_pda));

    // 2. Fetch metadata account
    let rpc_client = rpc::get_rpc_client();
    let metadata_account = rpc_client.get_account(&metadata_pda).await.map_err(|e| {
        logger::warning(
            LogTag::License,
            &format!("Failed to get metadata account for mint={}: {}", mint, e),
        );
        format!("Failed to get metadata account: {}", e)
    })?;

    logger::info(
        LogTag::License,
        &format!(
            "Fetched metadata account: {} bytes",
            metadata_account.data.len()
        ),
    );

    // 3. Parse Metaplex metadata (simplified - just extract URI and creators)
    let metadata = parse_metaplex_metadata(&metadata_account.data)?;

    logger::info(
        LogTag::License,
        &format!(
            "Parsed metadata: URI={}, creators={}",
            metadata.uri,
            metadata.creators.len()
        ),
    );

    // 4. Verify creator matches LICENSE_ISSUER_PUBKEY
    let issuer_pubkey = parse_pubkey_safe(LICENSE_ISSUER_PUBKEY)?;
    let creator_valid = metadata
        .creators
        .iter()
        .any(|c| c.address == issuer_pubkey && c.verified);

    if !creator_valid {
        logger::info(
            LogTag::License,
            &format!("Creator not valid for mint={}", mint),
        );
        return Ok(None); // Not our NFT
    }

    logger::info(
        LogTag::License,
        &format!("Creator verified for mint={}", mint),
    );

    // 5. Fetch metadata JSON from URI
    let metadata_json = match fetch_metadata_json(&metadata.uri) {
        Ok(json) => json,
        Err(e) => {
            logger::warning(
                LogTag::License,
                &format!("Failed to fetch metadata JSON for mint={}: {}", mint, e),
            );
            return Err(e);
        }
    };

    logger::info(
        LogTag::License,
        &format!("Fetched metadata JSON for mint={}", mint),
    );

    // 6. Check if it's a ScreenerBot license
    if !metadata_json.is_screenerbot_license() {
        logger::info(
            LogTag::License,
            &format!("Not a ScreenerBot license: mint={}", mint),
        );
        return Ok(None);
    }

    // 7. Parse and validate attributes
    let tier = metadata_json
        .get_attribute("Tier")
        .ok_or("Missing Tier attribute")?;

    // Parse dates from "Start Date" and "Expiry Date" attributes (YYYY-MM-DD format)
    let start_date = metadata_json
        .get_attribute("Start Date")
        .ok_or("Missing Start Date attribute")?;
    let expiry_date = metadata_json
        .get_attribute("Expiry Date")
        .ok_or("Missing Expiry Date attribute")?;

    // Convert dates to Unix timestamps (assume midnight UTC)
    let start_ts =
        parse_date_to_timestamp(&start_date).map_err(|e| format!("Invalid Start Date: {}", e))?;
    let expiry_ts =
        parse_date_to_timestamp(&expiry_date).map_err(|e| format!("Invalid Expiry Date: {}", e))?;

    // Check if revoked (optional field, defaults to false)
    let revoked = metadata_json
        .get_attribute("Revoked")
        .unwrap_or_else(|| "false".to_string())
        == "true";

    // 8. Verify issuer from properties.creators matches LICENSE_ISSUER_PUBKEY
    let json_issuer = metadata_json
        .get_issuer_address()
        .ok_or("Missing issuer in metadata properties.creators")?;

    if json_issuer != LICENSE_ISSUER_PUBKEY {
        logger::warning(
            LogTag::License,
            &format!(
                "Issuer mismatch in JSON: mint={}, json_issuer={}, expected={}",
                mint, json_issuer, LICENSE_ISSUER_PUBKEY
            ),
        );
        return Ok(None);
    }

    logger::info(
        LogTag::License,
        &format!("Issuer verified: mint={}, issuer={}", mint, json_issuer),
    );

    logger::info(
        LogTag::License,
        &format!(
            "Parsed license: mint={}, tier={}, start={}, expiry={}, revoked={}",
            mint, tier, start_ts, expiry_ts, revoked
        ),
    );

    // 9. Check revoked
    if revoked {
        logger::warning(LogTag::License, &format!("License revoked: mint={}", mint));
        return Ok(None);
    }

    // 10. Check expiry
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now < start_ts {
        logger::warning(
            LogTag::License,
            &format!(
                "License not yet valid: mint={}, start_ts={}, now={}",
                mint, start_ts, now
            ),
        );
        return Ok(None);
    }

    if now > expiry_ts {
        logger::warning(
            LogTag::License,
            &format!(
                "License expired: mint={}, expiry_ts={}, now={}",
                mint, expiry_ts, now
            ),
        );
        return Ok(None);
    }

    // Valid license!
    Ok(Some(LicenseStatus::valid(
        tier,
        start_ts,
        expiry_ts,
        mint.to_string(),
    )))
}

/// Check if an NFT is a valid ScreenerBot license using a custom RPC client (for pre-initialization)
async fn check_license_nft_with_client(
    client: &rpc::RpcClient,
    mint: &Pubkey,
) -> Result<Option<LicenseStatus>, String> {
    logger::info(LogTag::License, &format!("Checking NFT: mint={}", mint));

    // 1. Derive Metaplex metadata PDA
    let metadata_pda = derive_metaplex_metadata_pda(mint)?;

    logger::info(LogTag::License, &format!("Metadata PDA: {}", metadata_pda));

    // 2. Fetch metadata account
    let metadata_account = client.get_account(&metadata_pda).await.map_err(|e| {
        logger::warning(
            LogTag::License,
            &format!("Failed to get metadata account for mint={}: {}", mint, e),
        );
        format!("Failed to get metadata account: {}", e)
    })?;

    logger::info(
        LogTag::License,
        &format!(
            "Fetched metadata account: {} bytes",
            metadata_account.data.len()
        ),
    );

    // 3. Parse Metaplex metadata (simplified - just extract URI and creators)
    let metadata = parse_metaplex_metadata(&metadata_account.data)?;

    logger::info(
        LogTag::License,
        &format!(
            "Parsed metadata: URI={}, creators={}",
            metadata.uri,
            metadata.creators.len()
        ),
    );

    // 4. Verify creator matches LICENSE_ISSUER_PUBKEY
    let issuer_pubkey = parse_pubkey_safe(LICENSE_ISSUER_PUBKEY)?;
    let creator_valid = metadata
        .creators
        .iter()
        .any(|c| c.address == issuer_pubkey && c.verified);

    if !creator_valid {
        logger::info(
            LogTag::License,
            &format!("Creator not valid for mint={}", mint),
        );
        return Ok(None); // Not our NFT
    }

    logger::info(
        LogTag::License,
        &format!("Creator verified for mint={}", mint),
    );

    // 5. Fetch metadata JSON from URI
    let metadata_json = match fetch_metadata_json(&metadata.uri) {
        Ok(json) => json,
        Err(e) => {
            logger::warning(
                LogTag::License,
                &format!("Failed to fetch metadata JSON for mint={}: {}", mint, e),
            );
            return Err(e);
        }
    };

    logger::info(
        LogTag::License,
        &format!("Fetched metadata JSON for mint={}", mint),
    );

    // 6. Check if it's a ScreenerBot license
    if !metadata_json.is_screenerbot_license() {
        logger::info(
            LogTag::License,
            &format!("Not a ScreenerBot license: mint={}", mint),
        );
        return Ok(None);
    }

    // 7. Parse and validate attributes
    let tier = metadata_json
        .get_attribute("Tier")
        .ok_or("Missing Tier attribute")?;

    // Parse dates from "Start Date" and "Expiry Date" attributes (YYYY-MM-DD format)
    let start_date = metadata_json
        .get_attribute("Start Date")
        .ok_or("Missing Start Date attribute")?;
    let expiry_date = metadata_json
        .get_attribute("Expiry Date")
        .ok_or("Missing Expiry Date attribute")?;

    // Convert dates to Unix timestamps (assume midnight UTC)
    let start_ts =
        parse_date_to_timestamp(&start_date).map_err(|e| format!("Invalid Start Date: {}", e))?;
    let expiry_ts =
        parse_date_to_timestamp(&expiry_date).map_err(|e| format!("Invalid Expiry Date: {}", e))?;

    // Check if revoked (optional field, defaults to false)
    let revoked = metadata_json
        .get_attribute("Revoked")
        .unwrap_or_else(|| "false".to_string())
        == "true";

    // 8. Verify issuer from properties.creators matches LICENSE_ISSUER_PUBKEY
    let json_issuer = metadata_json
        .get_issuer_address()
        .ok_or("Missing issuer in metadata properties.creators")?;

    if json_issuer != LICENSE_ISSUER_PUBKEY {
        logger::warning(
            LogTag::License,
            &format!(
                "Issuer mismatch in JSON: mint={}, json_issuer={}, expected={}",
                mint, json_issuer, LICENSE_ISSUER_PUBKEY
            ),
        );
        return Ok(None);
    }

    logger::info(
        LogTag::License,
        &format!("Issuer verified: mint={}, issuer={}", mint, json_issuer),
    );

    logger::info(
        LogTag::License,
        &format!(
            "Parsed license: mint={}, tier={}, start={}, expiry={}, revoked={}",
            mint, tier, start_ts, expiry_ts, revoked
        ),
    );

    // 9. Check revoked
    if revoked {
        logger::warning(LogTag::License, &format!("License revoked: mint={}", mint));
        return Ok(None);
    }

    // 10. Check expiry
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now < start_ts {
        logger::warning(
            LogTag::License,
            &format!(
                "License not yet valid: mint={}, start_ts={}, now={}",
                mint, start_ts, now
            ),
        );
        return Ok(None);
    }

    if now > expiry_ts {
        logger::warning(
            LogTag::License,
            &format!(
                "License expired: mint={}, expiry_ts={}, now={}",
                mint, expiry_ts, now
            ),
        );
        return Ok(None);
    }

    // Valid license!
    Ok(Some(LicenseStatus::valid(
        tier,
        start_ts,
        expiry_ts,
        mint.to_string(),
    )))
}

/// Derive Metaplex metadata PDA from mint address
fn derive_metaplex_metadata_pda(mint: &Pubkey) -> Result<Pubkey, String> {
    let metaplex_program = parse_pubkey_safe(METAPLEX_PROGRAM_ID)?;

    let (pda, _bump) = Pubkey::find_program_address(
        &[b"metadata", metaplex_program.as_ref(), mint.as_ref()],
        &metaplex_program,
    );

    Ok(pda)
}

/// Parse Metaplex metadata account (simplified - just get URI and creators)
fn parse_metaplex_metadata(data: &[u8]) -> Result<MetaplexMetadata, String> {
    // Metaplex metadata uses Borsh serialization
    // For simplicity, we'll do manual parsing of the key fields we need

    if data.len() < 100 {
        return Err("Metadata account too small".to_string());
    }

    // Skip key (1 byte) + update_authority (32 bytes) + mint (32 bytes) = 65 bytes
    let mut offset = 65;

    // Read name (string with 4-byte length prefix)
    if offset + 4 > data.len() {
        return Err("Invalid metadata: cannot read name length".to_string());
    }
    let name_len = u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]) as usize;
    offset += 4 + name_len;

    // Read symbol (string with 4-byte length prefix)
    if offset + 4 > data.len() {
        return Err("Invalid metadata: cannot read symbol length".to_string());
    }
    let symbol_len = u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]) as usize;
    offset += 4 + symbol_len;

    // Read URI (string with 4-byte length prefix)
    if offset + 4 > data.len() {
        return Err("Invalid metadata: cannot read URI length".to_string());
    }
    let uri_len = u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]) as usize;
    offset += 4;

    if offset + uri_len > data.len() {
        return Err("Invalid metadata: URI extends beyond data".to_string());
    }

    let uri = String::from_utf8(data[offset..offset + uri_len].to_vec())
        .map_err(|e| format!("Invalid UTF-8 in URI: {}", e))?
        .trim_end_matches('\0')
        .to_string();
    offset += uri_len;

    // Skip seller_fee_basis_points (2 bytes)
    offset += 2;

    // Read creators option (1 byte for Some/None)
    if offset >= data.len() {
        return Err("Invalid metadata: cannot read creators option".to_string());
    }

    let has_creators = data[offset] == 1;
    offset += 1;

    let mut creators = Vec::new();
    if has_creators {
        // Read creators array length (4 bytes)
        if offset + 4 > data.len() {
            return Err("Invalid metadata: cannot read creators length".to_string());
        }
        let creators_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        // Each creator is 32 bytes (address) + 1 byte (verified) + 1 byte (share)
        for _ in 0..creators_len {
            if offset + 34 > data.len() {
                break; // Not enough data, stop parsing
            }

            let address_bytes: [u8; 32] = match data[offset..offset + 32].try_into() {
                Ok(bytes) => bytes,
                Err(_) => break, // Invalid slice length, stop parsing
            };
            let address = Pubkey::new_from_array(address_bytes);
            let verified = data[offset + 32] == 1;

            creators.push(MetaplexCreator { address, verified });
            offset += 34; // 32 (address) + 1 (verified) + 1 (share)
        }
    }

    Ok(MetaplexMetadata { uri, creators })
}

/// Fetch metadata JSON from URI (IPFS/Arweave)
fn fetch_metadata_json(uri: &str) -> Result<MetadataJson, String> {
    logger::info(LogTag::License, &format!("Fetching metadata from: {}", uri));

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(uri)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let metadata: MetadataJson = response
        .json()
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(metadata)
}

/// Parse date string (YYYY-MM-DD) to Unix timestamp (midnight UTC)
fn parse_date_to_timestamp(date_str: &str) -> Result<u64, String> {
    // Parse YYYY-MM-DD format
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid date format: {}", date_str));
    }

    let year = parts[0]
        .parse::<i32>()
        .map_err(|e| format!("Invalid year: {}", e))?;
    let month = parts[1]
        .parse::<u32>()
        .map_err(|e| format!("Invalid month: {}", e))?;
    let day = parts[2]
        .parse::<u32>()
        .map_err(|e| format!("Invalid day: {}", e))?;

    // Validate ranges
    if month < 1 || month > 12 {
        return Err(format!("Month out of range: {}", month));
    }
    if day < 1 || day > 31 {
        return Err(format!("Day out of range: {}", day));
    }

    // Calculate days since Unix epoch (1970-01-01)
    let mut days = 0i64;

    // Add days for complete years
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Add days for complete months in the target year
    let days_in_month = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for m in 1..month {
        days += days_in_month[(m - 1) as usize] as i64;
    }

    // Add remaining days
    days += (day - 1) as i64;

    // Convert to seconds
    let timestamp = days * 86400;

    if timestamp < 0 {
        return Err(format!("Date before Unix epoch: {}", date_str));
    }

    Ok(timestamp as u64)
}

/// Check if a year is a leap year
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Invalidate cache for a wallet (useful when license is transferred)
pub fn invalidate_license_cache(wallet: &str) {
    LICENSE_CACHE.invalidate(wallet);
}

/// Clear entire license cache
pub fn clear_license_cache() {
    LICENSE_CACHE.clear();
}
