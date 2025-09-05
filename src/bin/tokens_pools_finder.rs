use std::collections::{ HashMap, HashSet };

use screenerbot::logger::{ log, LogTag };
use screenerbot::rpc::get_rpc_client;
use screenerbot::tokens::pool_old::{
    get_pool_program_display_name,
    PoolInfo,
    PoolPriceCalculator,
    METEORA_DAMM_V2_PROGRAM_ID,
    METEORA_DLMM_PROGRAM_ID,
    RAYDIUM_CPMM_PROGRAM_ID,
    RAYDIUM_CLMM_PROGRAM_ID,
    RAYDIUM_LEGACY_AMM_PROGRAM_ID,
};

/// Simple program descriptor for memcmp scanning
struct ProgramScan {
    id: &'static str,
    name: &'static str,
    // Offsets within account data where token mints are stored (run separate queries per offset)
    mint_offsets: &'static [usize],
}

// Known AMM programs and mint offsets derived from on-chain decoders in tokens/pool.rs
// Notes:
// - Raydium CPMM: mints at offsets 168 and 200 (after 8-byte discriminator + 5 pubkeys)
// - Raydium Legacy AMM: mints at 0x190 (400) and 0x1b0 (432)
// - Meteora DAMM v2: mints at 136 and 168
// - Meteora DLMM: mints at 88 and 120
// Orca Whirlpool & Pump.fun are not scanned here due to uncertain stable offsets for memcmp.
const PROGRAMS: &[ProgramScan] = &[
    ProgramScan {
        id: RAYDIUM_CPMM_PROGRAM_ID,
        name: "RAYDIUM CPMM",
        mint_offsets: &[168, 200],
    },
    ProgramScan {
        id: RAYDIUM_CLMM_PROGRAM_ID,
        name: "RAYDIUM CLMM",
        mint_offsets: &[73, 105], // mintA at 73, mintB at 105
    },
    ProgramScan {
        id: RAYDIUM_LEGACY_AMM_PROGRAM_ID,
        name: "RAYDIUM LEGACY AMM",
        mint_offsets: &[400, 432],
    },
    ProgramScan {
        id: METEORA_DAMM_V2_PROGRAM_ID,
        name: "METEORA DAMM v2",
        mint_offsets: &[136, 168],
    },
    ProgramScan {
        id: METEORA_DLMM_PROGRAM_ID,
        name: "METEORA DLMM",
        mint_offsets: &[88, 120],
    },
];

#[derive(Clone, Debug)]
struct FoundPool {
    pubkey: String,
    program_id: String,
}

async fn scan_program_for_mint(
    program: &ProgramScan,
    mint: &str
) -> Result<Vec<FoundPool>, String> {
    let rpc = get_rpc_client(); // Now using premium RPC due to FORCE_PREMIUM_RPC_ONLY=true

    log(
        LogTag::Pool,
        "SCAN_START",
        &format!(
            "Scanning {} for mint {}",
            program.name,
            screenerbot::utils::safe_truncate(mint, 12)
        )
    );

    let mut results = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for &offset in program.mint_offsets {
        log(
            LogTag::Pool,
            "MEMCMP",
            &format!(
                "Checking {} at offset {} for mint {}",
                program.name,
                offset,
                screenerbot::utils::safe_truncate(mint, 12)
            )
        );

        // Build memcmp filter to match the mint at the given offset
        let filters = serde_json::json!([
			{ "memcmp": { "offset": offset, "bytes": mint } }
		]);

        log(
            LogTag::Pool,
            "RPC_CALL",
            &format!("getProgramAccounts for {} with filter: {}", program.name, filters)
        );

        let accounts = rpc
            .get_program_accounts(program.id, Some(filters), Some("base64"), Some(45)).await
            .map_err(|e| format!("getProgramAccounts error for {}: {:?}", program.name, e))?;

        log(
            LogTag::Pool,
            "RPC_RESULT",
            &format!("{} returned {} accounts for offset {}", program.name, accounts.len(), offset)
        );

        for acc in accounts {
            if let Some(pubkey) = acc.get("pubkey").and_then(|v| v.as_str()) {
                if seen.insert(pubkey.to_string()) {
                    log(
                        LogTag::Pool,
                        "FOUND",
                        &format!(
                            "Found pool {} in {}",
                            screenerbot::utils::safe_truncate(pubkey, 12),
                            program.name
                        )
                    );
                    results.push(FoundPool {
                        pubkey: pubkey.to_string(),
                        program_id: program.id.to_string(),
                    });
                }
            }
        }
    }

    log(
        LogTag::Pool,
        "SCAN_DONE",
        &format!(
            "Finished scanning {} for mint {}: found {} pools",
            program.name,
            screenerbot::utils::safe_truncate(mint, 12),
            results.len()
        )
    );

    Ok(results)
}

async fn decode_and_filter(
    pool_addresses: &[(String, String)],
    target_mints: &HashSet<String>
) -> Vec<PoolInfo> {
    let svc = PoolPriceCalculator::new();
    let mut decoded = Vec::new();

    for (addr, program_id) in pool_addresses.iter() {
        match svc.get_pool_info(addr).await {
            Ok(Some(info)) => {
                // Filter to pools containing any of the target mints
                if
                    target_mints.contains(&info.token_0_mint) ||
                    target_mints.contains(&info.token_1_mint)
                {
                    decoded.push(info);
                } else {
                    // In rare cases, memcmp yielded false positives; ignore
                    log(
                        LogTag::Pool,
                        "FILTER",
                        &format!(
                            "Discarding pool {} ({}): token mints don't match provided targets",
                            screenerbot::utils::safe_truncate(addr, 12),
                            get_pool_program_display_name(program_id)
                        )
                    );
                }
            }
            Ok(None) => {
                log(
                    LogTag::Pool,
                    "DECODE_SKIP",
                    &format!(
                        "No decodable pool info for {} ({})",
                        screenerbot::utils::safe_truncate(addr, 12),
                        get_pool_program_display_name(program_id)
                    )
                );
            }
            Err(e) => {
                log(
                    LogTag::Pool,
                    "DECODE_ERR",
                    &format!(
                        "Failed to decode pool {} ({}): {}",
                        screenerbot::utils::safe_truncate(addr, 12),
                        get_pool_program_display_name(program_id),
                        e
                    )
                );
            }
        }
    }

    decoded
}

fn print_pools(pools: &[PoolInfo]) {
    println!("found_pools={}", pools.len());
    for p in pools {
        println!(
            "pool_address={}\nprogram_id={}\npool_type={}\ntoken_0_mint={}\ntoken_1_mint={}\ntoken_0_vault={:?}\ntoken_1_vault={:?}\nreserves=({}, {})\ndecimals=({}, {})\n---",
            p.pool_address,
            p.pool_program_id,
            p.pool_type,
            p.token_0_mint,
            p.token_1_mint,
            p.token_0_vault,
            p.token_1_vault,
            p.token_0_reserve,
            p.token_1_reserve,
            p.token_0_decimals,
            p.token_1_decimals
        );
    }
}

#[tokio::main]
async fn main() {
    // Inputs: one or more token mint addresses as positional args
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: tokens_pools_finder <TOKEN_MINT> [TOKEN_MINT ...]");
        std::process::exit(1);
    }

    // Normalize target mints to set
    let target_mints: HashSet<String> = args
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Kick the global RPC client (reads configs, sets up premium URL etc.)
    let _ = get_rpc_client();

    log(
        LogTag::Pool,
        "START",
        &format!("Scanning on-chain pools for {} mint(s)...", target_mints.len())
    );

    // 1) Scan programs via memcmp filters for each target mint
    let mut found: HashMap<String, FoundPool> = HashMap::new();

    for mint in &target_mints {
        for program in PROGRAMS {
            match scan_program_for_mint(program, mint).await {
                Ok(mut pools) => {
                    for fp in pools.drain(..) {
                        found.entry(fp.pubkey.clone()).or_insert(fp);
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "SCAN_ERR",
                        &format!(
                            "{}: failed scanning for mint {}: {}",
                            program.name,
                            screenerbot::utils::safe_truncate(mint, 8),
                            e
                        )
                    );
                }
            }
        }
    }

    if found.is_empty() {
        println!("No pools found on-chain for provided mint(s).");
        return;
    }

    log(
        LogTag::Pool,
        "SCAN_DONE",
        &format!("Discovered {} candidate pool accounts. Decoding...", found.len())
    );

    // 2) Decode discovered pools and filter by target mints
    let addrs: Vec<(String, String)> = found
        .values()
        .map(|fp| (fp.pubkey.clone(), fp.program_id.clone()))
        .collect();

    let decoded = decode_and_filter(&addrs, &target_mints).await;

    // 3) Print results
    print_pools(&decoded);
}
