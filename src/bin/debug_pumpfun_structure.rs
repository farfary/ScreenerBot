use std::env;
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use screenerbot::rpc::{ init_rpc_client };

const PUMP_FUN_AMM_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

fn parse_pubkey(s: &str) -> Result<Pubkey> {
    Pubkey::from_str(s).map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))
}

fn read_pubkey(data: &[u8], offset: usize) -> Option<String> {
    if offset + 32 <= data.len() {
        let pubkey_bytes = &data[offset..offset + 32];
        if let Ok(pubkey) = Pubkey::try_from(pubkey_bytes) {
            Some(pubkey.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 <= data.len() {
        Some(
            u64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ])
        )
    } else {
        None
    }
}

fn try_extract_fields(
    data: &[u8],
    offset_pattern: &str,
    start_offset: usize
) -> Option<(String, String, String, String, String)> {
    let mut offset = start_offset;

    if
        let (Some(field1), Some(field2), Some(field3), Some(field4), Some(field5)) = (
            read_pubkey(data, offset),
            read_pubkey(data, offset + 32),
            read_pubkey(data, offset + 64),
            read_pubkey(data, offset + 96),
            read_pubkey(data, offset + 128),
        )
    {
        Some((field1, field2, field3, field4, field5))
    } else {
        None
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <pool_address>", args[0]);
        std::process::exit(1);
    }

    // Initialize RPC
    init_rpc_client().unwrap();
    let rpc = screenerbot::rpc::get_rpc_client();

    println!("🔍 PUMPFUN STRUCTURE ANALYZER");
    println!("=============================");
    println!("Pool address: {}\n", args[1]);

    let pool_pk = parse_pubkey(&args[1])?;
    let pool_acc = rpc.client().get_account(&pool_pk)?;

    println!("📦 POOL ACCOUNT INFO");
    println!("Owner: {}", pool_acc.owner);
    println!("Data size: {} bytes", pool_acc.data.len());
    println!("Owner is PumpFun AMM: {}\n", if pool_acc.owner.to_string() == PUMP_FUN_AMM_PROGRAM_ID {
        "✅"
    } else {
        "❌"
    });

    if pool_acc.data.len() < 200 {
        println!("❌ Data too short for PumpFun pool");
        return Ok(());
    }

    // Try different offset patterns to understand the structure
    println!("🔍 TRYING DIFFERENT STRUCTURE PATTERNS");
    println!("=====================================");

    // Pattern 1: Standard (discriminator + bump + index + creator + base + quote + lp + vault1 + vault2)
    println!(
        "\n📋 Pattern 1: disc(8) + bump(1) + index(2) + creator(32) + base(32) + quote(32) + lp(32) + vault1(32) + vault2(32)"
    );
    if
        let Some((base, quote, lp, vault1, vault2)) = try_extract_fields(
            &pool_acc.data,
            "standard",
            43
        )
    {
        println!("  Base mint: {}", base);
        println!("  Quote mint: {}", quote);
        println!("  LP mint: {}", lp);
        println!("  Vault 1: {}", vault1);
        println!("  Vault 2: {}", vault2);
        println!("  SOL detection: Base={}, Quote={}", base == SOL_MINT, quote == SOL_MINT);
    }

    // Pattern 2: With duplicate creator (discriminator + bump + index + creator + creator + base + quote + lp + vault1 + vault2)
    println!(
        "\n📋 Pattern 2: disc(8) + bump(1) + index(2) + creator(32) + creator(32) + base(32) + quote(32) + lp(32) + vault1(32) + vault2(32)"
    );
    if
        let Some((base, quote, lp, vault1, vault2)) = try_extract_fields(
            &pool_acc.data,
            "duplicate_creator",
            75
        )
    {
        println!("  Base mint: {}", base);
        println!("  Quote mint: {}", quote);
        println!("  LP mint: {}", lp);
        println!("  Vault 1: {}", vault1);
        println!("  Vault 2: {}", vault2);
        println!("  SOL detection: Base={}, Quote={}", base == SOL_MINT, quote == SOL_MINT);
    }

    // Pattern 3: Reverse field order
    println!("\n📋 Pattern 3: quote(32) + base(32) + lp(32) + vault1(32) + vault2(32)");
    if
        let Some((quote, base, lp, vault1, vault2)) = try_extract_fields(
            &pool_acc.data,
            "reverse",
            43
        )
    {
        println!("  Base mint: {}", base);
        println!("  Quote mint: {}", quote);
        println!("  LP mint: {}", lp);
        println!("  Vault 1: {}", vault1);
        println!("  Vault 2: {}", vault2);
        println!("  SOL detection: Base={}, Quote={}", base == SOL_MINT, quote == SOL_MINT);
    }

    // Pattern 4: Different starting offset
    println!("\n📋 Pattern 4: Starting at offset 11 (disc(8) + bump(1) + index(2))");
    if
        let Some((field1, field2, field3, field4, field5)) = try_extract_fields(
            &pool_acc.data,
            "offset_11",
            11
        )
    {
        println!("  Field 1: {}", field1);
        println!("  Field 2: {}", field2);
        println!("  Field 3: {}", field3);
        println!("  Field 4: {}", field4);
        println!("  Field 5: {}", field5);
        println!(
            "  SOL detection: F1={}, F2={}, F3={}, F4={}, F5={}",
            field1 == SOL_MINT,
            field2 == SOL_MINT,
            field3 == SOL_MINT,
            field4 == SOL_MINT,
            field5 == SOL_MINT
        );
    }

    // Show raw hex for manual analysis
    println!("\n📄 RAW HEX DATA (first 320 bytes)");
    println!("==================================");
    for (i, chunk) in pool_acc.data.chunks(16).take(20).enumerate() {
        print!("{:04x}: ", i * 16);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        // Also show as ASCII
        print!(" |");
        for byte in chunk {
            if *byte >= 32 && *byte <= 126 {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }

    // Let's also check what's at each 32-byte boundary
    println!("\n🔍 32-BYTE BOUNDARY ANALYSIS");
    println!("============================");
    for i in 0..10 {
        let offset = i * 32;
        if let Some(pubkey_str) = read_pubkey(&pool_acc.data, offset) {
            println!("Offset {}: {}", offset, pubkey_str);
            if pubkey_str == SOL_MINT {
                println!("  ^ SOL MINT FOUND!");
            }
        }
    }

    println!("\n✅ Analysis complete");
    Ok(())
}
