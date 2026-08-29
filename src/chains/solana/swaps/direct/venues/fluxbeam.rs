//! FluxBeam AMM (`FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X`) — the ONLY venue
//! in this engine with no on-chain Anchor IDL, because it is not Anchor: it is a
//! fork of the well-known open-source `spl-token-swap` reference programme,
//! dispatched by a single leading instruction-tag BYTE rather than an 8-byte
//! Anchor discriminator.
//!
//! # Method: reference implementation as HYPOTHESIS, chain as PROOF
//!
//! `spl-token-swap`'s `SwapVersion`/`SwapV1` account layout and its `Swap`
//! instruction (tag `1`, args `amount_in: u64, minimum_amount_out: u64`) are
//! public and stable. That was the starting hypothesis here, not the answer —
//! every offset, the discriminator, the curve and the fee model below were
//! confirmed against a real pool account and two real mainnet buys (different
//! pools), one of which was replayed to the RAW UNIT against the vault deltas
//! `meta.preTokenBalances`/`postTokenBalances` actually recorded. See the field
//! comments below for the exact transactions and accounts.
//!
//! # `SwapV1`, 324 bytes, ONE extra leading byte the vanilla struct does not have
//!
//! ```text
//!  0 version u8        1 is_initialized bool   2 bump_seed u8
//!  3 token_program_id (legacy, vestigial -- see below)
//! 35 token_a_vault     67 token_b_vault        99 pool_mint
//! 131 token_a_mint     163 token_b_mint        195 pool_fee_account
//! 227 fees: trade_fee_numerator/denominator, owner_trade_fee_numerator/
//!     denominator, owner_withdraw_fee_numerator/denominator, host_fee_numerator/
//!     denominator -- eight u64s, 64 bytes
//! 291 curve_type u8    292 curve_parameters (32 bytes, unused here)
//! ```
//! `3 + 1 + 1 + 32*8 + 64 + 1 + 32 == 324`, the real account length, confirmed
//! against pool `7uajENggf2MaiZ5XGff91uoVsch1y5QN3bqjisv7eP6V` (a SOL /
//! Token-2022 pool) AND `5oA8PtzRTkvw8qfY6GWaaEGBfXscut25JtZq5XHLCHwi`. Both
//! decode `version == 1`, `is_initialized == true`, and `pool_fee_account`
//! matching the exact address a real swap passed as its fee account. The
//! existing PRICE decoder at `pools/decoders/fluxbeam_amm.rs` reads
//! `token_a_vault@35`, `token_b_vault@67`, `token_a_mint@131`,
//! `token_b_mint@163` -- all four CONFIRMED correct here independently.
//!
//! `token_program_id@3` is a single, pool-wide field left over from the vanilla
//! struct. On the confirmed Token-2022 pool it read the Token-2022 programme
//! id even though `token_a` (SOL) is a LEGACY mint -- it does not describe
//! either side reliably, so this venue never reads it. Each side's real token
//! programme is read from the MINT ACCOUNT's own owner (`load()`'s batch),
//! exactly like `raydium_cpmm.rs`.
//!
//! No status/pause bit exists anywhere in these 324 bytes (every byte between
//! the header and the curve is accounted for above) -- tradability is just
//! `is_initialized` plus non-empty reserves once the curve type is one this
//! venue understands.
//!
//! # The authority PDA: `find_program_address(&[pool], programme)`, bump 255
//!
//! Confirmed to decode the stored `bump_seed` byte (255 on both pools above)
//! and to independently RE-DERIVE the exact account a live swap named as its
//! `authority`/vault-owner: `5WCAmQDfnpfYDcNnCbcpf69tHVVLwnTWs1QGae145VPg` for
//! the first pool, `6NahzTketWh1HSZxc5zJgGXrqJfnW2FUVMQzDPE8kE9v` for the
//! second. This is the vanilla `spl-token-swap` authority derivation (seed is
//! just the pool's own pubkey), unmodified by the fork.
//!
//! # The curve: vanilla `ConstantProductCurve`, replayed to the exact raw unit
//!
//! `curve_type == 0` (`ConstantProduct`) on every pool seen; any other value is
//! refused rather than guessed at, since only this one was replayed. Both fee
//! numerator/denominator pairs are charged on the INPUT, before the curve runs
//! (this is the vanilla programme's own `Fees::trading_fee` +
//! `Fees::owner_trading_fee`, both computed off the caller's `amount_in`):
//!
//! ```text
//! trade_fee  = ceil(amount_in * trade_fee_numerator  / trade_fee_denominator)
//! owner_fee  = ceil(amount_in * owner_trade_fee_numerator / owner_trade_fee_denominator)
//! source_less_fees = amount_in - trade_fee - owner_fee
//! new_source = reserve_in + source_less_fees
//! new_dest   = ceil(reserve_out * reserve_in / new_source)
//! output     = reserve_out - new_dest
//! ```
//!
//! Replayed EXACTLY against a real buy on pool `7uajENgg…`
//! (`3S8k6zhDzADZ4NLn9kguFvFnUmw9PXCA83qNPL92AzTMNvQT4BEiCYYm6QypNniuM8gPB6Nw6krr18YZ3DfioJEm`):
//! `reserve_a (SOL) = 46_507_309_993`, `reserve_b (token) = 987_683_727_042_899_679`,
//! `amount_in = 330_352_000`, `trade_fee_numerator/denominator = 20/10_000`,
//! `owner_trade_fee_numerator/denominator = 99/100`. The formula above gives
//! `output = 56_122_747_909_206`, which is EXACTLY the amount the wallet's own
//! token account received (it immediately burned that exact figure in the same
//! transaction, giving a second independent confirmation of the same number).
//! The SOL vault's own balance rose by the FULL `amount_in` (not
//! `source_less_fees`) -- the fee split is a notional accounting split inside
//! the curve calculation, not a partial transfer, so `load()` reads the real
//! vault balance as the reserve with no haircut, unlike Raydium/CPMM's
//! `need_take_pnl`/`protocol_fee` subtraction. `owner_fee` is compensated
//! separately by newly-MINTED pool (LP) tokens credited to `pool_fee_account`
//! (confirmed: the fee account's own LP balance rose by exactly the trade's
//! diluted share on the same transaction) -- it is never physically removed
//! from the swap amount the trader loses, so [`VenueQuote::lp_fee`] is simply
//! `trade_fee + owner_fee`, already in INPUT raw units with no output-side
//! conversion needed (unlike `meteora_damm`/`meteora_dbc`/`moonit`, which all
//! charge on the output and must convert back).
//!
//! Both numerator/denominator pairs are read from the POOL's own account, never
//! hardcoded: the two pools replayed here carry the identical `20/10_000` and
//! `99/100` rates, but nothing in the layout says every pool must.
//!
//! # Instruction — `Swap`, tag `1`, 17 bytes, 14 accounts
//!
//! `[1u8][amount_in: u64 LE][minimum_amount_out: u64 LE]`, confirmed against
//! three real buys across two pools (decoded tag, byte length and argument
//! values all matched the transaction's own effect on the vaults). Tag `3`
//! (`WithdrawAllTokenTypes`, 25 bytes / 3 args) was also observed on the same
//! programme id and decodes cleanly under the vanilla enum ordering, which is
//! independent corroboration that the fork kept the vanilla dispatcher.
//!
//! ```text
//! 0 pool (swap account, writable)  1 authority          2 owner (signer)
//! 3 source token account           4 swap_source_vault  5 swap_destination_vault
//! 6 destination token account      7 pool_mint          8 fee_account
//! 9 source_mint                    10 destination_mint
//! 11 source token program          12 destination token program
//! 13 pool_mint token program
//! ```
//! No `host_fee` account is passed in any observed transaction (all real pools
//! here carry `host_fee_numerator == 0`); this venue never builds one.
//!
//! **Every slot that names a side is SWAP-ordered — the INPUT side first**, the
//! vanilla `spl-token-swap` contract, for 9-13 exactly as for 3-6.
//!
//! That is worth stating loudly because a same-orientation pool CANNOT prove
//! it, and this venue was first written pool-ordered (`mint_a`, `mint_b`,
//! `program_a`, `program_pool`, `program_b`) on the strength of buys that all
//! had SOL as `token_a`. On such a pool the two hypotheses coincide for the
//! mints, and on the fixture pool they coincided for the PROGRAMMES too, purely
//! because its `pool_mint` and its `token_b` are both Token-2022 — so
//! `(program_a, program_pool, program_b)` happened to equal
//! `(source, destination, pool)` value-for-value. A live buy simulated clean
//! and the error was invisible.
//!
//! What exposes it is a pool whose SOL side is **token B**, where swap order and
//! pool order disagree on both the mints and the programmes:
//! `82Gxnc1ubRPWKn8nQRRb45KhBKJ15LoxtQ9rRnWPPUSq` rejects the pool-ordered list
//! with `custom program error: 0x18`, "the provided token program does not
//! match the token program expected by the swap". Under the pool-ordered build
//! every SELL, and every buy on a SOL-as-token-B pool, would have failed in
//! production. `fluxbeam_accepts_a_swap_whose_sol_side_is_token_b` is that
//! discriminator, and it costs nothing to run.
//!
//! # Token-2022: structurally supported, transfer-fee mints refused
//!
//! Both replayed pools mix a legacy SOL vault with a Token-2022 custom-token
//! vault, and the account list above supports that (three independent
//! token-programme slots). What is NOT verified is how this fork's naive
//! `amount_in`-based fee split interacts with a Token-2022 mint that ALSO
//! charges its own transfer fee: the vanilla curve computes off the caller's
//! declared `amount_in`, but if the input mint's transfer fee shrank what
//! actually lands in the vault, the curve's assumed reserve growth would be
//! wrong. Neither replayed pool's mints charge a transfer fee, so this could
//! not be checked against a real trade. Rather than guess, `load()` REFUSES
//! any pool where either mint's `TransferFeeConfig` carries a non-zero rate
//! (`PoolNotTradable`) -- the Token-2022 SIDE of the account list is still
//! fully exercised by every other pool, which is the actual common case.

use super::layout::{mint_decimals, pubkey_at, u64_at, u8_at};
use super::math::{fee_amount, mul_div_ceil, price_impact_pct};
use super::token2022::transfer_fee_schedule;
use crate::chains::solana::constants::FLUXBEAM_AMM_PROGRAM_ID;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::chains::solana::swaps::direct::error::{DirectSwapError, DirectSwapResult};
use crate::chains::solana::swaps::direct::venue::{
    PoolMarket, PoolVenue, SwapAccounts, VenueQuote,
};
use async_trait::async_trait;
use std::str::FromStr;

/// The vanilla `spl-token-swap` `Instruction::Swap` tag: a single leading byte,
/// not an 8-byte Anchor discriminator. Confirmed against three real buys.
const SWAP_TAG: u8 = 1;

/// `curve_type` byte for `ConstantProduct`, the only curve this venue's
/// formula was replayed against.
const CURVE_TYPE_CONSTANT_PRODUCT: u8 = 0;

/// Compute units this venue's swap needs. A real `Swap` CPI measured 72,384 CU
/// on the confirmed replay transaction (Token-2022 output leg, LP-token mint
/// for the owner fee, both included); this leaves headroom for a legacy-only
/// pool's slightly different CPI shape without over-requesting on every swap.
const COMPUTE_UNITS: u32 = 160_000;

/// The venue adapter.
pub struct FluxbeamVenue;

#[async_trait]
impl PoolVenue for FluxbeamVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::FluxbeamAmm
    }

    fn program_id(&self) -> Pubkey {
        fluxbeam_program_id()
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let state = FluxbeamPoolState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "FluxBeam SwapV1 state did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if !state.is_initialized {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "pool is not initialized".to_owned(),
            });
        }
        if state.curve_type != CURVE_TYPE_CONSTANT_PRODUCT {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!(
                    "curve_type {} is not ConstantProduct (0); this venue's formula was \
                     verified only for that curve",
                    state.curve_type
                ),
            });
        }

        // One batched read: both vaults, both mints, and the pool (LP) mint --
        // its own token programme is a real, separate account read, never
        // assumed from either trading mint's programme.
        let addresses = [
            state.vault_a,
            state.vault_b,
            state.mint_a,
            state.mint_b,
            state.pool_mint,
        ];
        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("FluxBeam pool accounts could not be read: {e}"),
            })?;

        let fetched = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let vault_a = fetched(0)?;
        let vault_b = fetched(1)?;
        let vault_a_balance =
            super::layout::token_account_amount(&vault_a.data).ok_or_else(|| {
                DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: "token_a vault is not a token account".to_owned(),
                }
            })?;
        let vault_b_balance =
            super::layout::token_account_amount(&vault_b.data).ok_or_else(|| {
                DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: "token_b vault is not a token account".to_owned(),
                }
            })?;

        let mint_a_account = fetched(2)?;
        let mint_b_account = fetched(3)?;
        let pool_mint_account = fetched(4)?;
        let decimals_a = mint_decimals(&mint_a_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_a_mint is not a mint account".to_owned(),
            }
        })?;
        let decimals_b = mint_decimals(&mint_b_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_b_mint is not a mint account".to_owned(),
            }
        })?;

        // Refuse rather than guess how this fork's amount_in-based fee split
        // interacts with a mint that ALSO charges its own transfer fee -- see
        // the module docs. Neither replayed pool exercised this.
        if let Some(schedule) = transfer_fee_schedule(mint_a_account) {
            if schedule.basis_points > 0 {
                return Err(DirectSwapError::PoolNotTradable {
                    pool: *pool,
                    detail: "token_a carries a Token-2022 transfer fee, unverified against this \
                             venue's fee math"
                        .to_owned(),
                });
            }
        }
        if let Some(schedule) = transfer_fee_schedule(mint_b_account) {
            if schedule.basis_points > 0 {
                return Err(DirectSwapError::PoolNotTradable {
                    pool: *pool,
                    detail: "token_b carries a Token-2022 transfer fee, unverified against this \
                             venue's fee math"
                        .to_owned(),
                });
            }
        }

        Ok(Box::new(FluxbeamMarket {
            state,
            vault_a_balance,
            vault_b_balance,
            decimals_a,
            decimals_b,
            program_a: mint_a_account.owner,
            program_b: mint_b_account.owner,
            program_pool: pool_mint_account.owner,
        }))
    }
}

/// The parts of a `SwapV1` account a swap needs. See the module docs for the
/// full byte-offset table and how it was confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluxbeamPoolState {
    pub pool: Pubkey,
    pub is_initialized: bool,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub pool_mint: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub fee_account: Pubkey,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub owner_trade_fee_numerator: u64,
    pub owner_trade_fee_denominator: u64,
    pub curve_type: u8,
}

impl FluxbeamPoolState {
    /// Decode a `SwapV1` account. Pure: no RPC, no cache, no clock. Layout
    /// confirmed byte-for-byte against two live pools -- see the module docs.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        if data.len() != 324 {
            return None;
        }
        Some(Self {
            pool,
            is_initialized: u8_at(data, 1)? != 0,
            vault_a: pubkey_at(data, 35)?,
            vault_b: pubkey_at(data, 67)?,
            pool_mint: pubkey_at(data, 99)?,
            mint_a: pubkey_at(data, 131)?,
            mint_b: pubkey_at(data, 163)?,
            fee_account: pubkey_at(data, 195)?,
            trade_fee_numerator: u64_at(data, 227)?,
            trade_fee_denominator: u64_at(data, 235)?,
            owner_trade_fee_numerator: u64_at(data, 243)?,
            owner_trade_fee_denominator: u64_at(data, 251)?,
            curve_type: u8_at(data, 291)?,
        })
    }
}

/// A decoded, quotable FluxBeam pool.
#[derive(Debug, Clone)]
pub struct FluxbeamMarket {
    state: FluxbeamPoolState,
    vault_a_balance: u64,
    vault_b_balance: u64,
    decimals_a: u8,
    decimals_b: u8,
    program_a: Pubkey,
    program_b: Pubkey,
    program_pool: Pubkey,
}

impl FluxbeamMarket {
    /// Build a market directly from decoded parts. The offline test tier uses
    /// this to drive real captured state without any RPC.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: FluxbeamPoolState,
        vault_a_balance: u64,
        vault_b_balance: u64,
        decimals_a: u8,
        decimals_b: u8,
        program_a: Pubkey,
        program_b: Pubkey,
        program_pool: Pubkey,
    ) -> Self {
        Self {
            state,
            vault_a_balance,
            vault_b_balance,
            decimals_a,
            decimals_b,
            program_a,
            program_b,
            program_pool,
        }
    }

    /// Whether `mint` is the pool's token_a side.
    fn is_side_a(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.state.mint_a {
            Some(true)
        } else if *mint == self.state.mint_b {
            Some(false)
        } else {
            None
        }
    }
}

impl PoolMarket for FluxbeamMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::FluxbeamAmm
    }

    fn pool(&self) -> Pubkey {
        self.state.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.state.mint_a, self.state.mint_b)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_side_a(mint).map(|side_a| {
            if side_a {
                self.program_a
            } else {
                self.program_b
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_side_a(mint).map(|side_a| {
            if side_a {
                self.decimals_a
            } else {
                self.decimals_b
            }
        })
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_a = self
            .is_side_a(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;

        let (reserve_in, reserve_out) = if input_is_a {
            (self.vault_a_balance, self.vault_b_balance)
        } else {
            (self.vault_b_balance, self.vault_a_balance)
        };
        if reserve_in == 0 || reserve_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "a vault side is empty".to_owned(),
            });
        }

        // Both fee components are charged on the INPUT, before the curve runs
        // -- the vanilla programme's own `trading_fee` + `owner_trading_fee`,
        // replayed exactly in the module docs.
        let trade_fee = fee_amount(
            amount_in,
            self.state.trade_fee_numerator,
            self.state.trade_fee_denominator,
        );
        let owner_fee = fee_amount(
            amount_in,
            self.state.owner_trade_fee_numerator,
            self.state.owner_trade_fee_denominator,
        );
        let lp_fee = trade_fee.saturating_add(owner_fee).min(amount_in);
        let source_less_fees = amount_in.saturating_sub(lp_fee);

        if source_less_fees == 0 {
            return Ok(VenueQuote {
                amount_in,
                expected_out: 0,
                lp_fee,
                price_impact_pct: 0.0,
            });
        }

        let new_source = (reserve_in as u128).saturating_add(source_less_fees as u128);
        let new_dest = mul_div_ceil(reserve_out as u128, reserve_in as u128, new_source)
            .ok_or_else(|| DirectSwapError::QuoteMath {
                detail: "FluxBeam curve division overflowed".to_owned(),
            })?;
        if new_dest > reserve_out as u128 {
            // The curve can never ask the pool to owe more than it holds; a
            // result above the reserve means the arithmetic is not trustworthy
            // for this size rather than a real, if surprising, quote.
            return Err(DirectSwapError::QuoteMath {
                detail: "FluxBeam curve produced an output above the pool's own reserve".to_owned(),
            });
        }
        let expected_out = reserve_out - new_dest as u64;

        Ok(VenueQuote {
            amount_in,
            expected_out,
            lp_fee,
            price_impact_pct: price_impact_pct(reserve_in, reserve_out, amount_in, expected_out),
        })
    }

    fn swap_instruction(
        &self,
        accounts: &SwapAccounts,
        amount_in: u64,
        min_out: u64,
    ) -> DirectSwapResult<Instruction> {
        let input_is_a =
            self.is_side_a(&accounts.input_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_a = self.is_side_a(&accounts.output_mint).ok_or_else(|| {
            DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            }
        })?;
        if input_is_a == output_is_a {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        // Accounts 3-6 AND 9-13 are all SWAP-ordered: everything that names a
        // side names the INPUT side first. Confirmed on a pool whose SOL side
        // is token B, where swap order and pool order do not coincide -- see
        // the module docs.
        let (swap_source_vault, swap_destination_vault) = if input_is_a {
            (self.state.vault_a, self.state.vault_b)
        } else {
            (self.state.vault_b, self.state.vault_a)
        };
        let (source_mint, destination_mint) = if input_is_a {
            (self.state.mint_a, self.state.mint_b)
        } else {
            (self.state.mint_b, self.state.mint_a)
        };
        let (source_program, destination_program) = if input_is_a {
            (self.program_a, self.program_b)
        } else {
            (self.program_b, self.program_a)
        };

        let authority =
            Pubkey::find_program_address(&[self.state.pool.as_ref()], &fluxbeam_program_id()).0;

        let mut data = Vec::with_capacity(17);
        data.push(SWAP_TAG);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&min_out.to_le_bytes());

        Ok(Instruction {
            program_id: fluxbeam_program_id(),
            accounts: vec![
                AccountMeta::new(self.state.pool, false),
                AccountMeta::new_readonly(authority, false),
                AccountMeta::new(accounts.owner, true),
                AccountMeta::new(accounts.input_token_account, false),
                AccountMeta::new(swap_source_vault, false),
                AccountMeta::new(swap_destination_vault, false),
                AccountMeta::new(accounts.output_token_account, false),
                AccountMeta::new(self.state.pool_mint, false),
                AccountMeta::new(self.state.fee_account, false),
                AccountMeta::new_readonly(source_mint, false),
                AccountMeta::new_readonly(destination_mint, false),
                AccountMeta::new_readonly(source_program, false),
                AccountMeta::new_readonly(destination_program, false),
                AccountMeta::new_readonly(self.program_pool, false),
            ],
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

fn fluxbeam_program_id() -> Pubkey {
    Pubkey::from_str(FLUXBEAM_AMM_PROGRAM_ID).expect("FluxBeam program id constant is valid")
}
