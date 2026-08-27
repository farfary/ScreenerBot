//! Pump.fun AMM (`pAMMBay6…`), the constant-product programme pump calls
//! "pump-swap". Every pump.fun token that graduates off its bonding curve lands
//! in one of these pools.
//!
//! # Layout, verified against mainnet
//!
//! `Pool` is 301 bytes on chain, of which the first 245 are meaningful:
//!
//! ```text
//!   8 pool_bump   9 index   11 creator   43 base_mint   75 quote_mint
//! 107 lp_mint    139 pool_base_token_account   171 pool_quote_token_account
//! 203 lp_supply  211 coin_creator
//! ```
//!
//! # The pair is BASE/QUOTE, not TOKEN/SOL
//!
//! The programme has no symmetric `swap`: it has `buy_exact_quote_in`, which
//! spends the quote mint, and `sell`, which spends the base mint. Which of the
//! two runs is decided purely by which side the input mint is — and the SOL side
//! is not always the quote side. The deepest pump-swap pool on mainnet has WSOL
//! as its BASE.
//!
//! # The fee, which is not a constant
//!
//! Total fee = `lp_fee + protocol_fee + coin_creator_fee`, all charged in the
//! QUOTE mint. The three rates come from a `FeeConfig` account owned by pump's
//! separate fee programme (`pfeeUxB6…`), as a table of tiers keyed by the pool's
//! market capitalisation in lamports. A brand new pool pays 125 bps; one above
//! roughly 98_240 SOL of market cap pays 30. Reading the flat rates out of
//! `GlobalConfig` and stopping there would under-charge a small pool by almost a
//! full percent, which becomes a `min_out` the pool cannot meet.
//!
//! Which of the two rate sources applies is decided by `coin_creator`: a pool
//! that carries one is a POOL PUMP CREATED, and pays the tier table; a pool
//! without one is a third-party pool and pays the flat rates. That rule was read
//! off the fee programme's own `Program return:` values on live trades, across
//! pools spanning 295 to 2.9 million SOL of market cap — never inferred.
//!
//! Market cap is `base_supply × quote_reserve / base_reserve`. When the tier
//! table cannot be read at all the venue falls back to the FIRST tier, which is
//! the most expensive one — that under-states the output, which lowers
//! `min_out`, which is the direction that still fills.
//!
//! # Virtual reserves
//!
//! A "boosted" pool prices off `quote_vault + virtual_quote_reserves`, a `u64`
//! the published IDL does not carry, at offset 245. Ignoring it over-states the
//! output of every buy in such a pool by around 16%. The programme's own logs
//! spell out the rule: "effective = real + virtual is pricing-only; payout is
//! capped at real_vault".
//!
//! # The buyback recipient
//!
//! The deployed programme is AHEAD of its own published IDL: it requires a
//! buyback fee recipient and that recipient's quote token account, appended
//! after the fee programme. Omitting them fails with `BuybackFeeRecipientMissing`
//! (`0x17aa`). The recipients live in `GlobalConfig.buyback_fee_recipients`, and
//! live swaps pick freely among the eight. The same applies to a `pool_v2`
//! account (`InvalidPoolV2`, `0x17ae`), a PDA of `["pool-v2", base_mint]` — note
//! the HYPHEN, which is why no underscore spelling derives it. Both seeds were
//! read out of the deployed programme's own binary, not guessed.
//!
//! These trailing accounts are POSITIONAL and CONDITIONAL, which is why passing
//! the full set unconditionally fails: the programme reads whatever is there in
//! order. The shape, confirmed against live swaps of three pools:
//!
//! ```text
//! [pool_v2]                 only when the pool has a coin creator
//! buyback_recipient         always
//! buyback_recipient_ata     always
//! ```
//!
//! A pool with `is_cashback_coin` set expects one further account ahead of all
//! of them. That path is not built here: the venue refuses such a pool so the
//! caller routes it elsewhere, rather than guessing at an account the programme
//! will settle real money against.
//!
//! On a buy the fees sit ON TOP of the swapped amount, so an exact-quote-in swap
//! has to back them out. The closed form `spendable × 10_000 / (10_000 + bps)`
//! is ONE RAW UNIT too high, because the programme rounds each of the three fees
//! up separately: what it actually swaps is the largest `q` whose full cost
//! `q + Σ⌈q·bpsᵢ/10_000⌉` still fits inside the spend. Reproducing that search
//! is the difference between a quote that is exact and one the pool rejects.
//! Verified against a live `buy_exact_quote_in`, whose vault delta matched the
//! searched `q` to the lamport.
//!
//! On a sell the fees come off the output, and the LP share stays in the vault
//! while the protocol and creator shares leave it — also confirmed against a
//! live sell's vault deltas.

use super::layout::{pubkey_at, token_account_amount, u128_at, u32_at, u64_at, u8_at};
use super::math::{constant_product_out, price_impact_pct};
use super::token2022::{transfer_fee_schedule, TransferFeeSchedule};
use crate::chains::solana::constants::{
    ASSOCIATED_TOKEN_PROGRAM_ID, PUMP_FUN_AMM_PROGRAM_ID, SYSTEM_PROGRAM_ID,
};
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id;
use crate::chains::solana::swaps::direct::error::{DirectSwapError, DirectSwapResult};
use crate::chains::solana::swaps::direct::venue::{
    PoolMarket, PoolVenue, SwapAccounts, VenueQuote,
};
use async_trait::async_trait;
use std::str::FromStr;

/// `sha256("global:buy_exact_quote_in")[..8]`.
const BUY_EXACT_QUOTE_IN: [u8; 8] = [0xc6, 0x2e, 0x15, 0x52, 0xb4, 0xd9, 0xe8, 0x70];

/// `sha256("global:sell")[..8]`, confirmed against live mainnet swaps.
const SELL: [u8; 8] = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];

/// Pump's separate fee programme, which owns the tier table.
const FEE_PROGRAM_ID: &str = "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ";

const GLOBAL_CONFIG_SEED: &[u8] = b"global_config";
const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";
const GLOBAL_VOLUME_SEED: &[u8] = b"global_volume_accumulator";
const USER_VOLUME_SEED: &[u8] = b"user_volume_accumulator";
const CREATOR_VAULT_SEED: &[u8] = b"creator_vault";
const POOL_V2_SEED: &[u8] = b"pool-v2";
const FEE_CONFIG_SEED: &[u8] = b"fee_config";

/// Basis-point denominator for every pump fee.
const BPS: u64 = 10_000;

/// Compute units a pump-swap swap needs.
const COMPUTE_UNITS: u32 = 160_000;

/// The venue adapter.
pub struct PumpFunAmmVenue;

#[async_trait]
impl PoolVenue for PumpFunAmmVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::PumpFunAmm
    }

    fn program_id(&self) -> Pubkey {
        pump_amm_program_id()
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let state = PumpAmmPoolState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "pump-swap pool state did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if state.is_cashback_coin {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "a cashback pool expects an extra trailing account this venue does not \
                         build; route it through an aggregator instead"
                    .to_owned(),
            });
        }

        let global_config = global_config_address();
        let fee_config = fee_config_address();
        let addresses = [
            global_config,
            fee_config,
            state.base_token_account,
            state.quote_token_account,
            state.base_mint,
            state.quote_mint,
        ];
        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("pump-swap pool accounts could not be read: {e}"),
            })?;

        let required = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let global = GlobalConfig::decode(&required(0)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "pump-swap GlobalConfig did not match the expected layout".to_owned(),
            }
        })?;
        if global.swaps_disabled() {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!(
                    "global disable_flags {} block swapping",
                    global.disable_flags
                ),
            });
        }
        let protocol_fee_recipient =
            global
                .first_fee_recipient()
                .ok_or_else(|| DirectSwapError::PoolNotTradable {
                    pool: *pool,
                    detail: "pump-swap global config lists no protocol fee recipient".to_owned(),
                })?;

        let tiers = accounts
            .get(1)
            .and_then(Option::as_ref)
            .and_then(|account| FeeTierTable::decode(&account.data));

        let base_reserve = token_account_amount(&required(2)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "the pool's base token account is not a token account".to_owned(),
            }
        })?;
        let quote_reserve = token_account_amount(&required(3)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "the pool's quote token account is not a token account".to_owned(),
            }
        })?;

        let base_mint_account = required(4)?;
        let quote_mint_account = required(5)?;
        let base_supply = mint_supply(&base_mint_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "the base mint is not a mint account".to_owned(),
            }
        })?;

        let buyback_fee_recipient =
            global
                .first_buyback_recipient()
                .ok_or_else(|| DirectSwapError::PoolNotTradable {
                    pool: *pool,
                    detail: "pump-swap global config lists no buyback fee recipient".to_owned(),
                })?;

        Ok(Box::new(PumpAmmMarket {
            state,
            protocol_fee_recipient,
            buyback_fee_recipient,
            tiers,
            flat_fees: global.flat_fees(),
            base_token_program: base_mint_account.owner,
            quote_token_program: quote_mint_account.owner,
            base_decimals: mint_decimals(&base_mint_account.data).unwrap_or(0),
            quote_decimals: mint_decimals(&quote_mint_account.data).unwrap_or(0),
            base_supply,
            base_reserve,
            quote_reserve,
            transfer_fee_base: transfer_fee_schedule(base_mint_account),
            transfer_fee_quote: transfer_fee_schedule(quote_mint_account),
        }))
    }
}

fn mint_supply(data: &[u8]) -> Option<u64> {
    u64_at(data, 36)
}

fn mint_decimals(data: &[u8]) -> Option<u8> {
    u8_at(data, 44)
}

/// The three rates pump charges, all in basis points of the QUOTE mint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PumpFees {
    pub lp_bps: u64,
    pub protocol_bps: u64,
    pub creator_bps: u64,
}

impl PumpFees {
    /// Total rate charged on a trade, creator share included.
    pub fn total_bps(&self, charge_creator: bool) -> u64 {
        let creator = if charge_creator { self.creator_bps } else { 0 };
        self.lp_bps
            .saturating_add(self.protocol_bps)
            .saturating_add(creator)
    }
}

/// The parts of pump's `GlobalConfig` this venue needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalConfig {
    pub lp_fee_bps: u64,
    pub protocol_fee_bps: u64,
    pub creator_fee_bps: u64,
    pub disable_flags: u8,
    pub fee_recipients: [Pubkey; 8],
    pub buyback_fee_recipients: [Pubkey; 8],
}

impl GlobalConfig {
    /// Decode a `GlobalConfig` account. Pure.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let mut fee_recipients = [Pubkey::default(); 8];
        for (index, slot) in fee_recipients.iter_mut().enumerate() {
            *slot = pubkey_at(data, 57 + index * 32)?;
        }
        let mut buyback_fee_recipients = [Pubkey::default(); 8];
        for (index, slot) in buyback_fee_recipients.iter_mut().enumerate() {
            *slot = pubkey_at(data, 643 + index * 32)?;
        }
        Some(Self {
            buyback_fee_recipients,
            lp_fee_bps: u64_at(data, 40)?,
            protocol_fee_bps: u64_at(data, 48)?,
            creator_fee_bps: u64_at(data, 313)?,
            disable_flags: u8_at(data, 56)?,
            fee_recipients,
        })
    }

    /// Whether any disable flag is set. The programme uses this byte as a
    /// bitmask of things that are switched OFF, so anything non-zero is a reason
    /// to refuse rather than to guess which bit meant what.
    pub fn swaps_disabled(&self) -> bool {
        self.disable_flags != 0
    }

    /// The first initialised protocol fee recipient. Live swaps rotate between
    /// the slots; any initialised one is accepted by the programme.
    pub fn first_fee_recipient(&self) -> Option<Pubkey> {
        self.fee_recipients
            .iter()
            .find(|key| **key != Pubkey::default())
            .copied()
    }

    /// The first initialised buyback fee recipient. Live swaps pick freely
    /// among the eight slots.
    pub fn first_buyback_recipient(&self) -> Option<Pubkey> {
        self.buyback_fee_recipients
            .iter()
            .find(|key| **key != Pubkey::default())
            .copied()
    }

    /// The flat rates, used when the tier table cannot be read.
    pub fn flat_fees(&self) -> PumpFees {
        PumpFees {
            lp_bps: self.lp_fee_bps,
            protocol_bps: self.protocol_fee_bps,
            creator_bps: self.creator_fee_bps,
        }
    }
}

/// Pump's market-cap-keyed fee table, out of the fee programme's `FeeConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeTierTable {
    tiers: Vec<(u128, PumpFees)>,
}

impl FeeTierTable {
    /// Decode the `fee_tiers` vector of a `FeeConfig` account.
    ///
    /// Layout: discriminator, `bump`, `admin`, `flat_fees` (three `u64`), then a
    /// borsh `Vec<FeeTier>` where each tier is a `u128` lamport threshold
    /// followed by the same three `u64` rates.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let mut offset = 8 + 1 + 32 + 24;
        let count = u32_at(data, offset)? as usize;
        offset += 4;
        // A table this long is a decode failure, not a real config.
        if count > 1_024 {
            return None;
        }
        let mut tiers = Vec::with_capacity(count);
        for _ in 0..count {
            let threshold = u128_at(data, offset)?;
            let fees = PumpFees {
                lp_bps: u64_at(data, offset + 16)?,
                protocol_bps: u64_at(data, offset + 24)?,
                creator_bps: u64_at(data, offset + 32)?,
            };
            tiers.push((threshold, fees));
            offset += 40;
        }
        if tiers.is_empty() {
            return None;
        }
        Some(Self { tiers })
    }

    /// The rates for a pool of this market capitalisation: the last tier whose
    /// threshold it has reached.
    pub fn fees_for(&self, market_cap_lamports: u128) -> PumpFees {
        let mut chosen = self.tiers[0].1;
        for (threshold, fees) in &self.tiers {
            if market_cap_lamports >= *threshold {
                chosen = *fees;
            } else {
                break;
            }
        }
        chosen
    }

    /// The most expensive tier, used when the market cap cannot be computed.
    pub fn most_expensive(&self) -> PumpFees {
        self.tiers[0].1
    }
}

/// The parts of the pump-swap `Pool` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PumpAmmPoolState {
    pub pool: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_token_account: Pubkey,
    pub quote_token_account: Pubkey,
    pub coin_creator: Pubkey,
    pub is_cashback_coin: bool,
    /// Virtual quote reserves of a boosted pool. Pricing only — the payout is
    /// still capped at what the real vault holds.
    pub virtual_quote_reserves: u64,
}

impl PumpAmmPoolState {
    /// Decode a pump-swap pool account. Pure.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            base_mint: pubkey_at(data, 43)?,
            quote_mint: pubkey_at(data, 75)?,
            base_token_account: pubkey_at(data, 139)?,
            quote_token_account: pubkey_at(data, 171)?,
            coin_creator: pubkey_at(data, 211)?,
            is_cashback_coin: u8_at(data, 244)? != 0,
            virtual_quote_reserves: u64_at(data, 245)?,
        })
    }
}

/// A decoded, quotable pump-swap pool.
#[derive(Debug, Clone)]
pub struct PumpAmmMarket {
    state: PumpAmmPoolState,
    protocol_fee_recipient: Pubkey,
    buyback_fee_recipient: Pubkey,
    tiers: Option<FeeTierTable>,
    flat_fees: PumpFees,
    base_token_program: Pubkey,
    quote_token_program: Pubkey,
    base_decimals: u8,
    quote_decimals: u8,
    base_supply: u64,
    base_reserve: u64,
    quote_reserve: u64,
    transfer_fee_base: Option<TransferFeeSchedule>,
    transfer_fee_quote: Option<TransferFeeSchedule>,
}

impl PumpAmmMarket {
    /// Build a market directly from decoded parts, for the offline test tier.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: PumpAmmPoolState,
        protocol_fee_recipient: Pubkey,
        buyback_fee_recipient: Pubkey,
        tiers: Option<FeeTierTable>,
        flat_fees: PumpFees,
        base_token_program: Pubkey,
        quote_token_program: Pubkey,
        base_decimals: u8,
        quote_decimals: u8,
        base_supply: u64,
        base_reserve: u64,
        quote_reserve: u64,
        transfer_fee_base: Option<TransferFeeSchedule>,
        transfer_fee_quote: Option<TransferFeeSchedule>,
    ) -> Self {
        Self {
            state,
            protocol_fee_recipient,
            buyback_fee_recipient,
            tiers,
            flat_fees,
            base_token_program,
            quote_token_program,
            base_decimals,
            quote_decimals,
            base_supply,
            base_reserve,
            quote_reserve,
            transfer_fee_base,
            transfer_fee_quote,
        }
    }

    /// Whether `mint` is this pool's base side.
    fn is_base(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.state.base_mint {
            Some(true)
        } else if *mint == self.state.quote_mint {
            Some(false)
        } else {
            None
        }
    }

    /// The pool's market capitalisation in quote raw units, which is what pump's
    /// fee tiers are keyed on.
    pub fn market_cap(&self) -> Option<u128> {
        if self.base_reserve == 0 || self.base_supply == 0 {
            return None;
        }
        Some(
            (self.base_supply as u128) * (self.quote_reserve as u128) / (self.base_reserve as u128),
        )
    }

    /// The rates this pool actually charges right now.
    ///
    /// A pool WITHOUT a coin creator is a third-party pool and pays the flat
    /// rates; one with a creator is pump's own and pays the market-cap tier.
    pub fn fees(&self) -> PumpFees {
        if !self.charges_creator_fee() {
            return self.flat_fees;
        }
        match (&self.tiers, self.market_cap()) {
            (Some(table), Some(cap)) => table.fees_for(cap),
            // No market cap means the pool is empty or unreadable: charge the
            // dearest tier rather than guess a cheap one.
            (Some(table), None) => table.most_expensive(),
            (None, _) => self.flat_fees,
        }
    }

    /// Whether the coin creator's share is charged. A pool with no creator pays
    /// no creator fee, because there is no vault for it to land in.
    fn charges_creator_fee(&self) -> bool {
        self.state.coin_creator != Pubkey::default()
    }

    /// Total rate charged on the quote leg of a trade.
    fn total_fee_bps(&self) -> u64 {
        self.fees().total_bps(self.charges_creator_fee())
    }

    /// The fee taken off `quote_amount`, rounded UP the way each of pump's three
    /// components rounds up individually.
    fn quote_fee_on(&self, quote_amount: u64) -> u64 {
        let fees = self.fees();
        let mut total = 0u128;
        let mut charge = |bps: u64| {
            if bps > 0 {
                total += ((quote_amount as u128) * (bps as u128)).div_ceil(BPS as u128);
            }
        };
        charge(fees.lp_bps);
        charge(fees.protocol_bps);
        if self.charges_creator_fee() {
            charge(fees.creator_bps);
        }
        total.min(quote_amount as u128) as u64
    }

    /// The full cost of swapping `quote_amount`: the amount itself plus each of
    /// pump's three fees, every one of them rounded UP on its own.
    fn spend_for(&self, quote_amount: u64) -> u128 {
        (quote_amount as u128) + (self.quote_fee_on(quote_amount) as u128)
    }

    /// How much of `spendable` actually reaches the curve on a buy.
    ///
    /// The largest `q` whose [`Self::spend_for`] still fits. A closed-form
    /// division is one raw unit too generous here, and one unit too generous is
    /// a `min_out` the pool refuses.
    fn swappable_from_spend(&self, spendable: u64) -> u64 {
        let (mut low, mut high) = (0u64, spendable);
        while low < high {
            let mid = low + (high - low + 1) / 2;
            if self.spend_for(mid) <= spendable as u128 {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        // One raw unit below the largest affordable amount is what the
        // programme actually puts on the curve, measured against live
        // `buy_exact_quote_in` swaps of two very different sizes. Being one unit
        // low costs a couple of raw units of output; being one unit high is an
        // unsatisfiable `min_out`.
        low.saturating_sub(1)
    }

    /// Quote reserve used for PRICING: the vault plus any virtual reserves.
    fn effective_quote_reserve(&self) -> u64 {
        self.quote_reserve
            .saturating_add(self.state.virtual_quote_reserves)
    }

    /// The PDA that owns the coin creator's fee vault.
    fn coin_creator_vault_authority(&self) -> Pubkey {
        Pubkey::find_program_address(
            &[CREATOR_VAULT_SEED, self.state.coin_creator.as_ref()],
            &pump_amm_program_id(),
        )
        .0
    }

    /// The account the coin creator's share is paid into.
    fn coin_creator_vault_ata(&self) -> Pubkey {
        get_associated_token_address_with_program_id(
            &self.coin_creator_vault_authority(),
            &self.state.quote_mint,
            &self.quote_token_program,
        )
    }

    /// The account the protocol's share is paid into.
    fn protocol_fee_recipient_token_account(&self) -> Pubkey {
        get_associated_token_address_with_program_id(
            &self.protocol_fee_recipient,
            &self.state.quote_mint,
            &self.quote_token_program,
        )
    }

    /// The named accounts every pump-swap trade instruction carries, in the
    /// order the programme reads them. The two trade instructions differ only in
    /// whether the volume accumulators are present, so the shared prefix is
    /// built once.
    fn common_accounts(&self, accounts: &SwapAccounts, input_is_base: bool) -> Vec<AccountMeta> {
        let (user_base, user_quote) = if input_is_base {
            (accounts.input_token_account, accounts.output_token_account)
        } else {
            (accounts.output_token_account, accounts.input_token_account)
        };
        vec![
            AccountMeta::new(self.state.pool, false),
            AccountMeta::new(accounts.owner, true),
            AccountMeta::new_readonly(global_config_address(), false),
            AccountMeta::new_readonly(self.state.base_mint, false),
            AccountMeta::new_readonly(self.state.quote_mint, false),
            AccountMeta::new(user_base, false),
            AccountMeta::new(user_quote, false),
            AccountMeta::new(self.state.base_token_account, false),
            AccountMeta::new(self.state.quote_token_account, false),
            AccountMeta::new_readonly(self.protocol_fee_recipient, false),
            AccountMeta::new(self.protocol_fee_recipient_token_account(), false),
            AccountMeta::new_readonly(self.base_token_program, false),
            AccountMeta::new_readonly(self.quote_token_program, false),
            AccountMeta::new_readonly(system_program_id(), false),
            AccountMeta::new_readonly(associated_token_program_id(), false),
            AccountMeta::new_readonly(event_authority_address(), false),
            AccountMeta::new_readonly(pump_amm_program_id(), false),
            AccountMeta::new(self.coin_creator_vault_ata(), false),
            AccountMeta::new_readonly(self.coin_creator_vault_authority(), false),
        ]
    }

    /// The `pool_v2` account the deployed programme insists on. It need not
    /// exist — the programme only checks the address — but omitting it fails
    /// the swap with `InvalidPoolV2`.
    fn pool_v2(&self) -> Pubkey {
        Pubkey::find_program_address(
            &[POOL_V2_SEED, self.state.base_mint.as_ref()],
            &pump_amm_program_id(),
        )
        .0
    }

    /// The account the buyback share is paid into.
    fn buyback_recipient_token_account(&self) -> Pubkey {
        get_associated_token_address_with_program_id(
            &self.buyback_fee_recipient,
            &self.state.quote_mint,
            &self.quote_token_program,
        )
    }
}

impl PoolMarket for PumpAmmMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::PumpFunAmm
    }

    fn pool(&self) -> Pubkey {
        self.state.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.state.base_mint, self.state.quote_mint)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_base(mint).map(|base| {
            if base {
                self.base_token_program
            } else {
                self.quote_token_program
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_base(mint).map(|base| {
            if base {
                self.base_decimals
            } else {
                self.quote_decimals
            }
        })
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_base = self
            .is_base(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;
        if self.base_reserve == 0 || self.quote_reserve == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "a side of the pool is empty".to_owned(),
            });
        }

        let (expected_out, lp_fee) = if input_is_base {
            // SELL: base in, quote out, fees off the output.
            let received = super::token2022::net_of_fee(self.transfer_fee_base.as_ref(), amount_in);
            let gross =
                constant_product_out(self.base_reserve, self.effective_quote_reserve(), received)
                    // A boosted pool prices off the virtual reserve but can only pay out
                    // of the real one.
                    .min(self.quote_reserve);
            let fee = self.quote_fee_on(gross);
            let net = gross.saturating_sub(fee);
            let out = super::token2022::net_of_fee(self.transfer_fee_quote.as_ref(), net);
            // Report the pool's fee in INPUT units, which is this venue's
            // contract, by converting at the realised rate of this same fill.
            let in_base = if gross == 0 {
                0
            } else {
                (((fee as u128) * (received as u128)) / (gross as u128)) as u64
            };
            (out, in_base)
        } else {
            // BUY: quote in, base out. Pump charges its fees ON TOP of the
            // swapped amount, so an exact-quote-in trade backs them out first.
            let received =
                super::token2022::net_of_fee(self.transfer_fee_quote.as_ref(), amount_in);
            // Pump charges its fees on top of the swapped amount, so the
            // swapped amount is what is left once a fee sized against ITSELF is
            // taken out. Rounding that fee UP is what the programme does to
            // every fee it charges, and rounding it down here would predict a
            // larger swap than the programme performs — which becomes a
            // `min_out` it cannot meet.
            let swapped = self.swappable_from_spend(received);
            let fee = received.saturating_sub(swapped);
            let gross =
                constant_product_out(self.effective_quote_reserve(), self.base_reserve, swapped);
            let out = super::token2022::net_of_fee(self.transfer_fee_base.as_ref(), gross);
            (out, fee)
        };

        if expected_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the pool returns nothing at this size".to_owned(),
            });
        }
        let output_reserve = if input_is_base {
            self.quote_reserve
        } else {
            self.base_reserve
        };
        if expected_out >= output_reserve {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the output exceeds what the pool holds".to_owned(),
            });
        }

        let (reserve_in, reserve_out) = if input_is_base {
            (self.base_reserve, self.effective_quote_reserve())
        } else {
            (self.effective_quote_reserve(), self.base_reserve)
        };

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
        let input_is_base =
            self.is_base(&accounts.input_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_base =
            self.is_base(&accounts.output_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        if input_is_base == output_is_base {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        let mut metas = self.common_accounts(accounts, input_is_base);
        let mut data = Vec::with_capacity(25);

        if input_is_base {
            data.extend_from_slice(&SELL);
            data.extend_from_slice(&amount_in.to_le_bytes());
            data.extend_from_slice(&min_out.to_le_bytes());
        } else {
            data.extend_from_slice(&BUY_EXACT_QUOTE_IN);
            data.extend_from_slice(&amount_in.to_le_bytes());
            data.extend_from_slice(&min_out.to_le_bytes());
            // `track_volume`: false. Tracking would write to the user's volume
            // accumulator, an account a first-time wallet does not have, and a
            // swap must never depend on a bookkeeping account existing.
            data.push(0);
            metas.push(AccountMeta::new_readonly(global_volume_address(), false));
            metas.push(AccountMeta::new(
                user_volume_address(&accounts.owner),
                false,
            ));
        }

        metas.push(AccountMeta::new_readonly(fee_config_address(), false));
        metas.push(AccountMeta::new_readonly(fee_program_id(), false));
        // Required by the deployed programme even though its published IDL does
        // not list them, and in this order: without `pool_v2` the swap fails
        // InvalidPoolV2, without the buyback pair BuybackFeeRecipientMissing.
        // `pool_v2` only exists for pump's OWN pools. Passing it for a
        // third-party pool shifts every following account by one, and the
        // programme reads the buyback slot as an unauthorised recipient.
        if self.charges_creator_fee() {
            metas.push(AccountMeta::new(self.pool_v2(), false));
        }
        metas.push(AccountMeta::new(self.buyback_fee_recipient, false));
        metas.push(AccountMeta::new(
            self.buyback_recipient_token_account(),
            false,
        ));

        Ok(Instruction {
            program_id: pump_amm_program_id(),
            accounts: metas,
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

fn pump_amm_program_id() -> Pubkey {
    Pubkey::from_str(PUMP_FUN_AMM_PROGRAM_ID).expect("pump AMM program id constant is valid")
}

fn fee_program_id() -> Pubkey {
    Pubkey::from_str(FEE_PROGRAM_ID).expect("pump fee program id constant is valid")
}

fn system_program_id() -> Pubkey {
    Pubkey::from_str(SYSTEM_PROGRAM_ID).expect("system program id constant is valid")
}

fn associated_token_program_id() -> Pubkey {
    Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID).expect("ATA program id constant is valid")
}

fn global_config_address() -> Pubkey {
    Pubkey::find_program_address(&[GLOBAL_CONFIG_SEED], &pump_amm_program_id()).0
}

fn event_authority_address() -> Pubkey {
    Pubkey::find_program_address(&[EVENT_AUTHORITY_SEED], &pump_amm_program_id()).0
}

fn global_volume_address() -> Pubkey {
    Pubkey::find_program_address(&[GLOBAL_VOLUME_SEED], &pump_amm_program_id()).0
}

fn user_volume_address(user: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[USER_VOLUME_SEED, user.as_ref()], &pump_amm_program_id()).0
}

/// The fee table's address: a PDA of the FEE programme, seeded with the pump AMM
/// programme's own id.
fn fee_config_address() -> Pubkey {
    Pubkey::find_program_address(
        &[FEE_CONFIG_SEED, pump_amm_program_id().as_ref()],
        &fee_program_id(),
    )
    .0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> FeeTierTable {
        FeeTierTable {
            tiers: vec![
                (
                    0,
                    PumpFees {
                        lp_bps: 2,
                        protocol_bps: 93,
                        creator_bps: 30,
                    },
                ),
                (
                    420_000_000_000,
                    PumpFees {
                        lp_bps: 20,
                        protocol_bps: 5,
                        creator_bps: 95,
                    },
                ),
                (
                    98_240_000_000_000,
                    PumpFees {
                        lp_bps: 20,
                        protocol_bps: 5,
                        creator_bps: 5,
                    },
                ),
            ],
        }
    }

    fn state() -> PumpAmmPoolState {
        PumpAmmPoolState {
            pool: Pubkey::new_unique(),
            base_mint: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            base_token_account: Pubkey::new_unique(),
            quote_token_account: Pubkey::new_unique(),
            coin_creator: Pubkey::new_unique(),
            is_cashback_coin: false,
            virtual_quote_reserves: 0,
        }
    }

    fn market(state: PumpAmmPoolState, base_reserve: u64, quote_reserve: u64) -> PumpAmmMarket {
        PumpAmmMarket::new(
            state,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Some(table()),
            PumpFees {
                lp_bps: 25,
                protocol_bps: 5,
                creator_bps: 0,
            },
            crate::chains::solana::spl_token::id(),
            crate::chains::solana::spl_token::id(),
            6,
            9,
            1_000_000_000_000_000,
            base_reserve,
            quote_reserve,
            None,
            None,
        )
    }

    #[test]
    fn the_tier_is_the_last_threshold_the_market_cap_has_reached() {
        let table = table();
        assert_eq!(
            table.fees_for(0).lp_bps,
            2,
            "a brand new pool pays the top rate"
        );
        assert_eq!(table.fees_for(500_000_000_000).creator_bps, 95);
        assert_eq!(table.fees_for(u128::MAX).creator_bps, 5);
    }

    #[test]
    fn a_market_cap_below_every_threshold_still_resolves_to_the_first_tier() {
        assert_eq!(table().fees_for(1).protocol_bps, 93);
    }

    #[test]
    fn the_market_cap_is_the_supply_priced_at_the_pools_own_rate() {
        let state = state();
        // 1e15 base units against 1e12 base / 1e12 quote: cap == supply.
        let market = market(state, 1_000_000_000_000, 1_000_000_000_000);
        assert_eq!(market.market_cap(), Some(1_000_000_000_000_000));
    }

    #[test]
    fn an_empty_pool_has_no_market_cap_and_pays_the_dearest_tier() {
        let state = state();
        let market = market(state, 0, 1_000);
        assert_eq!(market.market_cap(), None);
        assert_eq!(
            market.fees().lp_bps,
            2,
            "the dearest tier, not a cheap guess"
        );
    }

    #[test]
    fn a_pool_without_a_creator_is_not_charged_the_creator_share() {
        let mut state = state();
        state.coin_creator = Pubkey::default();
        let market = market(state, 1_000_000_000_000, 1_000_000_000_000);
        // A creator-less pool falls off the tier table entirely and onto the
        // flat rates, which carry no creator share at all.
        assert_eq!(market.fees(), market.flat_fees);
        assert_eq!(
            market.total_fee_bps(),
            market.flat_fees.lp_bps + market.flat_fees.protocol_bps
        );
    }

    #[test]
    fn a_buy_backs_the_fee_out_of_the_amount_the_caller_is_spending() {
        let state = state();
        let market = market(state, 1_000_000_000_000_000, 10_000_000_000_000);
        let quote = market.quote(&state.quote_mint, 1_000_000_000).expect("buy");
        assert!(quote.lp_fee > 0, "the pool's own fee must be reported");
        // This pool's market cap lands on the 20 + 5 + 95 bps tier, so the fee
        // backed out of a 1e9 spend is 1.2% of it, not the headline 0.3%.
        assert_eq!(market.total_fee_bps(), 120);
        // The searched amount, not the closed form: the closed form would leave
        // one more raw unit on the curve than the programme actually swaps.
        let swapped = market.swappable_from_spend(1_000_000_000);
        assert!(market.spend_for(swapped) < 1_000_000_000);
        assert!(market.spend_for(swapped + 2) > 1_000_000_000);
        assert_eq!(quote.lp_fee, 1_000_000_000 - swapped, "{quote:?}");
    }

    #[test]
    fn a_sell_takes_the_fee_off_the_output() {
        let state = state();
        let market = market(state, 1_000_000_000_000_000, 10_000_000_000_000);
        let sell = market.quote(&state.base_mint, 1_000_000_000).expect("sell");
        assert!(sell.expected_out > 0);
        assert!(sell.lp_fee > 0);
    }

    #[test]
    fn a_round_trip_at_this_size_can_never_come_back_up() {
        let state = state();
        let market = market(state, 1_000_000_000_000_000, 10_000_000_000_000);
        let buy = market.quote(&state.quote_mint, 1_000_000_000).expect("buy");
        let sell = market
            .quote(&state.base_mint, buy.expected_out)
            .expect("sell");
        assert!(
            sell.expected_out < 1_000_000_000,
            "two 30 bps fees must cost something: {} back from 1_000_000_000",
            sell.expected_out
        );
    }

    #[test]
    fn a_buy_and_a_sell_build_different_instructions() {
        let state = state();
        let market = market(state, 1_000_000_000_000_000, 10_000_000_000_000);
        let owner = Pubkey::new_unique();
        let base_account = Pubkey::new_unique();
        let quote_account = Pubkey::new_unique();

        let buy = market
            .swap_instruction(
                &SwapAccounts {
                    owner,
                    input_mint: state.quote_mint,
                    output_mint: state.base_mint,
                    input_token_account: quote_account,
                    output_token_account: base_account,
                },
                1_000,
                900,
            )
            .expect("buy builds");
        let sell = market
            .swap_instruction(
                &SwapAccounts {
                    owner,
                    input_mint: state.base_mint,
                    output_mint: state.quote_mint,
                    input_token_account: base_account,
                    output_token_account: quote_account,
                },
                1_000,
                900,
            )
            .expect("sell builds");

        assert_eq!(&buy.data[..8], &BUY_EXACT_QUOTE_IN);
        assert_eq!(&sell.data[..8], &SELL);
        assert_eq!(buy.data.len(), 25, "a buy carries the track_volume flag");
        assert_eq!(buy.data[24], 0, "volume tracking stays off");
        assert_eq!(sell.data.len(), 24);

        // Both orientations must place the wallet's accounts on the right side.
        assert_eq!(
            buy.accounts[5].pubkey, base_account,
            "user_base_token_account"
        );
        assert_eq!(
            buy.accounts[6].pubkey, quote_account,
            "user_quote_token_account"
        );
        assert_eq!(sell.accounts[5].pubkey, base_account);
        assert_eq!(sell.accounts[6].pubkey, quote_account);

        assert_eq!(
            buy.accounts.len(),
            26,
            "a buy carries both volume accumulators"
        );
        assert_eq!(sell.accounts.len(), 24);
        assert_eq!(buy.accounts[22].pubkey, fee_program_id());
        assert_eq!(sell.accounts[20].pubkey, fee_program_id());
        assert_eq!(
            buy.accounts[23].pubkey, sell.accounts[21].pubkey,
            "pool_v2 is a property of the pool, not of the direction"
        );
        assert_eq!(
            buy.accounts[24].pubkey, sell.accounts[22].pubkey,
            "the buyback recipient is a property of the config, not the direction"
        );
        assert_eq!(buy.accounts[23].pubkey, market.pool_v2());
    }

    #[test]
    fn pool_v2_is_derived_from_the_base_mint_with_a_hyphenated_seed() {
        // Both fixtures come from live mainnet swaps that carry the account.
        let mut state = state();
        state.base_mint = Pubkey::from_str("5UUH9RTDiSpq6HKS6bp4NdU9PNJpXRXuiw6ShBTBhgH2").unwrap();
        assert_eq!(
            market(state, 1, 1).pool_v2().to_string(),
            "7ksJEP8TRm1YxCSKrEfu9BzkDheKZJgS1yctmBS4mWY8"
        );
        state.base_mint = Pubkey::from_str("9cRCn9rGT8V2imeM2BaKs13yhMEais3ruM3rPvTGpump").unwrap();
        assert_eq!(
            market(state, 1, 1).pool_v2().to_string(),
            "8oVgtpGYR2GvCQCVQg8ZcXoYj8qrzfeyimDWE6FTuW9k"
        );
    }

    #[test]
    fn the_fee_config_is_a_pda_of_the_fee_programme_seeded_with_the_amm() {
        // Verified against the account live swaps actually pass.
        assert_eq!(
            fee_config_address().to_string(),
            "5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx"
        );
        assert_eq!(
            global_config_address().to_string(),
            "ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw"
        );
        assert_eq!(
            event_authority_address().to_string(),
            "GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR"
        );
        assert_eq!(
            global_volume_address().to_string(),
            "C2aFPdENg4A2HQsmrd5rTw5TaYBX5Ku887cWjbFKtZpw"
        );
    }

    #[test]
    fn a_pool_with_no_creator_derives_the_vault_authority_live_swaps_use() {
        let mut state = state();
        state.coin_creator = Pubkey::default();
        let market = market(state, 1, 1);
        assert_eq!(
            market.coin_creator_vault_authority().to_string(),
            "8N3GDaZ2iwN65oxVatKTLPNooAVUJTbfiVJ1ahyqwjSk"
        );
    }

    #[test]
    fn a_boosted_pool_prices_off_the_virtual_reserve() {
        let mut state = state();
        let plain = market(state, 1_000_000_000_000_000, 1_000_000_000);
        state.virtual_quote_reserves = 9_000_000_000;
        let boosted = market(state, 1_000_000_000_000_000, 1_000_000_000);

        let plain_buy = plain.quote(&state.quote_mint, 10_000_000).expect("plain");
        let boosted_buy = boosted
            .quote(&state.quote_mint, 10_000_000)
            .expect("boosted");
        assert!(
            boosted_buy.expected_out < plain_buy.expected_out / 5,
            "a ten-times deeper effective reserve must buy far less base: {} vs {}",
            boosted_buy.expected_out,
            plain_buy.expected_out
        );
    }

    #[test]
    fn a_boosted_pool_never_promises_more_quote_than_its_real_vault_holds() {
        let mut state = state();
        state.virtual_quote_reserves = 9_000_000_000;
        let boosted = market(state, 1_000_000_000_000_000, 1_000_000_000);
        // A sell big enough that the virtual-reserve curve would pay out more
        // quote than the vault contains.
        let quote = boosted
            .quote(&state.base_mint, 900_000_000_000_000_000)
            .expect("a huge sell still quotes");
        assert!(
            quote.expected_out <= 1_000_000_000,
            "the real vault is the ceiling, got {}",
            quote.expected_out
        );
    }

    #[test]
    fn the_cashback_flag_and_virtual_reserve_are_read_from_the_bytes_past_the_idl() {
        let mut data = vec![0u8; 301];
        data[244] = 1;
        data[245..253].copy_from_slice(&17_584_505_358u64.to_le_bytes());
        let decoded = PumpAmmPoolState::decode(Pubkey::new_unique(), &data).expect("decodes");
        assert!(decoded.is_cashback_coin);
        assert_eq!(decoded.virtual_quote_reserves, 17_584_505_358);
    }

    #[test]
    fn a_mint_the_pool_does_not_hold_is_refused() {
        let state = state();
        let market = market(state, 1_000, 1_000);
        assert!(matches!(
            market.quote(&Pubkey::new_unique(), 1),
            Err(DirectSwapError::PairNotInPool { .. })
        ));
    }

    #[test]
    fn a_table_claiming_an_absurd_number_of_tiers_is_a_decode_failure() {
        let mut data = vec![0u8; 4_073];
        data[65..69].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(FeeTierTable::decode(&data).is_none());
    }

    #[test]
    fn a_truncated_fee_config_is_a_decode_failure_rather_than_a_panic() {
        assert!(FeeTierTable::decode(&[0u8; 20]).is_none());
    }
}
