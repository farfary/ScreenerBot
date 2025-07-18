use screenerbot::rpc::RpcManager;
use screenerbot::config::RpcConfig;
use screenerbot::pairs::decoders::{ DecoderRegistry, PoolInfo, price_math };
use anyhow::Result;
use std::sync::Arc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct PoolProgram {
    name: &'static str,
    program_id: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone)]
struct KnownField {
    name: String,
    offset: usize,
    field_type: FieldType,
    expected_value: Option<String>,
}

#[derive(Debug, Clone)]
enum FieldType {
    U8,
    U16,
    U32,
    U64,
    I32,
    I64,
    Pubkey,
    Bool,
    Unknown,
}

// Known pool programs
const KNOWN_PROGRAMS: &[PoolProgram] = &[
    PoolProgram {
        name: "Raydium CLMM",
        program_id: "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
        description: "Raydium Concentrated Liquidity Market Maker",
    },
    PoolProgram {
        name: "Meteora DLMM",
        program_id: "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
        description: "Meteora Dynamic Liquidity Market Maker",
    },
    PoolProgram {
        name: "Orca Whirlpools",
        program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
        description: "Orca Whirlpool AMM",
    },
    PoolProgram {
        name: "Pump.fun AMM",
        program_id: "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
        description: "Pump.fun Automated Market Maker",
    },
    PoolProgram {
        name: "Raydium V4",
        program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
        description: "Raydium V4 AMM Program",
    },
    PoolProgram {
        name: "Raydium CPMM",
        program_id: "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
        description: "Raydium Constant Product Market Maker",
    },
];

fn parse_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

fn parse_u16(data: &[u8], offset: usize) -> Option<u16> {
    if offset + 2 <= data.len() {
        let bytes: [u8; 2] = data[offset..offset + 2].try_into().ok()?;
        Some(u16::from_le_bytes(bytes))
    } else {
        None
    }
}

fn parse_u32(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 <= data.len() {
        let bytes: [u8; 4] = data[offset..offset + 4].try_into().ok()?;
        Some(u32::from_le_bytes(bytes))
    } else {
        None
    }
}

fn parse_u64(data: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 <= data.len() {
        let bytes: [u8; 8] = data[offset..offset + 8].try_into().ok()?;
        Some(u64::from_le_bytes(bytes))
    } else {
        None
    }
}

fn parse_i32(data: &[u8], offset: usize) -> Option<i32> {
    if offset + 4 <= data.len() {
        let bytes: [u8; 4] = data[offset..offset + 4].try_into().ok()?;
        Some(i32::from_le_bytes(bytes))
    } else {
        None
    }
}

fn parse_i64(data: &[u8], offset: usize) -> Option<i64> {
    if offset + 8 <= data.len() {
        let bytes: [u8; 8] = data[offset..offset + 8].try_into().ok()?;
        Some(i64::from_le_bytes(bytes))
    } else {
        None
    }
}

fn parse_pubkey(data: &[u8], offset: usize) -> Option<Pubkey> {
    if offset + 32 <= data.len() {
        let bytes: [u8; 32] = data[offset..offset + 32].try_into().ok()?;
        Some(Pubkey::from(bytes))
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage(&args[0]);
        return Ok(());
    }

    // Initialize RPC manager
    let primary_url = std::env
        ::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    let fallback_urls = vec!["https://solana-api.projectserum.com".to_string()];

    let rpc_config = RpcConfig::default();
    let rpc_manager = Arc::new(RpcManager::new(primary_url, fallback_urls, rpc_config)?);

    match args[1].as_str() {
        "analyze" => {
            if args.len() < 3 {
                println!("Usage: {} analyze <POOL_ADDRESS> [expected_values.json]", args[0]);
                return Ok(());
            }
            let pool_address = &args[2];
            let expected_file = args.get(3);
            analyze_pool(&rpc_manager, pool_address, expected_file).await?;
        }
        "compare" => {
            if args.len() < 4 {
                println!("Usage: {} compare <POOL_ADDRESS_1> <POOL_ADDRESS_2>", args[0]);
                return Ok(());
            }
            let pool1 = &args[2];
            let pool2 = &args[3];
            compare_pools(&rpc_manager, pool1, pool2).await?;
        }
        "search" => {
            if args.len() < 4 {
                println!("Usage: {} search <POOL_ADDRESS> <SEARCH_VALUE>", args[0]);
                return Ok(());
            }
            let pool_address = &args[2];
            let search_value = &args[3];
            search_in_pool(&rpc_manager, pool_address, search_value).await?;
        }
        "programs" => {
            list_known_programs();
        }
        "price" => {
            if args.len() < 3 {
                println!("Usage: {} price <POOL_ADDRESS> [--verbose]", args[0]);
                return Ok(());
            }
            let pool_address = &args[2];
            let verbose = args
                .get(3)
                .map(|s| s == "--verbose")
                .unwrap_or(false);
            calculate_pool_price(&rpc_manager, pool_address, verbose).await?;
        }
        "export" => {
            if args.len() < 3 {
                println!("Usage: {} export <POOL_ADDRESS> [output.bin]", args[0]);
                return Ok(());
            }
            let pool_address = &args[2];
            let output_file = args
                .get(3)
                .map(|s| s.as_str())
                .unwrap_or("pool_data.bin");
            export_pool_data(&rpc_manager, pool_address, output_file).await?;
        }
        _ => {
            print_usage(&args[0]);
        }
    }

    Ok(())
}

fn print_usage(program_name: &str) {
    println!("🔧 Advanced Pool Debug Tool");
    println!("===========================");
    println!("Usage: {} <COMMAND> [OPTIONS]", program_name);
    println!();
    println!("Commands:");
    println!("  analyze <POOL_ADDRESS> [expected.json]  - Analyze pool structure");
    println!("  compare <POOL_1> <POOL_2>              - Compare two pools");
    println!("  search <POOL_ADDRESS> <VALUE>          - Search for value in pool data");
    println!("  price <POOL_ADDRESS> [--verbose]       - Calculate pool price");
    println!("  programs                               - List known pool programs");
    println!("  export <POOL_ADDRESS> [file.bin]       - Export raw pool data");
    println!();
    println!("Examples:");
    println!("  {} analyze CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C", program_name);
    println!("  {} price LaT4mSXV2gPyjQsuCBZ7XmV1G7DEoToXVBf4pEvL6be --verbose", program_name);
    println!("  {} search CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C So11111111111111111111111111111111111111112", program_name);
    println!("  {} compare POOL1 POOL2", program_name);
}

async fn analyze_pool(
    rpc_manager: &Arc<RpcManager>,
    pool_address: &str,
    expected_file: Option<&String>
) -> Result<()> {
    let pool_pubkey = Pubkey::from_str(pool_address)?;

    println!("🔍 Analyzing Pool: {}", pool_address);
    println!("================================================================================");

    // Get account data
    let account = rpc_manager.get_account(&pool_pubkey).await?;
    let data = &account.data;

    println!("✅ Account found!");
    println!("📋 Program owner: {}", account.owner);
    println!("📏 Data length: {} bytes", data.len());
    println!("💰 Lamports: {}", account.lamports);

    // Identify pool type
    let pool_program = identify_pool_program(&account.owner);
    if let Some(program) = pool_program {
        println!("🏷️  Pool Type: {} ({})", program.name, program.description);
    } else {
        println!("❓ Unknown pool type - Program: {}", account.owner);
    }

    // Load expected values if provided
    let expected_values = if let Some(file_path) = expected_file {
        load_expected_values(file_path)?
    } else {
        HashMap::new()
    };

    // Analyze structure based on pool type
    match account.owner.to_string().as_str() {
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => {
            analyze_raydium_cpmm_structure(data, &expected_values);
        }
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => {
            analyze_meteora_dlmm_structure(data, &expected_values);
        }
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => {
            analyze_pumpfun_structure(data, &expected_values);
        }
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => {
            analyze_raydium_clmm_structure(data, &expected_values);
        }
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => {
            analyze_whirlpool_structure(data, &expected_values);
        }
        _ => {
            analyze_generic_structure(data, &expected_values);
        }
    }

    // Always show hex dump for manual analysis
    print_hex_dump(data);

    Ok(())
}

fn identify_pool_program(program_id: &Pubkey) -> Option<&'static PoolProgram> {
    KNOWN_PROGRAMS.iter().find(|p| p.program_id == program_id.to_string())
}

fn load_expected_values(file_path: &str) -> Result<HashMap<String, String>> {
    // For now, return empty HashMap. Could implement JSON parsing here
    println!("📄 Loading expected values from: {}", file_path);
    Ok(HashMap::new())
}

fn analyze_raydium_cpmm_structure(data: &[u8], _expected_values: &HashMap<String, String>) {
    println!("\n🔍 Analyzing Raydium CPMM structure...");

    // Based on the JSON structure provided, let's map known fields
    let known_fields = vec![
        ("token_0_mint", "So11111111111111111111111111111111111111112"),
        ("token_1_mint", "83kGGSggYGP2ZEEyvX54SkZR1kFn84RgGCDyptbDbonk"),
        ("token_0_vault", "7MMHQRjsGQNBY5cXieD6oEjZq313Bc4WaYfC9fvQaZaC"),
        ("token_1_vault", "iRy53eLQWsYxC9vbp8bnpDeywin1CLUFhgFwqAguxLD"),
        ("lp_mint", "H1vC6DAHGtDzeqDyXN6nXJzmK7CAM581cEfgAGcigRdb"),
        ("amm_config", "D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2")
    ];

    search_known_pubkeys(data, &known_fields);

    // Look for specific numeric values
    search_numeric_values(data);

    // Try to identify structure by looking for discriminator patterns
    analyze_discriminator_patterns(data);
}

fn analyze_meteora_dlmm_structure(data: &[u8], _expected_values: &HashMap<String, String>) {
    println!("\n🔍 Analyzing Meteora DLMM structure...");

    // We already know this structure from previous analysis
    let known_offsets = vec![
        (48, "activeId", FieldType::I32),
        (73, "binStep", FieldType::U16),
        (88, "tokenXMint", FieldType::Pubkey),
        (120, "tokenYMint", FieldType::Pubkey),
        (152, "reserveX", FieldType::Pubkey),
        (184, "reserveY", FieldType::Pubkey)
    ];

    for (offset, name, field_type) in known_offsets {
        print_field_at_offset(data, offset, name, field_type);
    }
}

fn analyze_pumpfun_structure(data: &[u8], _expected_values: &HashMap<String, String>) {
    println!("\n🔍 Analyzing Pump.fun AMM structure...");

    // Known structure from previous analysis
    let known_offsets = vec![
        (8, "pool_bump", FieldType::U8),
        (9, "index", FieldType::U16),
        (11, "creator", FieldType::Pubkey),
        (43, "base_mint", FieldType::Pubkey),
        (75, "quote_mint", FieldType::Pubkey),
        (107, "lp_mint", FieldType::Pubkey),
        (139, "pool_base_token_account", FieldType::Pubkey),
        (171, "pool_quote_token_account", FieldType::Pubkey),
        (203, "lp_supply", FieldType::U64),
        (211, "coin_creator", FieldType::Pubkey)
    ];

    for (offset, name, field_type) in known_offsets {
        print_field_at_offset(data, offset, name, field_type);
    }
}

fn analyze_raydium_clmm_structure(data: &[u8], _expected_values: &HashMap<String, String>) {
    println!("\n🔍 Analyzing Raydium CLMM structure...");
    println!("ℹ️  This structure needs to be mapped - please provide sample data");
    analyze_generic_structure(data, _expected_values);
}

fn analyze_whirlpool_structure(data: &[u8], _expected_values: &HashMap<String, String>) {
    println!("\n🔍 Analyzing Whirlpool structure...");
    println!("ℹ️  This structure needs to be mapped - please provide sample data");
    analyze_generic_structure(data, _expected_values);
}

fn analyze_generic_structure(data: &[u8], _expected_values: &HashMap<String, String>) {
    println!("\n🔍 Performing generic structure analysis...");

    // Look for common patterns
    search_common_pubkeys(data);
    analyze_discriminator_patterns(data);
    find_potential_numbers(data);
}

fn search_known_pubkeys(data: &[u8], known_fields: &[(&str, &str)]) {
    println!("\n📍 Searching for known pubkeys...");

    for (field_name, expected_pubkey) in known_fields {
        if let Ok(expected) = Pubkey::from_str(expected_pubkey) {
            for i in 0..data.len().saturating_sub(32) {
                if let Some(found_pubkey) = parse_pubkey(data, i) {
                    if found_pubkey == expected {
                        println!("   ✅ Found {} at offset {}: {}", field_name, i, found_pubkey);
                    }
                }
            }
        }
    }
}

fn search_common_pubkeys(data: &[u8]) {
    println!("\n🔍 Searching for common pubkeys...");

    let common_pubkeys = vec![
        ("SOL", "So11111111111111111111111111111111111111112"),
        ("USDC", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        ("USDT", "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"),
        ("Token Program", "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
        ("System Program", "11111111111111111111111111111111")
    ];

    for (name, pubkey_str) in common_pubkeys {
        if let Ok(pubkey) = Pubkey::from_str(pubkey_str) {
            for i in 0..data.len().saturating_sub(32) {
                if let Some(found_pubkey) = parse_pubkey(data, i) {
                    if found_pubkey == pubkey {
                        println!("   ✅ Found {} at offset {}: {}", name, i, found_pubkey);
                    }
                }
            }
        }
    }
}

fn search_numeric_values(data: &[u8]) {
    println!("\n🔢 Searching for specific numeric patterns...");

    // Look for common decimal values
    let common_decimals = [6, 9];
    for decimal in common_decimals {
        for i in 0..data.len() {
            if let Some(val) = parse_u8(data, i) {
                if val == decimal {
                    println!("   📊 Found decimal {} at offset {}", decimal, i);
                }
            }
        }
    }

    // Look for status values (0, 1, 2)
    for status in [0u8, 1u8, 2u8] {
        for i in 0..data.len() {
            if let Some(val) = parse_u8(data, i) {
                if val == status {
                    println!("   🔄 Found potential status {} at offset {}", status, i);
                }
            }
        }
    }
}

fn analyze_discriminator_patterns(data: &[u8]) {
    println!("\n🔬 Analyzing discriminator patterns...");

    if data.len() >= 8 {
        let discriminator = &data[0..8];
        println!("   📝 8-byte discriminator: {:02x?}", discriminator);

        // Check if it looks like an Anchor discriminator (first 8 bytes)
        let is_likely_anchor = discriminator.iter().any(|&b| b != 0);
        if is_likely_anchor {
            println!("   🎯 Likely Anchor program (non-zero discriminator)");
        } else {
            println!("   ❓ Zero discriminator - might be native program");
        }
    }
}

fn find_potential_numbers(data: &[u8]) {
    println!("\n🔍 Looking for interesting numeric values...");

    // Look for large numbers that might be amounts
    for i in 0..data.len().saturating_sub(8) {
        if let Some(val) = parse_u64(data, i) {
            if val > 1_000_000 && val < u64::MAX / 2 {
                println!("   💰 Large number at offset {}: {} ({})", i, val, format_number(val));
            }
        }
    }
}

fn print_field_at_offset(data: &[u8], offset: usize, name: &str, field_type: FieldType) {
    match field_type {
        FieldType::U8 => {
            if let Some(val) = parse_u8(data, offset) {
                println!("   {} at offset {}: {}", name, offset, val);
            }
        }
        FieldType::U16 => {
            if let Some(val) = parse_u16(data, offset) {
                println!("   {} at offset {}: {}", name, offset, val);
            }
        }
        FieldType::U32 => {
            if let Some(val) = parse_u32(data, offset) {
                println!("   {} at offset {}: {}", name, offset, val);
            }
        }
        FieldType::U64 => {
            if let Some(val) = parse_u64(data, offset) {
                println!("   {} at offset {}: {} ({})", name, offset, val, format_number(val));
            }
        }
        FieldType::I32 => {
            if let Some(val) = parse_i32(data, offset) {
                println!("   {} at offset {}: {}", name, offset, val);
            }
        }
        FieldType::I64 => {
            if let Some(val) = parse_i64(data, offset) {
                println!("   {} at offset {}: {}", name, offset, val);
            }
        }
        FieldType::Pubkey => {
            if let Some(val) = parse_pubkey(data, offset) {
                println!("   {} at offset {}: {}", name, offset, val);
            }
        }
        _ => {}
    }
}

fn format_number(num: u64) -> String {
    if num >= 1_000_000_000 {
        format!("{:.2}B", (num as f64) / 1_000_000_000.0)
    } else if num >= 1_000_000 {
        format!("{:.2}M", (num as f64) / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.2}K", (num as f64) / 1_000.0)
    } else {
        num.to_string()
    }
}

async fn compare_pools(rpc_manager: &Arc<RpcManager>, pool1: &str, pool2: &str) -> Result<()> {
    println!("🔍 Comparing Pools");
    println!("================================================================================");

    let pubkey1 = Pubkey::from_str(pool1)?;
    let pubkey2 = Pubkey::from_str(pool2)?;

    let account1 = rpc_manager.get_account(&pubkey1).await?;
    let account2 = rpc_manager.get_account(&pubkey2).await?;

    println!("Pool 1: {} ({} bytes)", pool1, account1.data.len());
    println!("Pool 2: {} ({} bytes)", pool2, account2.data.len());
    println!("Program 1: {}", account1.owner);
    println!("Program 2: {}", account2.owner);

    if account1.owner == account2.owner {
        println!("✅ Same program - comparing structure...");
        compare_data_structures(&account1.data, &account2.data);
    } else {
        println!("❌ Different programs - limited comparison possible");
    }

    Ok(())
}

fn compare_data_structures(data1: &[u8], data2: &[u8]) {
    let min_len = data1.len().min(data2.len());
    let mut differences = 0;

    for i in 0..min_len {
        if data1[i] != data2[i] {
            differences += 1;
            if differences <= 10 {
                println!("   Diff at offset {}: {:02x} vs {:02x}", i, data1[i], data2[i]);
            }
        }
    }

    if differences > 10 {
        println!("   ... and {} more differences", differences - 10);
    }

    println!(
        "Total differences: {} / {} bytes ({:.1}%)",
        differences,
        min_len,
        ((differences as f64) / (min_len as f64)) * 100.0
    );
}

async fn search_in_pool(
    rpc_manager: &Arc<RpcManager>,
    pool_address: &str,
    search_value: &str
) -> Result<()> {
    let pool_pubkey = Pubkey::from_str(pool_address)?;
    let account = rpc_manager.get_account(&pool_pubkey).await?;
    let data = &account.data;

    println!("🔍 Searching for '{}' in pool {}", search_value, pool_address);
    println!("================================================================================");

    // Try to parse as pubkey
    if let Ok(search_pubkey) = Pubkey::from_str(search_value) {
        println!("🔍 Searching for pubkey: {}", search_pubkey);
        for i in 0..data.len().saturating_sub(32) {
            if let Some(found_pubkey) = parse_pubkey(data, i) {
                if found_pubkey == search_pubkey {
                    println!("   ✅ Found at offset {}", i);
                }
            }
        }
    }

    // Try to parse as number
    if let Ok(search_num) = search_value.parse::<u64>() {
        println!("🔍 Searching for number: {}", search_num);
        for i in 0..data.len().saturating_sub(8) {
            if let Some(found_num) = parse_u64(data, i) {
                if found_num == search_num {
                    println!("   ✅ Found u64 at offset {}", i);
                }
            }
        }

        if search_num <= (u32::MAX as u64) {
            for i in 0..data.len().saturating_sub(4) {
                if let Some(found_num) = parse_u32(data, i) {
                    if found_num == (search_num as u32) {
                        println!("   ✅ Found u32 at offset {}", i);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn export_pool_data(
    rpc_manager: &Arc<RpcManager>,
    pool_address: &str,
    output_file: &str
) -> Result<()> {
    let pool_pubkey = Pubkey::from_str(pool_address)?;
    let account = rpc_manager.get_account(&pool_pubkey).await?;

    std::fs::write(output_file, &account.data)?;
    println!("✅ Exported {} bytes to {}", account.data.len(), output_file);

    Ok(())
}

fn list_known_programs() {
    println!("🏷️  Known Pool Programs");
    println!("================================================================================");

    for program in KNOWN_PROGRAMS {
        println!("✅ {}", program.name);
        println!("   Program ID: {}", program.program_id);
        println!("   Description: {}", program.description);
        println!();
    }
}

async fn calculate_pool_price(
    rpc_manager: &Arc<RpcManager>,
    pool_address: &str,
    verbose: bool
) -> Result<()> {
    let pool_pubkey = Pubkey::from_str(pool_address)?;

    println!("💰 Calculating Pool Price: {}", pool_address);
    println!("================================================================================");

    // Get account data
    let account = rpc_manager.get_account(&pool_pubkey).await?;
    let data = &account.data;

    println!("✅ Account found!");
    println!("📋 Program owner: {}", account.owner);
    println!("📏 Data length: {} bytes", data.len());

    // Identify pool type
    let pool_program = identify_pool_program(&account.owner);
    if let Some(program) = pool_program {
        println!("🏷️  Pool Type: {} ({})", program.name, program.description);
    } else {
        println!("❓ Unknown pool type - Program: {}", account.owner);
        return Ok(());
    }

    // Create decoder registry
    let registry = DecoderRegistry::new();

    // Try to decode pool data
    match registry.decode_pool(&account.owner, data) {
        Ok(mut pool_info) => {
            // Set the pool address since the decoder doesn't know it
            pool_info.pool_address = pool_pubkey;

            // Fetch actual vault balances to get reserves
            if let Err(e) = fetch_vault_balances(rpc_manager, &mut pool_info).await {
                println!("⚠️  Warning: Could not fetch vault balances: {}", e);
            }

            if verbose {
                print_detailed_pool_info(&pool_info);
            }

            // Calculate price using the decoder
            match registry.calculate_price(&account.owner, &pool_info) {
                Ok(price) => {
                    println!("\n💱 Price Calculation Results:");
                    println!(
                        "================================================================================"
                    );
                    println!("✅ Price (token1/token0): {:.8}", price);
                    println!("✅ Inverted price (token0/token1): {:.8}", if price != 0.0 {
                        1.0 / price
                    } else {
                        0.0
                    });

                    // Calculate alternative price representations
                    print_price_analysis(&pool_info, price);
                }
                Err(e) => {
                    println!("❌ Failed to calculate price: {}", e);

                    // Try manual calculation based on pool type
                    println!("\n🔧 Attempting manual price calculation...");
                    attempt_manual_price_calculation(&pool_info, verbose);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to decode pool: {}", e);

            // Try to extract basic information manually
            println!("\n🔧 Attempting manual analysis...");
            attempt_manual_pool_analysis(&account.owner, data, verbose).await?;
        }
    }

    Ok(())
}

async fn fetch_vault_balances(
    rpc_manager: &Arc<RpcManager>,
    pool_info: &mut PoolInfo
) -> Result<()> {
    // Fetch token account balances for the vaults
    if let Ok(vault_0_account) = rpc_manager.get_account(&pool_info.token_vault_0).await {
        if vault_0_account.data.len() >= 72 {
            // SPL Token account data: amount is at offset 64 (8 bytes, little-endian)
            let amount_bytes: [u8; 8] = vault_0_account.data[64..72]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to parse vault 0 amount"))?;
            pool_info.reserve_0 = u64::from_le_bytes(amount_bytes);
        }
    }

    if let Ok(vault_1_account) = rpc_manager.get_account(&pool_info.token_vault_1).await {
        if vault_1_account.data.len() >= 72 {
            // SPL Token account data: amount is at offset 64 (8 bytes, little-endian)
            let amount_bytes: [u8; 8] = vault_1_account.data[64..72]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to parse vault 1 amount"))?;
            pool_info.reserve_1 = u64::from_le_bytes(amount_bytes);
        }
    }

    Ok(())
}

fn print_detailed_pool_info(pool_info: &PoolInfo) {
    println!("\n📊 Pool Information:");
    println!("================================================================================");
    println!("🏷️  Pool Type: {:?}", pool_info.pool_type);
    println!("📍 Pool Address: {}", pool_info.pool_address);
    println!("🏛️  Program ID: {}", pool_info.program_id);
    println!("🪙 Token Mint 0: {}", pool_info.token_mint_0);
    println!("🪙 Token Mint 1: {}", pool_info.token_mint_1);
    println!("🏦 Token Vault 0: {}", pool_info.token_vault_0);
    println!("🏦 Token Vault 1: {}", pool_info.token_vault_1);
    println!("💰 Reserve 0: {} (decimals: {})", pool_info.reserve_0, pool_info.decimals_0);
    println!("💰 Reserve 1: {} (decimals: {})", pool_info.reserve_1, pool_info.decimals_1);
    println!("🔄 Status: {:?}", pool_info.status);

    if let Some(liquidity) = pool_info.liquidity {
        println!("💧 Liquidity: {}", liquidity);
    }

    if let Some(sqrt_price) = pool_info.sqrt_price {
        println!("📐 Sqrt Price: {}", sqrt_price);
        let price_from_sqrt = price_math::sqrt_price_to_price(sqrt_price);
        println!("📐 Price from Sqrt: {:.8}", price_from_sqrt);
    }

    if let Some(tick) = pool_info.current_tick {
        println!("📊 Current Tick: {}", tick);
        let price_from_tick = price_math::tick_to_price(tick);
        println!("📊 Price from Tick: {:.8}", price_from_tick);
    }

    if let Some(fee_rate) = pool_info.fee_rate {
        println!("💸 Fee Rate: {}", fee_rate);
    }
}

fn print_price_analysis(pool_info: &PoolInfo, price: f64) {
    println!("\n📈 Price Analysis:");
    println!("================================================================================");

    // Format price in different ways
    if price > 0.0 {
        println!("📊 Price (scientific): {:.2e}", price);

        if price >= 1.0 {
            println!("📊 Price (standard): {:.6}", price);
        } else if price >= 0.001 {
            println!("📊 Price (milli): {:.6} ({:.3} m)", price, price * 1000.0);
        } else if price >= 0.000001 {
            println!("📊 Price (micro): {:.6} ({:.3} μ)", price, price * 1_000_000.0);
        } else {
            println!("📊 Price (nano): {:.6} ({:.3} n)", price, price * 1_000_000_000.0);
        }
    }

    // Show reserve-based calculation for comparison
    if pool_info.reserve_0 > 0 && pool_info.reserve_1 > 0 {
        let reserve_price = price_math::reserves_to_price(
            pool_info.reserve_0,
            pool_info.reserve_1,
            pool_info.decimals_0,
            pool_info.decimals_1
        );
        println!("🔧 Reserve-based price: {:.8}", reserve_price);

        if (price - reserve_price).abs() > 0.0001 {
            println!(
                "⚠️  Price discrepancy detected! Difference: {:.8}",
                (price - reserve_price).abs()
            );
        }
    }

    // Token info if available
    if pool_info.token_mint_0.to_string() == "So11111111111111111111111111111111111111112" {
        println!("💡 Token 0 is SOL, price shows tokens per SOL");
    }
    if pool_info.token_mint_1.to_string() == "So11111111111111111111111111111111111111112" {
        println!("💡 Token 1 is SOL, price shows SOL per token");
    }
}

fn attempt_manual_price_calculation(pool_info: &PoolInfo, verbose: bool) {
    println!("🔧 Manual Price Calculation Attempt:");

    // Try different calculation methods based on available data
    let mut calculated_prices = Vec::new();

    // Method 1: Reserve-based (for AMM pools)
    if pool_info.reserve_0 > 0 && pool_info.reserve_1 > 0 {
        let price = price_math::reserves_to_price(
            pool_info.reserve_0,
            pool_info.reserve_1,
            pool_info.decimals_0,
            pool_info.decimals_1
        );
        calculated_prices.push(("Reserve-based", price));
        println!("   📊 Reserve-based: {:.8}", price);
    }

    // Method 2: Sqrt price (for concentrated liquidity)
    if let Some(sqrt_price) = pool_info.sqrt_price {
        let price = price_math::sqrt_price_to_price(sqrt_price);
        calculated_prices.push(("Sqrt price", price));
        println!("   📐 Sqrt price: {:.8}", price);
    }

    // Method 3: Tick-based (for concentrated liquidity)
    if let Some(tick) = pool_info.current_tick {
        let price = price_math::tick_to_price(tick);
        calculated_prices.push(("Tick-based", price));
        println!("   📊 Tick-based: {:.8}", price);
    }

    if calculated_prices.is_empty() {
        println!("   ❌ No data available for price calculation");
    } else if verbose {
        println!("\n🔍 Price Comparison:");
        for (method, price) in &calculated_prices {
            println!("   {} = {:.8}", method, price);
        }

        // Check for consensus
        if calculated_prices.len() > 1 {
            let prices: Vec<f64> = calculated_prices
                .iter()
                .map(|(_, p)| *p)
                .collect();
            let avg_price = prices.iter().sum::<f64>() / (prices.len() as f64);
            let max_deviation = prices
                .iter()
                .map(|p| (p - avg_price).abs())
                .fold(0.0, f64::max);

            println!("   📊 Average: {:.8}", avg_price);
            println!(
                "   📏 Max deviation: {:.8} ({:.2}%)",
                max_deviation,
                (max_deviation / avg_price) * 100.0
            );

            if max_deviation / avg_price < 0.01 {
                println!("   ✅ Good price consensus (< 1% deviation)");
            } else {
                println!("   ⚠️  High price variance (> 1% deviation)");
            }
        }
    }
}

async fn attempt_manual_pool_analysis(
    program_id: &Pubkey,
    data: &[u8],
    verbose: bool
) -> Result<()> {
    println!("🔧 Manual Pool Analysis:");

    match program_id.to_string().as_str() {
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => {
            println!("   🔍 Raydium CPMM - extracting reserves from vaults...");
            // For CPMM, we need to fetch vault token amounts
            attempt_cpmm_price_calculation(data, verbose).await?;
        }
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => {
            println!("   🔍 Meteora DLMM - analyzing bin structure...");
            attempt_dlmm_price_calculation(data, verbose);
        }
        _ => {
            println!("   ❓ Unknown pool type for manual analysis");
        }
    }

    Ok(())
}

async fn attempt_cpmm_price_calculation(data: &[u8], verbose: bool) -> Result<()> {
    // Extract vault addresses from known offsets
    if data.len() >= 232 {
        if let (Some(vault_0), Some(vault_1)) = (parse_pubkey(data, 72), parse_pubkey(data, 104)) {
            println!("   🏦 Token Vault 0: {}", vault_0);
            println!("   🏦 Token Vault 1: {}", vault_1);
            println!(
                "   ℹ️  Note: Need RPC call to get actual vault balances for price calculation"
            );

            if verbose {
                if
                    let (Some(mint_0), Some(mint_1)) = (
                        parse_pubkey(data, 168),
                        parse_pubkey(data, 200),
                    )
                {
                    println!("   🪙 Token Mint 0: {}", mint_0);
                    println!("   🪙 Token Mint 1: {}", mint_1);
                }
            }
        }
    }

    Ok(())
}

fn attempt_dlmm_price_calculation(data: &[u8], verbose: bool) {
    // Extract active bin and other DLMM-specific data
    if data.len() >= 120 {
        if let Some(active_id) = parse_i32(data, 48) {
            println!("   📊 Active Bin ID: {}", active_id);

            // Convert bin ID to price (simplified)
            let bin_step = parse_u16(data, 73).unwrap_or(1);
            let price_from_bin = (1.0001_f64).powi(active_id) * (1.0 + (bin_step as f64) / 10000.0);
            println!("   📊 Estimated price from bin: {:.8}", price_from_bin);
        }

        if verbose {
            if let Some(bin_step) = parse_u16(data, 73) {
                println!("   📏 Bin Step: {}", bin_step);
            }
        }
    }
}

fn print_hex_dump(data: &[u8]) {
    println!("\n📊 Raw data (hex dump):");
    println!("================================================================================");
    for (i, chunk) in data.chunks(32).enumerate() {
        print!("{:04x}: ", i * 32);
        for byte in chunk {
            print!("{:02x} ", byte);
        }

        // Print ASCII representation
        print!(" |");
        for byte in chunk {
            if byte.is_ascii_graphic() || *byte == b' ' {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}
