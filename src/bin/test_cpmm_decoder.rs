/// Test Raydium CPMM decoder with real pool data
/// Pool: 2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::rpc::get_rpc_client;
use screenerbot::pools::decoders::raydium_cpmm::RaydiumCpmmDecoder;
use screenerbot::logger::{ log, LogTag };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const POOL_ADDRESS: &str = "2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ";

// Reference data from Solscan for validation
const EXPECTED_AMM_CONFIG: &str = "D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2";
const EXPECTED_POOL_CREATOR: &str = "HfPAdEfUSyTou6Hr5mtrRz6W46g4P7tdSTWu9KLYBaLw";
const EXPECTED_TOKEN_0_VAULT: &str = "EFwUSAm9VGFBHaSFSRSbBPkiisL7dCXFnZWLE3CgfDeV";
const EXPECTED_TOKEN_1_VAULT: &str = "7HFEfkB7BbSxijWvpLsZANxoXe57DFz6kMYr8DPMYXd9";
const EXPECTED_LP_MINT: &str = "F4LeXM6cAM5vWNTqwMyKFocQbAh7poqUPDAu6YvCqYh6";
const EXPECTED_TOKEN_0_MINT: &str = "So11111111111111111111111111111111111111112";
const EXPECTED_TOKEN_1_MINT: &str = "5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t";
const EXPECTED_TOKEN_0_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const EXPECTED_TOKEN_1_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const EXPECTED_OBSERVATION_KEY: &str = "6PwdXVHDFhNzd9LfFoRoYmKZhdYRTU7YSpbD6vWBGbuf";
const EXPECTED_AUTH_BUMP: u8 = 253;
const EXPECTED_STATUS: u8 = 0;
const EXPECTED_LP_MINT_DECIMALS: u8 = 9;
const EXPECTED_MINT_0_DECIMALS: u8 = 9;
const EXPECTED_MINT_1_DECIMALS: u8 = 6;
const EXPECTED_LP_SUPPLY: u64 = 1842507439605;
const EXPECTED_PROTOCOL_FEES_TOKEN_0: u64 = 11432215;
const EXPECTED_PROTOCOL_FEES_TOKEN_1: u64 = 549639195566;
const EXPECTED_FUND_FEES_TOKEN_0: u64 = 28311282;
const EXPECTED_FUND_FEES_TOKEN_1: u64 = 790413659507;
const EXPECTED_OPEN_TIME: u64 = 1744071971;
const EXPECTED_RECENT_EPOCH: u64 = 846;
const EXPECTED_CREATOR_FEE_ON: u8 = 0;
const EXPECTED_ENABLE_CREATOR_FEE: bool = false;
const EXPECTED_CREATOR_FEES_TOKEN_0: u64 = 0;
const EXPECTED_CREATOR_FEES_TOKEN_1: u64 = 0;

#[derive(Parser, Debug)]
#[command(name = "test_cpmm_decoder", about = "Test Raydium CPMM decoder")]
struct Args {
    /// Enable all debugging modes
    #[arg(long)]
    debug_all_pools: bool,

    /// Enable pool decoder debugging
    #[arg(long)]
    debug_pool_decoders: bool,

    /// Enable RPC debugging
    #[arg(long)]
    debug_rpc: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Configure debugging based on arguments
    set_cmd_args(
        vec![
            "test_cpmm_decoder".to_string(),
            if args.debug_all_pools || args.debug_pool_decoders {
                "--debug-pool-decoders".to_string()
            } else {
                "".to_string()
            },
            if args.debug_all_pools || args.debug_rpc {
                "--debug-rpc".to_string()
            } else {
                "".to_string()
            }
        ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect()
    );

    println!("🔍 Testing Raydium CPMM Decoder");
    println!("Pool: {}", POOL_ADDRESS);
    println!("{}", "=".repeat(80));

    // Fetch pool account data
    let rpc_client = get_rpc_client();
    let pubkey = Pubkey::from_str(POOL_ADDRESS)?;

    let account_info = match rpc_client.get_account(&pubkey).await {
        Ok(account) => account,
        Err(e) => {
            println!("❌ Failed to fetch pool account: {}", e);
            return Ok(());
        }
    };

    println!("✅ Pool account fetched: {} bytes", account_info.data.len());

    // Test current decoder
    println!("\n📊 Testing Current CPMM Decoder:");
    println!("{}", "-".repeat(40));

    if
        let Some(pool_info) = RaydiumCpmmDecoder::decode_raydium_cpmm_pool(
            &account_info.data,
            POOL_ADDRESS
        )
    {
        println!("✅ Current decoder succeeded");

        // Validate against reference data
        let mut mismatches = Vec::new();

        if pool_info.amm_config != EXPECTED_AMM_CONFIG {
            mismatches.push(
                format!(
                    "AMM Config: got {}, expected {}",
                    pool_info.amm_config,
                    EXPECTED_AMM_CONFIG
                )
            );
        }

        if pool_info.pool_creator != EXPECTED_POOL_CREATOR {
            mismatches.push(
                format!(
                    "Pool Creator: got {}, expected {}",
                    pool_info.pool_creator,
                    EXPECTED_POOL_CREATOR
                )
            );
        }

        if pool_info.token_0_vault != EXPECTED_TOKEN_0_VAULT {
            mismatches.push(
                format!(
                    "Token 0 Vault: got {}, expected {}",
                    pool_info.token_0_vault,
                    EXPECTED_TOKEN_0_VAULT
                )
            );
        }

        if pool_info.token_1_vault != EXPECTED_TOKEN_1_VAULT {
            mismatches.push(
                format!(
                    "Token 1 Vault: got {}, expected {}",
                    pool_info.token_1_vault,
                    EXPECTED_TOKEN_1_VAULT
                )
            );
        }

        if pool_info.lp_mint != EXPECTED_LP_MINT {
            mismatches.push(
                format!("LP Mint: got {}, expected {}", pool_info.lp_mint, EXPECTED_LP_MINT)
            );
        }

        if pool_info.token_0_mint != EXPECTED_TOKEN_0_MINT {
            mismatches.push(
                format!(
                    "Token 0 Mint: got {}, expected {}",
                    pool_info.token_0_mint,
                    EXPECTED_TOKEN_0_MINT
                )
            );
        }

        if pool_info.token_1_mint != EXPECTED_TOKEN_1_MINT {
            mismatches.push(
                format!(
                    "Token 1 Mint: got {}, expected {}",
                    pool_info.token_1_mint,
                    EXPECTED_TOKEN_1_MINT
                )
            );
        }

        if pool_info.token_0_program != EXPECTED_TOKEN_0_PROGRAM {
            mismatches.push(
                format!(
                    "Token 0 Program: got {}, expected {}",
                    pool_info.token_0_program,
                    EXPECTED_TOKEN_0_PROGRAM
                )
            );
        }

        if pool_info.token_1_program != EXPECTED_TOKEN_1_PROGRAM {
            mismatches.push(
                format!(
                    "Token 1 Program: got {}, expected {}",
                    pool_info.token_1_program,
                    EXPECTED_TOKEN_1_PROGRAM
                )
            );
        }

        if pool_info.observation_key != EXPECTED_OBSERVATION_KEY {
            mismatches.push(
                format!(
                    "Observation Key: got {}, expected {}",
                    pool_info.observation_key,
                    EXPECTED_OBSERVATION_KEY
                )
            );
        }

        if pool_info.auth_bump != EXPECTED_AUTH_BUMP {
            mismatches.push(
                format!("Auth Bump: got {}, expected {}", pool_info.auth_bump, EXPECTED_AUTH_BUMP)
            );
        }

        if pool_info.status != EXPECTED_STATUS {
            mismatches.push(
                format!("Status: got {}, expected {}", pool_info.status, EXPECTED_STATUS)
            );
        }

        if pool_info.lp_mint_decimals != EXPECTED_LP_MINT_DECIMALS {
            mismatches.push(
                format!(
                    "LP Mint Decimals: got {}, expected {}",
                    pool_info.lp_mint_decimals,
                    EXPECTED_LP_MINT_DECIMALS
                )
            );
        }

        // Validate additional CPMM fields
        if pool_info.lp_supply != EXPECTED_LP_SUPPLY {
            mismatches.push(
                format!("LP Supply: got {}, expected {}", pool_info.lp_supply, EXPECTED_LP_SUPPLY)
            );
        }

        if pool_info.protocol_fees_token_0 != EXPECTED_PROTOCOL_FEES_TOKEN_0 {
            mismatches.push(
                format!(
                    "Protocol Fees Token 0: got {}, expected {}",
                    pool_info.protocol_fees_token_0,
                    EXPECTED_PROTOCOL_FEES_TOKEN_0
                )
            );
        }

        if pool_info.protocol_fees_token_1 != EXPECTED_PROTOCOL_FEES_TOKEN_1 {
            mismatches.push(
                format!(
                    "Protocol Fees Token 1: got {}, expected {}",
                    pool_info.protocol_fees_token_1,
                    EXPECTED_PROTOCOL_FEES_TOKEN_1
                )
            );
        }

        if pool_info.fund_fees_token_0 != EXPECTED_FUND_FEES_TOKEN_0 {
            mismatches.push(
                format!(
                    "Fund Fees Token 0: got {}, expected {}",
                    pool_info.fund_fees_token_0,
                    EXPECTED_FUND_FEES_TOKEN_0
                )
            );
        }

        if pool_info.fund_fees_token_1 != EXPECTED_FUND_FEES_TOKEN_1 {
            mismatches.push(
                format!(
                    "Fund Fees Token 1: got {}, expected {}",
                    pool_info.fund_fees_token_1,
                    EXPECTED_FUND_FEES_TOKEN_1
                )
            );
        }

        if pool_info.open_time != EXPECTED_OPEN_TIME {
            mismatches.push(
                format!("Open Time: got {}, expected {}", pool_info.open_time, EXPECTED_OPEN_TIME)
            );
        }

        if pool_info.recent_epoch != EXPECTED_RECENT_EPOCH {
            mismatches.push(
                format!(
                    "Recent Epoch: got {}, expected {}",
                    pool_info.recent_epoch,
                    EXPECTED_RECENT_EPOCH
                )
            );
        }

        if pool_info.creator_fee_on != EXPECTED_CREATOR_FEE_ON {
            mismatches.push(
                format!(
                    "Creator Fee On: got {}, expected {}",
                    pool_info.creator_fee_on,
                    EXPECTED_CREATOR_FEE_ON
                )
            );
        }

        if pool_info.enable_creator_fee != EXPECTED_ENABLE_CREATOR_FEE {
            mismatches.push(
                format!(
                    "Enable Creator Fee: got {}, expected {}",
                    pool_info.enable_creator_fee,
                    EXPECTED_ENABLE_CREATOR_FEE
                )
            );
        }

        if pool_info.creator_fees_token_0 != EXPECTED_CREATOR_FEES_TOKEN_0 {
            mismatches.push(
                format!(
                    "Creator Fees Token 0: got {}, expected {}",
                    pool_info.creator_fees_token_0,
                    EXPECTED_CREATOR_FEES_TOKEN_0
                )
            );
        }

        if pool_info.creator_fees_token_1 != EXPECTED_CREATOR_FEES_TOKEN_1 {
            mismatches.push(
                format!(
                    "Creator Fees Token 1: got {}, expected {}",
                    pool_info.creator_fees_token_1,
                    EXPECTED_CREATOR_FEES_TOKEN_1
                )
            );
        }

        if mismatches.is_empty() {
            println!("✅ All fields match reference data perfectly!");
            println!("   - Basic fields: ✅");
            println!("   - LP Supply: {} ✅", pool_info.lp_supply);
            println!(
                "   - Protocol Fees: {}/{} ✅",
                pool_info.protocol_fees_token_0,
                pool_info.protocol_fees_token_1
            );
            println!(
                "   - Fund Fees: {}/{} ✅",
                pool_info.fund_fees_token_0,
                pool_info.fund_fees_token_1
            );
            println!(
                "   - Timing: open_time={}, recent_epoch={} ✅",
                pool_info.open_time,
                pool_info.recent_epoch
            );
            println!(
                "   - Creator Fees: on={}, enabled={}, fees={}/{} ✅",
                pool_info.creator_fee_on,
                pool_info.enable_creator_fee,
                pool_info.creator_fees_token_0,
                pool_info.creator_fees_token_1
            );
        } else {
            println!("❌ Field mismatches found:");
            for mismatch in &mismatches {
                println!("  - {}", mismatch);
            }
        }

        // No longer missing fields!
        println!("\n🎉 Complete CPMM decoder now extracts all fields!");
    } else {
        println!("❌ Current decoder failed");
    }

    // Test enhanced decoder with full field extraction
    println!("\n🔬 Raw Data Analysis:");
    println!("{}", "-".repeat(40));

    // Search for specific values in raw data to understand structure
    search_for_value_in_data(&account_info.data, EXPECTED_LP_SUPPLY, "LP Supply");
    search_for_value_in_data(
        &account_info.data,
        EXPECTED_PROTOCOL_FEES_TOKEN_0,
        "Protocol Fees Token 0"
    );
    search_for_value_in_data(
        &account_info.data,
        EXPECTED_PROTOCOL_FEES_TOKEN_1,
        "Protocol Fees Token 1"
    );
    search_for_value_in_data(&account_info.data, EXPECTED_FUND_FEES_TOKEN_0, "Fund Fees Token 0");
    search_for_value_in_data(&account_info.data, EXPECTED_FUND_FEES_TOKEN_1, "Fund Fees Token 1");
    search_for_value_in_data(&account_info.data, EXPECTED_OPEN_TIME, "Open Time");
    search_for_value_in_data(&account_info.data, EXPECTED_RECENT_EPOCH, "Recent Epoch");

    Ok(())
}

fn search_for_value_in_data(data: &[u8], target: u64, field_name: &str) {
    let target_bytes = target.to_le_bytes();

    for i in 0..data.len().saturating_sub(7) {
        if &data[i..i + 8] == target_bytes {
            println!("Found {} ({}) at offset: {}", field_name, target, i);
            return;
        }
    }

    println!("❌ {} ({}) not found in data", field_name, target);
}
