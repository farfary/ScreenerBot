use screenerbot::pools::decoders::raydium_clmm::RaydiumClmmDecoder;
use screenerbot::pools::AccountData;
use screenerbot::rpc::get_rpc_client;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::collections::HashMap;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing CLMM Decoder with Real Pool Data");

    // Initialize RPC client
    let rpc_client = get_rpc_client();

    // Test pool address (CLMM pool)
    let pool_pubkey = Pubkey::from_str("HWek4aDnvgbBiDAGsJHN7JERv8sWbRnRa51KeoDff7xv")?;

    println!("📊 Fetching pool account data for: {}", pool_pubkey);

    // Fetch account data
    let account = rpc_client.get_account(&pool_pubkey).await?;

    println!("✅ Account fetched: {} bytes, owner: {}", account.data.len(), account.owner);

    let data_len = account.data.len();

    // Create AccountData structure
    let account_data = AccountData {
        pubkey: pool_pubkey,
        data: account.data,
        owner: account.owner,
        lamports: account.lamports,
        slot: 0,
        fetched_at: Instant::now(),
    };

    // Create accounts map
    let mut accounts = HashMap::new();
    accounts.insert(pool_pubkey.to_string(), account_data.clone());

    // Test the decoder
    match RaydiumClmmDecoder::extract_pool_data(&accounts) {
        Some(pool_info) => {
            println!("🎉 CLMM Pool Data Successfully Extracted!");
            println!("=== COMPARISON WITH SOLSCAN DATA ===");

            println!("   Pool ID: {}", pool_pubkey);
            println!("   Bump: {} (Solscan: 252)", pool_info.bump);
            println!(
                "   AMM Config: {} (Solscan: E64NGkDLLCdQ2yFNPcavaKptrEgmiQaNykUuLC1Qgwyp)",
                pool_info.amm_config
            );
            println!(
                "   Owner: {} (Solscan: HfPAdEfUSyTou6Hr5mtrRz6W46g4P7tdSTWu9KLYBaLw)",
                pool_info.owner
            );

            println!(
                "   Token Mint 0: {} (Solscan: So11111111111111111111111111111111111111112)",
                pool_info.token_mint_0
            );
            println!(
                "   Token Mint 1: {} (Solscan: 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t)",
                pool_info.token_mint_1
            );
            println!(
                "   Token Vault 0: {} (Solscan: 3CHdLG2ixohzG9MkcrN9Ar5Vnc9M3L3zpgd7wvfwkmd4)",
                pool_info.token_vault_0
            );
            println!(
                "   Token Vault 1: {} (Solscan: arzPwc4gAiqrTgYzDEZVE85pGnSxBEibdt67czNWw79)",
                pool_info.token_vault_1
            );
            println!(
                "   Observation Key: {} (Solscan: 7Zyr7wwAhyCc7a7YzPhq25aDwdCzb3o5o9YRsbvEUXCC)",
                pool_info.observation_key
            );

            println!(
                "   Decimals: {} / {} (Solscan: 9 / 6)",
                pool_info.mint_decimals_0,
                pool_info.mint_decimals_1
            );
            println!("   Tick Spacing: {} (Solscan: 60)", pool_info.tick_spacing);
            println!("   Liquidity: {} (Solscan: 10000000023)", pool_info.liquidity);
            println!(
                "   Sqrt Price X64: {} (Solscan: 6182269088572990140989)",
                pool_info.sqrt_price_x64
            );
            println!("   Tick Current: {} (Solscan: 116296)", pool_info.tick_current);
            println!("   Padding3: {} (Solscan: 0)", pool_info.padding3);
            println!("   Padding4: {} (Solscan: 0)", pool_info.padding4);

            println!(
                "   Fee Growth Global 0 X64: {} (Solscan: 62584157794465641)",
                pool_info.fee_growth_global_0_x64
            );
            println!(
                "   Fee Growth Global 1 X64: {} (Solscan: 327585634049132759451)",
                pool_info.fee_growth_global_1_x64
            );

            println!(
                "   Protocol Fees: {} / {} (Solscan: 1513857 / 12736120252)",
                pool_info.protocol_fees_token_0,
                pool_info.protocol_fees_token_1
            );

            println!(
                "   Swap In Amount Token 0: {} (Solscan: 16154977365)",
                pool_info.swap_in_amount_token_0
            );
            println!(
                "   Swap Out Amount Token 1: {} (Solscan: 81101235574256)",
                pool_info.swap_out_amount_token_1
            );
            println!(
                "   Swap In Amount Token 1: {} (Solscan: 84564060522666)",
                pool_info.swap_in_amount_token_1
            );
            println!(
                "   Swap Out Amount Token 0: {} (Solscan: 17084750755)",
                pool_info.swap_out_amount_token_0
            );

            println!("   Status: {} (Solscan: 0)", pool_info.status);

            println!("   Total Fees Token 0: {} (Solscan: 33926940)", pool_info.total_fees_token_0);
            println!(
                "   Total Fees Claimed Token 0: {} (Solscan: 33926931)",
                pool_info.total_fees_claimed_token_0
            );
            println!(
                "   Total Fees Token 1: {} (Solscan: 177584528464)",
                pool_info.total_fees_token_1
            );
            println!(
                "   Total Fees Claimed Token 1: {} (Solscan: 177584528455)",
                pool_info.total_fees_claimed_token_1
            );

            println!("   Fund Fees Token 0: {} (Solscan: 1614969)", pool_info.fund_fees_token_0);
            println!("   Fund Fees Token 1: {} (Solscan: 8456405596)", pool_info.fund_fees_token_1);

            println!("   Open Time: {} (Solscan: 0)", pool_info.open_time);
            println!("   Recent Epoch: {} (Solscan: 845)", pool_info.recent_epoch);

            println!("   Reward Infos: {} rewards (Solscan: 3)", pool_info.reward_infos.len());
            for (i, reward) in pool_info.reward_infos.iter().enumerate() {
                println!(
                    "     Reward {}: State={}, Token={}, Vault={}, Authority={}",
                    i,
                    reward.reward_state,
                    reward.token_mint,
                    reward.token_vault,
                    reward.authority
                );
            }

            println!(
                "   Tick Array Bitmap: {:?} (first few values)",
                &pool_info.tick_array_bitmap[0..8]
            );

            // Check for exact matches
            let mut mismatches = Vec::new();

            if pool_info.amm_config != "E64NGkDLLCdQ2yFNPcavaKptrEgmiQaNykUuLC1Qgwyp" {
                mismatches.push("AMM Config");
            }
            if pool_info.sqrt_price_x64 != 6182269088572990140989_u128 {
                mismatches.push("Sqrt Price X64");
            }
            if pool_info.liquidity != 10000000023_u128 {
                mismatches.push("Liquidity");
            }
            if pool_info.recent_epoch != 845 {
                mismatches.push("Recent Epoch");
            }

            if mismatches.is_empty() {
                println!("✅ All key fields match Solscan data!");
            } else {
                println!("❌ Mismatched fields: {:?}", mismatches);

                // Debug: search for recent_epoch=845 in the raw data
                println!("🔍 Searching raw data for recent_epoch=845:");
                let data = &account_data.data;
                for i in (0..data.len() - 8).step_by(8) {
                    let bytes: [u8; 8] = data[i..i + 8].try_into().unwrap();
                    let value = u64::from_le_bytes(bytes);
                    if value == 845 {
                        println!("   Found 845 at offset {} (expected recent_epoch)", i);
                    }
                    if value == 33926940 {
                        println!("   Found 33926940 at offset {} (expected total_fees_token_0)", i);
                    }
                    if value == 177584528464 {
                        println!("   Found 177584528464 at offset {} (expected total_fees_token_1)", i);
                    }
                }
            }
        }
        None => {
            println!("❌ Failed to decode CLMM pool data");
            println!("Account size: {} bytes", data_len);

            // Let's try a simpler approach - just check the first few fields
            println!("🔍 Debugging first few bytes:");
            let data = &account_data.data;
            if data.len() >= 100 {
                println!("   Discriminator: {:?}", &data[0..8]);
                println!("   Bump: {}", data[8]);

                // Try to extract just the first pubkey (amm_config)
                if data.len() >= 41 {
                    let amm_config_bytes: [u8; 32] = data[9..41].try_into().unwrap();
                    let amm_config = solana_sdk::pubkey::Pubkey::new_from_array(amm_config_bytes);
                    println!("   AMM Config: {}", amm_config);
                }

                // Check near the end for recent_epoch (should be 845)
                if data.len() >= 8 {
                    println!("🔍 Checking last 100 bytes for recent_epoch=845:");
                    for i in (data.len() - 100..data.len() - 8).step_by(8) {
                        let bytes: [u8; 8] = data[i..i + 8].try_into().unwrap();
                        let value = u64::from_le_bytes(bytes);
                        if value == 845 {
                            println!("   Found 845 at offset {}", i);
                        }
                        if i % 32 == 0 {
                            // Print every 4th value
                            println!("   Offset {}: {}", i, value);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
