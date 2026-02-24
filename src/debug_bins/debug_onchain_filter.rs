//! Debug binary — tests on-chain token filtering rules against live mint data.

use clap::Parser;
use screenerbot::constants::METAPLEX_PROGRAM_ID;
use screenerbot::rpc::{get_rpc_client, init_rpc_client, RpcClientMethods};
use screenerbot::tokens::database::{init_global_database, TokenDatabase};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

/// Debug binary for on-chain token metadata extraction and scam detection.
///
/// Tests the proposed on-chain core filtering system by fetching Metaplex
/// metadata + SPL Token mint data directly from the Solana blockchain and
/// running scam detection heuristics.
///
/// Usage:
///   cargo build -p screenerbot-debug-tools --bin debug_onchain_filter
///   ./target/debug/debug_onchain_filter --mint <ADDRESS>
///   ./target/debug/debug_onchain_filter --scan-scam-symbols --limit 20
#[derive(Parser, Debug)]
#[command(name = "debug_onchain_filter", about = "On-chain metadata extraction + scam detection")]
struct Args {
    /// Inspect a single mint address
    #[arg(long)]
    mint: Option<String>,

    /// Scan tokens with suspicious symbols from the local database
    #[arg(long, default_value_t = false)]
    scan_scam_symbols: bool,

    /// Max tokens to scan in batch mode
    #[arg(long, default_value_t = 10)]
    limit: usize,

    /// Custom symbol pattern to search for (e.g. "00")
    #[arg(long)]
    symbol: Option<String>,

    /// Show raw account bytes for debugging
    #[arg(long, default_value_t = false)]
    raw: bool,
}

// ============================================================================
// ENHANCED METAPLEX METADATA PARSING
// ============================================================================

/// Extended on-chain metadata — includes update_authority and is_mutable
/// that the standard nfts/metadata.rs parser skips.
#[derive(Debug, Clone)]
struct ExtendedOnChainMetadata {
    pub update_authority: Pubkey,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub is_mutable: bool,
}

/// Reads a u8 from the buffer
fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, String> {
    if *offset >= data.len() {
        return Err(format!("Buffer underflow reading u8 at offset {}", offset));
    }
    let value = data[*offset];
    *offset += 1;
    Ok(value)
}

/// Reads a u16 (little-endian) from the buffer
fn read_u16_le(data: &[u8], offset: &mut usize) -> Result<u16, String> {
    if *offset + 2 > data.len() {
        return Err(format!("Buffer underflow reading u16 at offset {}", offset));
    }
    let value = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    Ok(value)
}

/// Reads a u32 (little-endian) from the buffer
fn read_u32_le(data: &[u8], offset: &mut usize) -> Result<u32, String> {
    if *offset + 4 > data.len() {
        return Err(format!("Buffer underflow reading u32 at offset {}", offset));
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

/// Reads a Pubkey (32 bytes) from the buffer
fn read_pubkey(data: &[u8], offset: &mut usize) -> Result<Pubkey, String> {
    if *offset + 32 > data.len() {
        return Err(format!(
            "Buffer underflow reading pubkey at offset {}",
            offset
        ));
    }
    let bytes: [u8; 32] = data[*offset..*offset + 32]
        .try_into()
        .map_err(|_| "Failed to convert pubkey bytes")?;
    *offset += 32;
    Ok(Pubkey::new_from_array(bytes))
}

/// Reads a Borsh string (length-prefixed, null-trimmed)
fn read_string(data: &[u8], offset: &mut usize) -> Result<String, String> {
    let len = read_u32_le(data, offset)? as usize;
    if *offset + len > data.len() {
        return Err(format!(
            "Buffer underflow reading string of len {} at offset {}",
            len, offset
        ));
    }
    let bytes = &data[*offset..*offset + len];
    *offset += len;
    let s = String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string();
    Ok(s)
}

/// Skip over the Borsh-encoded Option<Vec<Creator>> field.
/// Layout: option_byte (0=None, 1=Some), then if Some: vec_len (u32) + vec_len * 34 bytes each.
/// Each Creator = 32 (pubkey) + 1 (verified) + 1 (share) = 34 bytes.
fn skip_creators(data: &[u8], offset: &mut usize) -> Result<(), String> {
    let has_creators = read_u8(data, offset)?;
    if has_creators == 1 {
        let count = read_u32_le(data, offset)? as usize;
        let bytes_to_skip = count * 34; // 32 (pubkey) + 1 (verified) + 1 (share)
        if *offset + bytes_to_skip > data.len() {
            return Err(format!(
                "Buffer underflow skipping {} creators at offset {}",
                count, offset
            ));
        }
        *offset += bytes_to_skip;
    }
    Ok(())
}

/// Enhanced Metaplex metadata deserializer — extracts ALL useful fields
fn deserialize_extended_metadata(data: &[u8]) -> Result<ExtendedOnChainMetadata, String> {
    let mut offset = 0;

    // Key discriminator (1 byte) — must be MetadataV1 (4)
    let key = read_u8(data, &mut offset)?;
    if key != 4 {
        return Err(format!(
            "Invalid metadata key: {}, expected 4 (MetadataV1)",
            key
        ));
    }

    // Update authority (32 bytes)
    let update_authority = read_pubkey(data, &mut offset)?;

    // Mint (32 bytes)
    let mint = read_pubkey(data, &mut offset)?;

    // Data struct
    let name = read_string(data, &mut offset)?;
    let symbol = read_string(data, &mut offset)?;
    let uri = read_string(data, &mut offset)?;
    let seller_fee_basis_points = read_u16_le(data, &mut offset)?;

    // Skip creators (Option<Vec<Creator>>)
    skip_creators(data, &mut offset)?;

    // primary_sale_happened (1 byte bool)
    let _primary_sale_happened = read_u8(data, &mut offset)?;

    // is_mutable (1 byte bool)
    let is_mutable = read_u8(data, &mut offset)? != 0;

    Ok(ExtendedOnChainMetadata {
        update_authority,
        mint,
        name,
        symbol,
        uri,
        seller_fee_basis_points,
        is_mutable,
    })
}

// ============================================================================
// ENHANCED SPL MINT PARSING
// ============================================================================

/// Extended mint information from SPL Token account
#[derive(Debug, Clone)]
struct ExtendedMintInfo {
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: Option<Pubkey>,
    pub is_token_2022: bool,
}

fn parse_extended_mint_info(
    account_data: &[u8],
    account_owner: &Pubkey,
) -> Result<ExtendedMintInfo, String> {
    use solana_program::program_pack::Pack;
    use spl_token::state::Mint as SplMint;
    use spl_token_2022::state::Mint as Mint2022;

    let is_token_2022 = *account_owner == spl_token_2022::id();

    if *account_owner == spl_token::id() {
        let mint = SplMint::unpack(account_data)
            .map_err(|e| format!("Failed to unpack SPL mint: {}", e))?;
        Ok(ExtendedMintInfo {
            mint_authority: mint.mint_authority.into(),
            supply: mint.supply,
            decimals: mint.decimals,
            is_initialized: mint.is_initialized,
            freeze_authority: mint.freeze_authority.into(),
            is_token_2022: false,
        })
    } else if is_token_2022 {
        // Try standard unpack first
        if let Ok(mint) = Mint2022::unpack(account_data) {
            return Ok(ExtendedMintInfo {
                mint_authority: mint.mint_authority.into(),
                supply: mint.supply,
                decimals: mint.decimals,
                is_initialized: mint.is_initialized,
                freeze_authority: mint.freeze_authority.into(),
                is_token_2022: true,
            });
        }
        // Fallback: extensions-aware parser
        let state =
            spl_token_2022::extension::StateWithExtensionsOwned::<Mint2022>::unpack(
                account_data.to_vec(),
            )
            .map_err(|e| format!("Failed to unpack Token-2022 mint: {}", e))?;
        Ok(ExtendedMintInfo {
            mint_authority: state.base.mint_authority.into(),
            supply: state.base.supply,
            decimals: state.base.decimals,
            is_initialized: state.base.is_initialized,
            freeze_authority: state.base.freeze_authority.into(),
            is_token_2022: true,
        })
    } else {
        Err(format!(
            "Unknown token program owner: {}",
            account_owner
        ))
    }
}

// ============================================================================
// SCAM DETECTION HEURISTICS
// ============================================================================

#[derive(Debug)]
struct ScamAnalysis {
    pub flags: Vec<String>,
    pub risk_score: u32, // 0-100
}

fn analyze_scam_signals(
    metadata: &ExtendedOnChainMetadata,
    mint_info: &ExtendedMintInfo,
) -> ScamAnalysis {
    let mut flags = Vec::new();
    let mut score: u32 = 0;

    // H1: Numeric-only symbol
    if !metadata.symbol.is_empty() && metadata.symbol.chars().all(|c| c.is_ascii_digit()) {
        flags.push(format!(
            "NUMERIC_SYMBOL: '{}' — all digits",
            metadata.symbol
        ));
        score += 40;
    }

    // H2: Empty/whitespace symbol
    if metadata.symbol.is_empty() || metadata.symbol.trim().is_empty() {
        flags.push(format!(
            "EMPTY_SYMBOL: '{}' (len={})",
            metadata.symbol.replace('\0', "\\0"),
            metadata.symbol.len()
        ));
        score += 30;
    }

    // H3: Suspicious round supply (100B with 6 decimals)
    let ui_supply = mint_info.supply as f64 / 10f64.powi(mint_info.decimals as i32);
    if (ui_supply - 99_999_999_999.0).abs() < 1.0 {
        flags.push(format!(
            "SUSPICIOUS_SUPPLY: {:.0} UI tokens ({} raw, {} decimals) — exact 100B pattern",
            ui_supply, mint_info.supply, mint_info.decimals
        ));
        score += 20;
    }

    // H4: Known scam authorities
    let known_scam_freeze = "9N2kn1C8sYM3PrTJ4DY5q7R4uaLXVkrc8C23JR1e6pWW";
    let known_scam_update = "4wTRxzhv8HZZPW6YgrPcrZEwtDTC4RvKjKzZVHbzGAxL";

    if let Some(ref fa) = mint_info.freeze_authority {
        if fa.to_string() == known_scam_freeze {
            flags.push(format!("KNOWN_SCAM_FREEZE_AUTH: {}", fa));
            score += 50;
        }
    }

    if metadata.update_authority.to_string() == known_scam_update {
        flags.push(format!(
            "KNOWN_SCAM_UPDATE_AUTH: {}",
            metadata.update_authority
        ));
        score += 50;
    }

    // H5: Freeze authority present (minor flag)
    if mint_info.freeze_authority.is_some() {
        flags.push(format!(
            "FREEZE_AUTHORITY: {}",
            mint_info.freeze_authority.unwrap()
        ));
        score += 5;
    }

    // H6: Mint authority present
    if mint_info.mint_authority.is_some() {
        flags.push(format!(
            "MINT_AUTHORITY: {}",
            mint_info.mint_authority.unwrap()
        ));
        score += 10;
    }

    // H7: Metadata is immutable (scam tokens often set immutable to prevent cleanup)
    if !metadata.is_mutable {
        // Not a flag by itself, but combined with other signals it's suspicious
        if score > 30 {
            flags.push("IMMUTABLE_METADATA: combined with other risk signals".to_string());
            score += 5;
        }
    }

    // Cap at 100
    score = score.min(100);

    ScamAnalysis {
        flags,
        risk_score: score,
    }
}

// ============================================================================
// PDA DERIVATION
// ============================================================================

fn derive_metadata_pda(mint: &Pubkey) -> Result<Pubkey, String> {
    let program_id =
        Pubkey::from_str(METAPLEX_PROGRAM_ID).map_err(|e| format!("Invalid program ID: {}", e))?;
    let seeds = &[b"metadata" as &[u8], program_id.as_ref(), mint.as_ref()];
    let (pda, _bump) = Pubkey::find_program_address(seeds, &program_id);
    Ok(pda)
}

// ============================================================================
// SINGLE MINT ANALYSIS
// ============================================================================

async fn analyze_single_mint(mint_str: &str, show_raw: bool) -> Result<(), String> {
    let mint_pubkey = Pubkey::from_str(mint_str).map_err(|e| format!("Invalid mint: {}", e))?;
    let metadata_pda = derive_metadata_pda(&mint_pubkey)?;

    println!("═══════════════════════════════════════════════════════════════");
    println!("  ON-CHAIN ANALYSIS: {}", mint_str);
    println!("  Metadata PDA: {}", metadata_pda);
    println!("═══════════════════════════════════════════════════════════════");

    let rpc = get_rpc_client();

    // Batch fetch both accounts in one RPC call
    let accounts = rpc
        .get_multiple_accounts(&[mint_pubkey, metadata_pda])
        .await
        .map_err(|e| format!("RPC error: {}", e))?;

    // Parse mint account
    let mint_info = match &accounts[0] {
        Some(account) => {
            if show_raw {
                println!("\n--- RAW MINT ACCOUNT ({} bytes) ---", account.data.len());
                println!("  Owner: {}", account.owner);
                println!("  Lamports: {}", account.lamports);
            }
            match parse_extended_mint_info(&account.data, &account.owner) {
                Ok(info) => {
                    println!("\n📦 SPL TOKEN MINT:");
                    println!("  Mint authority:   {:?}", info.mint_authority.map(|p| p.to_string()));
                    println!("  Freeze authority: {:?}", info.freeze_authority.map(|p| p.to_string()));
                    println!("  Supply:           {} (raw)", info.supply);
                    println!("  Decimals:         {}", info.decimals);
                    let ui_supply =
                        info.supply as f64 / 10f64.powi(info.decimals as i32);
                    println!("  UI Supply:        {:.2}", ui_supply);
                    println!("  Initialized:      {}", info.is_initialized);
                    println!("  Token-2022:       {}", info.is_token_2022);
                    Some(info)
                }
                Err(e) => {
                    println!("\n❌ MINT PARSE ERROR: {}", e);
                    None
                }
            }
        }
        None => {
            println!("\n❌ MINT ACCOUNT NOT FOUND");
            None
        }
    };

    // Parse metadata account
    let metadata = match &accounts[1] {
        Some(account) => {
            if show_raw {
                println!("\n--- RAW METADATA ACCOUNT ({} bytes) ---", account.data.len());
                println!("  Owner: {}", account.owner);
                // Show first 100 bytes hex
                let hex: String = account.data.iter().take(100).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
                println!("  First 100 bytes: {}", hex);
            }
            match deserialize_extended_metadata(&account.data) {
                Ok(meta) => {
                    println!("\n📋 METAPLEX METADATA:");
                    println!("  Name:             '{}'", meta.name);
                    println!("  Symbol:           '{}'", meta.symbol);
                    println!("  URI:              '{}'", if meta.uri.len() > 80 { format!("{}...", &meta.uri[..80]) } else { meta.uri.clone() });
                    println!("  Update authority:  {}", meta.update_authority);
                    println!("  Mint (in metadata): {}", meta.mint);
                    println!("  Is mutable:       {}", meta.is_mutable);
                    println!("  Seller fee bps:   {}", meta.seller_fee_basis_points);
                    Some(meta)
                }
                Err(e) => {
                    println!("\n❌ METADATA PARSE ERROR: {}", e);
                    None
                }
            }
        }
        None => {
            println!("\n❌ METADATA ACCOUNT NOT FOUND (no Metaplex metadata for this token)");
            None
        }
    };

    // Run scam analysis if we have both
    if let (Some(ref meta), Some(ref mint)) = (&metadata, &mint_info) {
        let analysis = analyze_scam_signals(meta, mint);
        println!("\n🔍 SCAM ANALYSIS:");
        println!("  Risk score: {}/100", analysis.risk_score);
        if analysis.flags.is_empty() {
            println!("  Flags: (none — looks clean)");
        } else {
            for flag in &analysis.flags {
                println!("  ⚠️  {}", flag);
            }
        }

        // Verdict
        if analysis.risk_score >= 60 {
            println!("\n  🔴 VERDICT: HIGH RISK — would be REJECTED by on-chain filter");
        } else if analysis.risk_score >= 30 {
            println!("\n  🟡 VERDICT: MEDIUM RISK — some suspicious signals");
        } else {
            println!("\n  🟢 VERDICT: LOW RISK — passes on-chain filter");
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");
    Ok(())
}

// ============================================================================
// BATCH SCAN FROM DATABASE
// ============================================================================

async fn scan_suspicious_tokens(symbol_pattern: Option<&str>, limit: usize) -> Result<(), String> {
    let db_path = screenerbot::paths::get_tokens_db_path();
    let db = Arc::new(
        TokenDatabase::new(&db_path.to_string_lossy())
            .map_err(|e| format!("DB error: {}", e))?,
    );
    if let Err(err) = init_global_database(db.clone()) {
        eprintln!("(Global DB already init: {})", err);
    }

    let pattern = symbol_pattern.unwrap_or("00");
    println!("═══════════════════════════════════════════════════════════════");
    println!("  SCANNING TOKENS WITH SYMBOL PATTERN: '{}'", pattern);
    println!("  Limit: {}", limit);
    println!("═══════════════════════════════════════════════════════════════\n");

    // Query tokens matching pattern
    let conn = db.connection();
    let conn = conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT mint, symbol, name FROM tokens WHERE symbol = ?1 ORDER BY RANDOM() LIMIT ?2",
        )
        .map_err(|e| format!("SQL error: {}", e))?;

    let rows: Vec<(String, String, String)> = stmt
        .query_map(rusqlite::params![pattern, limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            ))
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        println!("No tokens found with symbol '{}'", pattern);
        return Ok(());
    }

    println!("Found {} tokens. Analyzing on-chain data...\n", rows.len());

    // Statistics tracking
    let mut total_analyzed = 0;
    let mut total_flagged = 0;
    let mut authority_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    let rpc = get_rpc_client();

    // Batch fetch all mint + metadata accounts
    let mint_pubkeys: Vec<Pubkey> = rows
        .iter()
        .filter_map(|(mint, _, _)| Pubkey::from_str(mint).ok())
        .collect();

    let metadata_pdas: Vec<Pubkey> = mint_pubkeys
        .iter()
        .filter_map(|pk| derive_metadata_pda(pk).ok())
        .collect();

    // Interleave: [mint0, meta0, mint1, meta1, ...]
    let mut all_pubkeys = Vec::with_capacity(mint_pubkeys.len() * 2);
    for i in 0..mint_pubkeys.len() {
        all_pubkeys.push(mint_pubkeys[i]);
        if i < metadata_pdas.len() {
            all_pubkeys.push(metadata_pdas[i]);
        }
    }

    // Fetch in batches of 50
    let mut all_accounts: Vec<Option<solana_sdk::account::Account>> = Vec::new();
    for chunk in all_pubkeys.chunks(50) {
        match rpc.get_multiple_accounts(chunk).await {
            Ok(accounts) => all_accounts.extend(accounts),
            Err(e) => {
                eprintln!("RPC batch error: {}", e);
                // Fill with None for failed batch
                all_accounts.extend(std::iter::repeat(None).take(chunk.len()));
            }
        }
    }

    // Process results
    for (idx, (mint_str, db_symbol, db_name)) in rows.iter().enumerate() {
        let mint_idx = idx * 2;
        let meta_idx = idx * 2 + 1;

        let mint_account = all_accounts.get(mint_idx).and_then(|a| a.as_ref());
        let meta_account = all_accounts.get(meta_idx).and_then(|a| a.as_ref());

        let mint_info = mint_account.and_then(|acc| {
            parse_extended_mint_info(&acc.data, &acc.owner).ok()
        });

        let metadata = meta_account.and_then(|acc| {
            deserialize_extended_metadata(&acc.data).ok()
        });

        total_analyzed += 1;

        print!("[{}/{}] {} ", idx + 1, rows.len(), &mint_str[..8]);

        if let (Some(ref meta), Some(ref mint)) = (&metadata, &mint_info) {
            let analysis = analyze_scam_signals(meta, mint);

            // Track authority clustering
            if let Some(ref fa) = mint.freeze_authority {
                *authority_counts.entry(format!("freeze:{}", fa)).or_insert(0) += 1;
            }
            *authority_counts
                .entry(format!("update:{}", meta.update_authority))
                .or_insert(0) += 1;

            if analysis.risk_score >= 30 {
                total_flagged += 1;
                println!(
                    "⚠️  score={} name='{}' symbol='{}' flags={}",
                    analysis.risk_score,
                    meta.name,
                    meta.symbol,
                    analysis.flags.len()
                );
                for flag in &analysis.flags {
                    println!("       {}", flag);
                }
            } else {
                println!(
                    "✅ score={} name='{}' symbol='{}'",
                    analysis.risk_score, meta.name, meta.symbol
                );
            }
        } else {
            println!(
                "❓ DB: name='{}' symbol='{}' (chain data unavailable)",
                db_name, db_symbol
            );
        }
    }

    // Summary
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("  BATCH SCAN SUMMARY");
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Total analyzed: {}", total_analyzed);
    println!("  Flagged (score≥30): {} ({:.1}%)", total_flagged, if total_analyzed > 0 { total_flagged as f64 / total_analyzed as f64 * 100.0 } else { 0.0 });

    // Authority clustering
    let mut auth_vec: Vec<_> = authority_counts.iter().filter(|(_, &c)| c > 1).collect();
    auth_vec.sort_by(|a, b| b.1.cmp(a.1));
    if !auth_vec.is_empty() {
        println!("\n  🔗 AUTHORITY CLUSTERING (shared across >1 token):");
        for (auth, count) in auth_vec.iter().take(10) {
            println!("     {} → {} tokens", auth, count);
        }
    }
    println!("═══════════════════════════════════════════════════════════════\n");

    Ok(())
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize RPC client
    screenerbot::paths::ensure_all_directories().ok();
    screenerbot::config::load_config().expect("Failed to load config");
    init_rpc_client().expect("Failed to init RPC client");

    if let Some(ref mint) = args.mint {
        analyze_single_mint(mint, args.raw).await.map_err(|e| {
            eprintln!("Error: {}", e);
            e
        })?;
    } else if args.scan_scam_symbols {
        scan_suspicious_tokens(args.symbol.as_deref(), args.limit)
            .await
            .map_err(|e| {
                eprintln!("Error: {}", e);
                e
            })?;
    } else {
        // Default: analyze known scam tokens
        println!("No arguments provided. Analyzing known scam tokens from '00' investigation...\n");

        let known_scam_mints = [
            "AuBdvhvJG9i682kQaJQFdKEzcTtvDxbAUWMtmX1LTaJH", // "BFS" — symbol "00"
            "D6mUqC4jag9CTP7WKR1S9vpgFs1h3Cn1PVYhYbTJJMs2", // "US Gold Reserve" — symbol "00"
            "Hm8eMDx24BpfmkHWERFMNUhNkE5qAwTNcsv2VMLbvMy5", // "Scoutly AI" — symbol "00"
        ];

        for mint in &known_scam_mints {
            if let Err(e) = analyze_single_mint(mint, args.raw).await {
                eprintln!("Failed to analyze {}: {}", mint, e);
            }
        }
    }

    Ok(())
}
