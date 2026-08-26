//! Raydium CPMM (`CPMMoo8L…`) — the constant-product programme Raydium calls
//! "CP-Swap".
//!
//! # Layout, verified against mainnet
//!
//! `PoolState` is 637 bytes. Offsets used here were read back off a live pool
//! rather than taken from a header file:
//!
//! ```text
//!   8 amm_config      40 pool_creator    72 token_0_vault  104 token_1_vault
//! 136 lp_mint        168 token_0_mint   200 token_1_mint   232 token_0_program
//! 264 token_1_program 296 observation   328 auth_bump      329 status
//! 330 lp_decimals    331 mint_0_decimals 332 mint_1_decimals
//! 333 lp_supply      341 protocol_fee_0 349 protocol_fee_1
//! 357 fund_fee_0     365 fund_fee_1     373 open_time      381 recent_epoch
//! 389 creator_fee_on 390 enable_creator_fee
//! 397 creator_fee_0  405 creator_fee_1
//! ```
//!
//! `AmmConfig` is 236 bytes: `12 trade_fee_rate · 20 protocol_fee_rate ·
//! 28 fund_fee_rate · 36 create_pool_fee · 108 creator_fee_rate`, all over a
//! denominator of 1_000_000.
//!
//! Decimals come from the POOL STATE (offsets 331/332), never from a cache. The
//! price decoder for this same program refuses to decode when its decimals cache
//! is cold; a swap venue cannot afford that dependency, and the pool already
//! carries the answer.
//!
//! # The curve
//!
//! Tradable reserves are NOT the vault balances. Protocol, fund and creator fees
//! sit in the same vaults awaiting collection and are not swappable, so each side
//! is `vault − protocol_fee − fund_fee − creator_fee`. Quoting off the raw vault
//! over-states the reserve, which over-states the output, which produces a
//! `min_out` the pool cannot meet.
//!
//! Both Token-2022 transfer fees apply: on the way in the pool receives less than
//! we sent, and on the way out we receive less than the pool sent. The programme
//! compares `minimum_amount_out` against the amount received NET of the output
//! transfer fee, so the quote must be net too.

use super::layout::{pubkey_at, token_account_amount, u64_at, u8_at};
use super::math::{constant_product_out, fee_amount, price_impact_pct};
use super::token2022::{transfer_fee_schedule, TransferFeeSchedule};
use crate::chains::solana::constants::RAYDIUM_CPMM_PROGRAM_ID;
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

/// Anchor discriminator for `swap_base_input`, i.e. `sha256("global:swap_base_input")[..8]`.
const SWAP_BASE_INPUT: [u8; 8] = [0x8f, 0xbe, 0x5a, 0xda, 0xc4, 0x1e, 0x33, 0xde];

/// Seed of the programme's single vault/LP authority PDA.
const AUTHORITY_SEED: &[u8] = b"vault_and_lp_mint_auth_seed";

/// Denominator every CP-Swap fee rate is expressed over.
const FEE_RATE_DENOMINATOR: u64 = 1_000_000;

/// Bit index of the swap permission inside `PoolState::status`. The bit is a
/// DISABLE flag: clear means swapping is allowed.
const STATUS_BIT_SWAP_DISABLED: u8 = 2;

/// Compute units a CP-Swap swap needs. Measured against mainnet simulations,
/// which land around 35k for the swap instruction itself.
const COMPUTE_UNITS: u32 = 120_000;

/// The venue adapter: loads a CP-Swap pool and its config.
pub struct RaydiumCpmmVenue;

#[async_trait]
impl PoolVenue for RaydiumCpmmVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::RaydiumCpmm
    }

    fn program_id(&self) -> Pubkey {
        Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID).expect("CPMM program id constant is valid")
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let state = CpmmPoolState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "CP-Swap pool state did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if !state.swap_enabled() {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!("status byte {} has swapping disabled", state.status),
            });
        }
        if !state.is_open(now_unix()) {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!("pool opens at unix {}", state.open_time),
            });
        }

        // One batched read for everything the quote needs: the fee config, both
        // vaults and both mints. Five sequential `get_account` calls was the old
        // shape and it cost five round trips per quote.
        let addresses = [
            state.amm_config,
            state.vault_0,
            state.vault_1,
            state.mint_0,
            state.mint_1,
        ];
        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("CP-Swap pool accounts could not be read: {e}"),
            })?;

        let fetched = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let config = CpmmFeeConfig::decode(&fetched(0)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "CP-Swap AmmConfig did not match the expected layout".to_owned(),
            }
        })?;
        let vault_0_balance = token_account_amount(&fetched(1)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_0 vault is not a token account".to_owned(),
            }
        })?;
        let vault_1_balance = token_account_amount(&fetched(2)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_1 vault is not a token account".to_owned(),
            }
        })?;
        let transfer_fee_0 = transfer_fee_schedule(fetched(3)?);
        let transfer_fee_1 = transfer_fee_schedule(fetched(4)?);

        Ok(Box::new(CpmmMarket {
            state,
            config,
            vault_0_balance,
            vault_1_balance,
            transfer_fee_0,
            transfer_fee_1,
        }))
    }
}

/// Seconds since the unix epoch, for the pool's open-time gate. Reading the clock
/// is the loader's job; the market itself stays pure.
fn now_unix() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

/// The parts of `PoolState` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpmmPoolState {
    pub pool: Pubkey,
    pub amm_config: Pubkey,
    pub vault_0: Pubkey,
    pub vault_1: Pubkey,
    pub mint_0: Pubkey,
    pub mint_1: Pubkey,
    pub program_0: Pubkey,
    pub program_1: Pubkey,
    pub observation: Pubkey,
    pub decimals_0: u8,
    pub decimals_1: u8,
    pub status: u8,
    pub open_time: u64,
    pub protocol_fees_0: u64,
    pub protocol_fees_1: u64,
    pub fund_fees_0: u64,
    pub fund_fees_1: u64,
    pub creator_fees_0: u64,
    pub creator_fees_1: u64,
    pub creator_fee_enabled: bool,
}

impl CpmmPoolState {
    /// Decode a CP-Swap pool account. Pure: no RPC, no cache, no clock.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            amm_config: pubkey_at(data, 8)?,
            vault_0: pubkey_at(data, 72)?,
            vault_1: pubkey_at(data, 104)?,
            mint_0: pubkey_at(data, 168)?,
            mint_1: pubkey_at(data, 200)?,
            program_0: pubkey_at(data, 232)?,
            program_1: pubkey_at(data, 264)?,
            observation: pubkey_at(data, 296)?,
            status: u8_at(data, 329)?,
            decimals_0: u8_at(data, 331)?,
            decimals_1: u8_at(data, 332)?,
            protocol_fees_0: u64_at(data, 341)?,
            protocol_fees_1: u64_at(data, 349)?,
            fund_fees_0: u64_at(data, 357)?,
            fund_fees_1: u64_at(data, 365)?,
            open_time: u64_at(data, 373)?,
            creator_fee_enabled: u8_at(data, 390)? != 0,
            creator_fees_0: u64_at(data, 397)?,
            creator_fees_1: u64_at(data, 405)?,
        })
    }

    /// Whether the swap permission bit is clear.
    pub fn swap_enabled(&self) -> bool {
        self.status & (1 << STATUS_BIT_SWAP_DISABLED) == 0
    }

    /// Whether the pool has reached its open time.
    pub fn is_open(&self, now_unix: u64) -> bool {
        now_unix >= self.open_time
    }
}

/// The fee rates from `AmmConfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpmmFeeConfig {
    pub trade_fee_rate: u64,
    pub protocol_fee_rate: u64,
    pub fund_fee_rate: u64,
    pub creator_fee_rate: u64,
}

impl CpmmFeeConfig {
    /// Decode an `AmmConfig` account. Pure.
    pub fn decode(data: &[u8]) -> Option<Self> {
        Some(Self {
            trade_fee_rate: u64_at(data, 12)?,
            protocol_fee_rate: u64_at(data, 20)?,
            fund_fee_rate: u64_at(data, 28)?,
            creator_fee_rate: u64_at(data, 108)?,
        })
    }
}

/// A decoded, quotable CP-Swap pool.
#[derive(Debug, Clone)]
pub struct CpmmMarket {
    state: CpmmPoolState,
    config: CpmmFeeConfig,
    vault_0_balance: u64,
    vault_1_balance: u64,
    transfer_fee_0: Option<TransferFeeSchedule>,
    transfer_fee_1: Option<TransferFeeSchedule>,
}

impl CpmmMarket {
    /// Build a market directly from decoded parts. The offline test tier uses
    /// this to drive real captured state without any RPC.
    pub fn new(
        state: CpmmPoolState,
        config: CpmmFeeConfig,
        vault_0_balance: u64,
        vault_1_balance: u64,
        transfer_fee_0: Option<TransferFeeSchedule>,
        transfer_fee_1: Option<TransferFeeSchedule>,
    ) -> Self {
        Self {
            state,
            config,
            vault_0_balance,
            vault_1_balance,
            transfer_fee_0,
            transfer_fee_1,
        }
    }

    /// Swappable reserves: vault balances less every fee bucket awaiting
    /// collection out of the same vault.
    pub fn reserves(&self) -> (u64, u64) {
        (
            self.vault_0_balance
                .saturating_sub(self.state.protocol_fees_0)
                .saturating_sub(self.state.fund_fees_0)
                .saturating_sub(self.state.creator_fees_0),
            self.vault_1_balance
                .saturating_sub(self.state.protocol_fees_1)
                .saturating_sub(self.state.fund_fees_1)
                .saturating_sub(self.state.creator_fees_1),
        )
    }

    /// Whether `mint` is the pool's token_0 side.
    fn is_side_0(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.state.mint_0 {
            Some(true)
        } else if *mint == self.state.mint_1 {
            Some(false)
        } else {
            None
        }
    }

    /// Total rate charged on the input, over [`FEE_RATE_DENOMINATOR`].
    ///
    /// The creator fee is added to the trade fee whenever the pool enables it.
    /// The programme can charge it on either leg depending on `creator_fee_on`;
    /// charging it on the input here can only UNDER-state the output, which
    /// lowers `min_out` — the direction that still fills.
    fn input_fee_rate(&self) -> u64 {
        if self.state.creator_fee_enabled {
            self.config
                .trade_fee_rate
                .saturating_add(self.config.creator_fee_rate)
        } else {
            self.config.trade_fee_rate
        }
    }

    /// The transfer-fee schedule for one side.
    fn transfer_fee(&self, side_0: bool) -> Option<&TransferFeeSchedule> {
        if side_0 {
            self.transfer_fee_0.as_ref()
        } else {
            self.transfer_fee_1.as_ref()
        }
    }
}

impl PoolMarket for CpmmMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::RaydiumCpmm
    }

    fn pool(&self) -> Pubkey {
        self.state.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.state.mint_0, self.state.mint_1)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_side_0(mint).map(|side_0| {
            if side_0 {
                self.state.program_0
            } else {
                self.state.program_1
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_side_0(mint).map(|side_0| {
            if side_0 {
                self.state.decimals_0
            } else {
                self.state.decimals_1
            }
        })
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_0 = self
            .is_side_0(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;

        let (reserve_0, reserve_1) = self.reserves();
        let (reserve_in, reserve_out) = if input_is_0 {
            (reserve_0, reserve_1)
        } else {
            (reserve_1, reserve_0)
        };
        if reserve_in == 0 || reserve_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "a vault side is empty once collectable fees are excluded".to_owned(),
            });
        }

        // The pool only ever sees what survives the input mint's transfer fee.
        let received_by_pool =
            super::token2022::net_of_fee(self.transfer_fee(input_is_0), amount_in);
        if received_by_pool == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the input mint's transfer fee consumes the whole amount".to_owned(),
            });
        }

        let lp_fee = fee_amount(
            received_by_pool,
            self.input_fee_rate(),
            FEE_RATE_DENOMINATOR,
        );
        let swappable = received_by_pool.saturating_sub(lp_fee);
        let gross_out = constant_product_out(reserve_in, reserve_out, swappable);

        // And we only ever see what survives the output mint's transfer fee --
        // which is exactly what the programme compares `minimum_amount_out` to.
        let expected_out = super::token2022::net_of_fee(self.transfer_fee(!input_is_0), gross_out);

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
        let input_is_0 =
            self.is_side_0(&accounts.input_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_0 = self.is_side_0(&accounts.output_mint).ok_or_else(|| {
            DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            }
        })?;
        if input_is_0 == output_is_0 {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        // Orientation is driven ENTIRELY by which side the input mint is. The
        // previous builder hardcoded token_0 as the SOL side, which silently
        // paired the WSOL account with the token vault on every pool whose SOL
        // side happened to be token_1.
        let (input_vault, output_vault, input_program, output_program) = if input_is_0 {
            (
                self.state.vault_0,
                self.state.vault_1,
                self.state.program_0,
                self.state.program_1,
            )
        } else {
            (
                self.state.vault_1,
                self.state.vault_0,
                self.state.program_1,
                self.state.program_0,
            )
        };

        let authority = Pubkey::find_program_address(&[AUTHORITY_SEED], &cpmm_program_id()).0;

        let mut data = Vec::with_capacity(24);
        data.extend_from_slice(&SWAP_BASE_INPUT);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&min_out.to_le_bytes());

        Ok(Instruction {
            program_id: cpmm_program_id(),
            accounts: vec![
                AccountMeta::new_readonly(accounts.owner, true),
                AccountMeta::new_readonly(authority, false),
                AccountMeta::new_readonly(self.state.amm_config, false),
                AccountMeta::new(self.state.pool, false),
                AccountMeta::new(accounts.input_token_account, false),
                AccountMeta::new(accounts.output_token_account, false),
                AccountMeta::new(input_vault, false),
                AccountMeta::new(output_vault, false),
                AccountMeta::new_readonly(input_program, false),
                AccountMeta::new_readonly(output_program, false),
                AccountMeta::new_readonly(accounts.input_mint, false),
                AccountMeta::new_readonly(accounts.output_mint, false),
                AccountMeta::new(self.state.observation, false),
            ],
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

fn cpmm_program_id() -> Pubkey {
    Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID).expect("CPMM program id constant is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> CpmmPoolState {
        CpmmPoolState {
            pool: Pubkey::new_unique(),
            amm_config: Pubkey::new_unique(),
            vault_0: Pubkey::new_unique(),
            vault_1: Pubkey::new_unique(),
            mint_0: Pubkey::new_unique(),
            mint_1: Pubkey::new_unique(),
            program_0: crate::chains::solana::spl_token::id(),
            program_1: crate::chains::solana::spl_token::id(),
            observation: Pubkey::new_unique(),
            decimals_0: 9,
            decimals_1: 6,
            status: 0,
            open_time: 0,
            protocol_fees_0: 0,
            protocol_fees_1: 0,
            fund_fees_0: 0,
            fund_fees_1: 0,
            creator_fees_0: 0,
            creator_fees_1: 0,
            creator_fee_enabled: false,
        }
    }

    fn config() -> CpmmFeeConfig {
        CpmmFeeConfig {
            trade_fee_rate: 2_500,
            protocol_fee_rate: 120_000,
            fund_fee_rate: 40_000,
            creator_fee_rate: 500,
        }
    }

    fn market() -> CpmmMarket {
        CpmmMarket::new(
            state(),
            config(),
            1_000_000_000_000,
            1_000_000_000_000,
            None,
            None,
        )
    }

    #[test]
    fn the_swap_status_bit_is_a_disable_flag_not_an_enable_flag() {
        let mut s = state();
        assert!(s.swap_enabled(), "a zero status permits swapping");
        s.status = 0b100;
        assert!(!s.swap_enabled());
        s.status = 0b011;
        assert!(
            s.swap_enabled(),
            "deposit and withdraw bits must not block a swap"
        );
    }

    #[test]
    fn a_pool_before_its_open_time_is_not_tradable_yet() {
        let mut s = state();
        s.open_time = 1_000;
        assert!(!s.is_open(999));
        assert!(s.is_open(1_000));
    }

    #[test]
    fn reserves_exclude_every_uncollected_fee_bucket() {
        let mut s = state();
        s.protocol_fees_0 = 10;
        s.fund_fees_0 = 20;
        s.creator_fees_0 = 30;
        s.protocol_fees_1 = 1;
        let m = CpmmMarket::new(s, config(), 1_000, 2_000, None, None);
        assert_eq!(m.reserves(), (940, 1_999));
    }

    #[test]
    fn a_quote_charges_the_trade_fee_and_returns_less_than_the_spot_rate() {
        let m = market();
        let (mint_0, _) = m.mints();
        let q = m
            .quote(&mint_0, 1_000_000_000)
            .expect("balanced pool quotes");

        // 0.25% of 1e9 = 2_500_000, rounded up.
        assert_eq!(q.lp_fee, 2_500_000);
        assert!(
            q.expected_out < 1_000_000_000,
            "the fee and curve both bite"
        );
        assert!(
            q.expected_out > 990_000_000,
            "but only slightly on a deep pool"
        );
        assert!(q.price_impact_pct > 0.0 && q.price_impact_pct < 1.0);
    }

    #[test]
    fn enabling_the_creator_fee_lowers_the_quote_rather_than_raising_it() {
        let plain = state();
        let creator = CpmmPoolState {
            creator_fee_enabled: true,
            ..plain
        };
        let baseline = CpmmMarket::new(
            plain,
            config(),
            1_000_000_000_000,
            1_000_000_000_000,
            None,
            None,
        )
        .quote(&plain.mint_0, 1_000_000_000)
        .expect("quote");
        let with_creator = CpmmMarket::new(
            creator,
            config(),
            1_000_000_000_000,
            1_000_000_000_000,
            None,
            None,
        )
        .quote(&creator.mint_0, 1_000_000_000)
        .expect("quote");

        assert!(
            with_creator.lp_fee > baseline.lp_fee,
            "the creator fee adds to the input fee"
        );
        assert!(
            with_creator.expected_out < baseline.expected_out,
            "and therefore lowers the quote"
        );
    }

    #[test]
    fn an_empty_reserve_is_a_liquidity_failure_not_a_zero_output() {
        let s = state();
        let m = CpmmMarket::new(s, config(), 0, 1_000, None, None);
        assert!(matches!(
            m.quote(&s.mint_0, 1_000),
            Err(DirectSwapError::InsufficientLiquidity { .. })
        ));
    }

    #[test]
    fn a_mint_the_pool_does_not_hold_cannot_be_quoted() {
        let m = market();
        assert!(matches!(
            m.quote(&Pubkey::new_unique(), 1_000),
            Err(DirectSwapError::PairNotInPool { .. })
        ));
    }

    #[test]
    fn a_transfer_fee_mint_quotes_lower_on_both_legs() {
        let s = state();
        let schedule = TransferFeeSchedule {
            basis_points: 500,
            maximum_fee: u64::MAX,
        };
        let plain = CpmmMarket::new(
            s,
            config(),
            1_000_000_000_000,
            1_000_000_000_000,
            None,
            None,
        )
        .quote(&s.mint_0, 1_000_000_000)
        .expect("quote");
        let taxed_in = CpmmMarket::new(
            s,
            config(),
            1_000_000_000_000,
            1_000_000_000_000,
            Some(schedule),
            None,
        )
        .quote(&s.mint_0, 1_000_000_000)
        .expect("quote");
        let taxed_out = CpmmMarket::new(
            s,
            config(),
            1_000_000_000_000,
            1_000_000_000_000,
            None,
            Some(schedule),
        )
        .quote(&s.mint_0, 1_000_000_000)
        .expect("quote");

        assert!(
            taxed_in.expected_out < plain.expected_out,
            "input leg taxed"
        );
        assert!(
            taxed_out.expected_out < plain.expected_out,
            "output leg taxed"
        );
    }

    #[test]
    fn the_instruction_orients_from_the_input_mint_not_from_a_hardcoded_sol_side() {
        let m = market();
        let (mint_0, mint_1) = m.mints();
        let owner = Pubkey::new_unique();
        let ata_0 = Pubkey::new_unique();
        let ata_1 = Pubkey::new_unique();

        let zero_in = m
            .swap_instruction(
                &SwapAccounts {
                    owner,
                    input_mint: mint_0,
                    output_mint: mint_1,
                    input_token_account: ata_0,
                    output_token_account: ata_1,
                },
                1_000,
                900,
            )
            .expect("token_0 -> token_1 builds");
        let one_in = m
            .swap_instruction(
                &SwapAccounts {
                    owner,
                    input_mint: mint_1,
                    output_mint: mint_0,
                    input_token_account: ata_1,
                    output_token_account: ata_0,
                },
                1_000,
                900,
            )
            .expect("token_1 -> token_0 builds");

        assert_eq!(zero_in.accounts[6].pubkey, m.state.vault_0);
        assert_eq!(zero_in.accounts[7].pubkey, m.state.vault_1);
        assert_eq!(
            one_in.accounts[6].pubkey, m.state.vault_1,
            "reversing the pair reverses the vaults"
        );
        assert_eq!(one_in.accounts[7].pubkey, m.state.vault_0);
        assert_eq!(one_in.accounts[10].pubkey, mint_1, "input mint follows too");
    }

    #[test]
    fn the_instruction_carries_the_documented_discriminator_and_amounts() {
        let m = market();
        let (mint_0, mint_1) = m.mints();
        let ix = m
            .swap_instruction(
                &SwapAccounts {
                    owner: Pubkey::new_unique(),
                    input_mint: mint_0,
                    output_mint: mint_1,
                    input_token_account: Pubkey::new_unique(),
                    output_token_account: Pubkey::new_unique(),
                },
                5_000_000,
                4_900_000,
            )
            .expect("builds");

        assert_eq!(ix.program_id, cpmm_program_id());
        assert_eq!(ix.accounts.len(), 13);
        assert!(ix.accounts[0].is_signer, "the owner signs");
        assert_eq!(&ix.data[0..8], &SWAP_BASE_INPUT);
        assert_eq!(
            u64::from_le_bytes(ix.data[8..16].try_into().unwrap()),
            5_000_000
        );
        assert_eq!(
            u64::from_le_bytes(ix.data[16..24].try_into().unwrap()),
            4_900_000,
            "min_out is the only on-chain protection and must be verbatim"
        );
    }

    #[test]
    fn swapping_a_mint_for_itself_cannot_produce_an_instruction() {
        let m = market();
        let (mint_0, _) = m.mints();
        assert!(matches!(
            m.swap_instruction(
                &SwapAccounts {
                    owner: Pubkey::new_unique(),
                    input_mint: mint_0,
                    output_mint: mint_0,
                    input_token_account: Pubkey::new_unique(),
                    output_token_account: Pubkey::new_unique(),
                },
                1_000,
                900,
            ),
            Err(DirectSwapError::PairNotInPool { .. })
        ));
    }

    #[test]
    fn a_truncated_pool_account_decodes_to_none_instead_of_panicking() {
        assert!(CpmmPoolState::decode(Pubkey::new_unique(), &[0u8; 100]).is_none());
        assert!(CpmmFeeConfig::decode(&[0u8; 20]).is_none());
    }
}
