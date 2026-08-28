//! Pump.fun legacy (`6EF8rrec…`), the bonding curve every pump.fun token trades
//! on before it graduates to pump-swap (`pumpfun_amm.rs`). The hardest venue in
//! this engine to get right, because the fee mechanism looked instruction-
//! dependent and was NOT — see "The fee" below for what it actually is.
//!
//! # Layout, verified against mainnet
//!
//! `BondingCurve` is 150 (migrated) or 256 (full) bytes, of which the first 115
//! are meaningful and identical in both sizes:
//!
//! ```text
//!  0..8   discriminator (17 b7 f8 37 60 d8 ac 60)
//!  8      virtual_token_reserves u64
//! 16      virtual_sol_reserves u64
//! 24      real_token_reserves u64
//! 32      real_sol_reserves u64
//! 40      token_total_supply u64
//! 48      complete bool
//! 49      creator pubkey
//! 81      is_mayhem_mode bool
//! 82      is_cashback_coin bool
//! 83      quote_mint pubkey   -- the DEFAULT pubkey for a native-SOL curve
//! ```
//!
//! Confirmed against the on-chain IDL (account `AYgC53tU5BbP2NAnv5nConJxAdpQZctvmZK88pu69xRs`)
//! and a real curve's bytes: `creator` at 49 derives the correct `creator_vault`
//! PDA for a live trade (`3yWiFWMJmcrRiaVJatHnz4o3D9qJ83xhkL6uSdcVBBor` from
//! creator `7ufmve7ZSFCzuNcKRunYrGtyb2Ka1MXzkWwf7jZhVsmL`, bump 254).
//!
//! # The pair is TOKEN/NATIVE SOL, never WSOL
//!
//! `quote_mint` at offset 83 is the DEFAULT pubkey on every classic curve this
//! venue supports — pump's newer per-curve alt-quote-mint feature (`buy_v2`,
//! `buy_exact_quote_in_v2`, a totally different, larger account shape with its
//! own `associated_creator_vault` WSOL leg) sets this field to a real mint and
//! is a DIFFERENT product on the same programme; a curve with a non-default
//! `quote_mint` is refused here rather than guessed at.
//!
//! Settlement is NATIVE lamports: a buy debits the signer's own balance and a
//! sell credits it directly, never through a WSOL token account. See
//! [`PoolMarket::settles_native_sol`] and the ordering note in `super::super::plan`
//! for what that changes about the platform fee.
//!
//! # The fee: NOT gated by which instruction runs
//!
//! An earlier attempt at this venue was abandoned because two real transactions
//! showed `creator_fee_basis_points` at 0 on a plain `buy` and 30 on what looked
//! like `buy_exact_quote_in`. Replaying five more real trades (three `buy`,
//! `sell` and `buy_exact_sol_in`; the earlier two turned out to be `Buy` and
//! `BuyExactQuoteInV2` — the SECOND one is not this venue's instruction at all,
//! it is the alt-quote-mint product above) shows the true rule is nothing to do
//! with the instruction:
//!
//! * Both rates come from the SAME `FeeConfig` account
//!   (`8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt`, PDA `["fee_config",
//!   PROGRAM_ID]` under the fee programme `pfeeUxB6…`), read directly rather
//!   than CPI'd — every trade this venue replayed returned the identical
//!   `(lp=0, protocol=95, creator=30)` bps regardless of size or instruction,
//!   because this `FeeConfig`'s tier table currently holds exactly ONE tier at
//!   threshold 0 (unlike pump-swap's many-tier table). Market cap is still
//!   computed and the table is still consulted properly, in case that changes.
//! * `protocol_fee_bps` is charged on every trade with no gate, split ~evenly
//!   between two of `Global.fee_recipients`/`buyback_fee_recipients` picked
//!   BY THE CALLER (live swaps used a different pair almost every time) — the
//!   programme does the actual split internally; this venue only has to name
//!   valid recipients, never compute the split.
//! * `creator_fee_bps` is charged ONLY when `BondingCurve.creator` is not the
//!   default pubkey. Verified exactly: a trade on a curve with `creator ==
//!   default` (`buy_exact_sol_in`,
//!   `QDtXrArfEXdJJDfh7iVQfcZynEz9E2xqrkHFQCYWKFr4kUAebUXipXLdC5mLYMahyj2CKwNKBPwiDZN3qJACXbr`)
//!   paid ZERO to `creator_vault`; three trades on curves WITH a creator paid
//!   `creator_vault` exactly `ceil(net_curve_amount * 30 / 10_000)` — to the
//!   lamport, on both a `buy` and two different `sell`s.
//! * `is_mayhem_mode` waives EVERY fee, protocol included: three independent
//!   zero-fee trades all had `is_mayhem_mode = true`; three fee-charging trades
//!   all had it `false`. This venue refuses a mayhem curve rather than guess at
//!   the alternate settlement its logs imply (`buyback_basis_points` reaching
//!   5000 suggests fees are redirected, not skipped, but that path is unbuilt).
//! * `is_cashback_coin` showed one trade with protocol charged normally and
//!   creator NOT charged, but that is a single data point read at CURRENT
//!   curve state rather than replayed at the trade's own slot — NOT verified
//!   to this venue's bar. A cashback curve is refused, exactly like pump-swap
//!   AMM already refuses one.
//!
//! Original transactions that looked instruction-gated, both re-examined:
//! `66zyMCmAsPhRdzzUUpsF8v7ee5hsnjYMhGH4DbKrtiEPqxhWAePjMhTSCzWLBWmoCnmqWuivEkFQXY8QGekRk4Mv`
//! (`Buy` on an `is_cashback_coin` curve — the missing creator fee is the
//! cashback case above, not the instruction) and
//! `2SjdDD9iktpupPZvj4eZepcCR1HgRdDmtZ98W7GCNJqumRcwjsHea2BA4u72vjTL4ozdpkam23a4Ljv5GYkqMVSi`
//! (`BuyExactQuoteInV2`, the alt-quote-mint product, out of scope here).
//!
//! Fee-charging replays that pin the formula exactly: `5tNyk8fRzJYM…` (`buy`,
//! protocol 656,791 = creator×10,000/30 = ceil(69,135,801×95/10,000), creator
//! 207,408 = ceil(69,135,801×30/10,000)), `3UJ5DiLxYAFg…` and `4D6CR9NhoBvn…`
//! (`sell`, same formula off the curve's gross SOL delta), `QDtXrArf…`
//! (`buy_exact_sol_in`, protocol only, creator curve had no creator).
//!
//! # Buy is a SEARCH, and it is exact
//!
//! `buy_exact_sol_in`'s `spendable_sol_in` is spent as `net + fees(net)`, fees
//! rounded up, so this venue searches for the largest `net` satisfying that and
//! then takes ONE RAW UNIT BELOW it — the identical technique and the identical
//! correction `pumpfun_amm.rs::swappable_from_spend` applies to the identical
//! fee mechanism in the same programme family. Without the `-1` the search is
//! one lamport too generous on some curves (not all), which is an unsatisfiable
//! `min_out`; with it, being one unit low costs a couple of raw units of output
//! and nothing else.
//!
//! It reproduces the real trade
//! `QDtXrArfEXdJJDfh7iVQfcZynEz9E2xqrkHFQCYWKFr4kUAebUXipXLdC5mLYMahyj2CKwNKBPwiDZN3qJACXbr`
//! (vault delta `591_381_872` for `spendable_sol_in = 597_000_000`), and both
//! `a_pump_legacy_quote_is_exact_to_the_raw_unit` and its `_with_no_creator`
//! twin pass — the zero-slippage proof, on a curve with the creator fee inside
//! the number and on one without it.
//!
//! # The undocumented trailing accounts
//!
//! Every trade instruction (`buy`, `sell`, `buy_exact_sol_in`) reads two more
//! accounts than its own published IDL entry lists, positioned right after
//! `fee_program` — the deployed programme is ahead of its IDL here exactly the
//! way pump-swap AMM's is. Both are named in the binary's own symbol strings
//! (`strings` on the `ProgramData` account, skipping its 45-byte header):
//! `buyback_fee_recipient` and `associated_quote_buyback_fee_recipient`. Live
//! trades pick two DIFFERENT entries out of `Global.buyback_fee_recipients`
//! (eight slots) freely — every one of five replayed trades used a different
//! pair, and only the SECOND of the two ever received lamports (the first
//! showed a zero balance delta in every replay). This venue passes the first
//! two distinct non-default slots; which slot the programme actually pays is
//! its own business, not something the caller's `min_out` depends on.
//!
//! A THIRD account, `bonding_curve_v2` (`["bonding-curve-v2", mint]`, again
//! hyphenated per the same binary strings), is required immediately before
//! that pair, but ONLY when the curve has a creator (`InvalidBondingCurveV2`,
//! `0x17ba`, "bonding_curve_v2 remaining account is missing or invalid" —
//! found live, after the buyback pair alone still failed on a creator-set
//! curve). It need not exist; only its address is checked, exactly like
//! pump-swap AMM's `pool_v2`. Passing it for a curve with no creator would
//! shift the buyback pair by one, the same off-by-one pump-swap AMM's own
//! `pool_v2` causes.
//!
//! # Reserves are VIRTUAL, same trap as pump-swap
//!
//! `virtual_token_reserves`/`virtual_sol_reserves` are what the constant-product
//! curve is priced against, not `real_token_reserves`/`real_sol_reserves` — the
//! same trap `pumpfun_amm.rs` documents for the post-graduation pool. A sell's
//! payout is additionally capped at `real_sol_reserves`, the vault's own
//! balance, the way pump-swap AMM caps a boosted pool's payout at its real
//! vault.

use super::layout::{pubkey_at, u64_at, u8_at};
use super::math::{constant_product_out, price_impact_pct};
use super::pumpfun_amm::{FeeTierTable, PumpFees};
use super::token2022::{transfer_fee_schedule, TransferFeeSchedule};
use crate::chains::solana::constants::{PUMP_FUN_LEGACY_PROGRAM_ID, SOL_MINT, SYSTEM_PROGRAM_ID};
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
use std::sync::LazyLock;

/// `sha256("global:buy")[..8]`, confirmed against a live mainnet swap.
const BUY: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
/// `sha256("global:buy_exact_sol_in")[..8]`, confirmed against a live mainnet swap.
const BUY_EXACT_SOL_IN: [u8; 8] = [56, 252, 116, 8, 158, 223, 205, 95];
/// `sha256("global:sell")[..8]`, confirmed against live mainnet swaps.
const SELL: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

/// `BondingCurve`'s own discriminator.
const BONDING_CURVE_DISCRIMINATOR: [u8; 8] = [0x17, 0xb7, 0xf8, 0x37, 0x60, 0xd8, 0xac, 0x60];

/// Pump's separate fee programme, which owns the flat/tiered rate table. Same
/// address `pumpfun_amm.rs` uses -- one fee programme serves both venues.
const FEE_PROGRAM_ID: &str = "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ";

const GLOBAL_SEED: &[u8] = b"global";
const BONDING_CURVE_SEED: &[u8] = b"bonding-curve";
/// Hyphenated, unlike pump-swap AMM's `creator_vault` (underscore) -- confirmed
/// against the on-chain IDL's own seed bytes and a live PDA derivation.
const CREATOR_VAULT_SEED: &[u8] = b"creator-vault";
/// Hyphenated, the same as pump-swap AMM's `pool-v2` -- recovered from the
/// deployed programme's binary, not the IDL (see `bonding_curve_v2`).
const BONDING_CURVE_V2_SEED: &[u8] = b"bonding-curve-v2";
const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";
const GLOBAL_VOLUME_SEED: &[u8] = b"global_volume_accumulator";
const USER_VOLUME_SEED: &[u8] = b"user_volume_accumulator";
const FEE_CONFIG_SEED: &[u8] = b"fee_config";

const BPS: u64 = 10_000;

/// Compute units this venue's swap needs. A classic `buy`/`sell` consumed
/// 71,621-78,920 CU in the live trades replayed while building this venue.
const COMPUTE_UNITS: u32 = 130_000;

static WSOL_MINT: LazyLock<Pubkey> =
    LazyLock::new(|| Pubkey::from_str(SOL_MINT).expect("SOL_MINT constant is a valid pubkey"));

/// The venue adapter.
pub struct PumpFunLegacyVenue;

#[async_trait]
impl PoolVenue for PumpFunLegacyVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::PumpFunLegacy
    }

    fn program_id(&self) -> Pubkey {
        pump_legacy_program_id()
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let mut curve = BondingCurve::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "pump.fun legacy bonding curve did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        // The account carries no mint field of its own -- the PDA is derived
        // FROM the mint (`["bonding-curve", mint]`), which cannot be inverted.
        // The one thing the account DOES hold is the curve's own token
        // balance, so the mint is recovered by asking what token account the
        // curve owns, across both SPL programmes (every pump.fun mint
        // observed while building this venue was Token-2022, but this does
        // not assume that).
        curve.mint = get_rpc_client()
            .get_all_token_accounts(pool)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("could not read the curve's own token account: {e}"),
            })?
            .into_iter()
            .max_by_key(|info| info.balance)
            .ok_or_else(|| DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "the bonding curve holds no token account, so its mint cannot be found"
                    .to_owned(),
            })
            .and_then(|info| {
                Pubkey::from_str(&info.mint).map_err(|e| DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: format!("the curve's token account names an invalid mint: {e}"),
                })
            })?;

        if curve.complete {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "the bonding curve has completed and migrated to pump-swap; trade the \
                         pump-swap pool instead"
                    .to_owned(),
            });
        }
        if curve.is_mayhem_mode {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "a mayhem-mode curve waives fees through a settlement path this venue \
                         does not build; route it through an aggregator instead"
                    .to_owned(),
            });
        }
        if curve.is_cashback_coin {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "a cashback curve's creator-fee waiver is not verified to this engine's \
                         bar; route it through an aggregator instead"
                    .to_owned(),
            });
        }
        if curve.quote_mint != Pubkey::default() {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "the curve trades an alternate quote mint (buy_v2 territory), which is \
                         a different product this venue does not build"
                    .to_owned(),
            });
        }

        let global = global_address();
        let fee_config = fee_config_address();
        let addresses = [curve.mint, global, fee_config];
        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("pump.fun legacy accounts could not be read: {e}"),
            })?;

        let required = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let mint_account = required(0)?;
        let mint_decimals = super::layout::u8_at(&mint_account.data, 44).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "the curve's mint is not a mint account".to_owned(),
            }
        })?;

        let global_state = GlobalFeeRecipients::decode(&required(1)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "pump.fun legacy Global did not match the expected layout".to_owned(),
            }
        })?;
        let fee_recipient =
            global_state
                .first_fee_recipient()
                .ok_or_else(|| DirectSwapError::PoolNotTradable {
                    pool: *pool,
                    detail: "pump.fun legacy Global lists no protocol fee recipient".to_owned(),
                })?;
        let (buyback_wallet, buyback_paid) =
            global_state.two_buyback_recipients().ok_or_else(|| {
                DirectSwapError::PoolNotTradable {
                    pool: *pool,
                    detail: "pump.fun legacy Global lists fewer than two buyback fee recipients"
                        .to_owned(),
                }
            })?;

        let tiers = FeeTierTable::decode(&required(2)?.data);

        Ok(Box::new(PumpLegacyMarket {
            curve,
            token_program: mint_account.owner,
            mint_decimals,
            transfer_fee: transfer_fee_schedule(mint_account),
            fee_recipient,
            buyback_wallet,
            buyback_paid,
            tiers,
        }))
    }
}

/// The parts of a pump.fun legacy `BondingCurve` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BondingCurve {
    pub pool: Pubkey,
    pub mint: Pubkey,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
    pub creator: Pubkey,
    pub is_mayhem_mode: bool,
    pub is_cashback_coin: bool,
    pub quote_mint: Pubkey,
}

impl BondingCurve {
    /// Decode a bonding curve account. Pure. `mint` is not stored in the
    /// account (the discovery layer already knows it, the same as the price
    /// decoder's contract), so the caller supplies it -- but a direct-swap
    /// venue is looked up by POOL ADDRESS, never by mint, so this decode takes
    /// it from the caller's own record via [`PumpFunLegacyVenue::load`]'s
    /// dispatcher instead: the pool account carries no mint field, so `mint`
    /// here is filled in from context by the loader, not decoded from bytes.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        if data.len() < 115 || data[0..8] != BONDING_CURVE_DISCRIMINATOR {
            return None;
        }
        Some(Self {
            pool,
            // Filled in by the loader once the mint is known; zero here is a
            // decode placeholder never used before it is overwritten.
            mint: Pubkey::default(),
            virtual_token_reserves: u64_at(data, 8)?,
            virtual_sol_reserves: u64_at(data, 16)?,
            real_token_reserves: u64_at(data, 24)?,
            real_sol_reserves: u64_at(data, 32)?,
            token_total_supply: u64_at(data, 40)?,
            complete: u8_at(data, 48)? != 0,
            creator: pubkey_at(data, 49)?,
            is_mayhem_mode: u8_at(data, 81)? != 0,
            is_cashback_coin: u8_at(data, 82)? != 0,
            quote_mint: pubkey_at(data, 83)?,
        })
    }

    pub fn creator_set(&self) -> bool {
        self.creator != Pubkey::default()
    }
}

/// The fee recipient arrays out of pump.fun legacy's `Global` account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalFeeRecipients {
    pub fee_recipients: [Pubkey; 7],
    pub buyback_fee_recipients: [Pubkey; 8],
}

impl GlobalFeeRecipients {
    /// Decode the two recipient arrays out of `Global`. Pure.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let mut fee_recipients = [Pubkey::default(); 7];
        for (index, slot) in fee_recipients.iter_mut().enumerate() {
            *slot = pubkey_at(data, 162 + index * 32)?;
        }
        let mut buyback_fee_recipients = [Pubkey::default(); 8];
        for (index, slot) in buyback_fee_recipients.iter_mut().enumerate() {
            *slot = pubkey_at(data, 741 + index * 32)?;
        }
        Some(Self {
            fee_recipients,
            buyback_fee_recipients,
        })
    }

    /// The first initialised protocol fee recipient. Live swaps rotate between
    /// the seven slots; any initialised one is accepted by the programme.
    pub fn first_fee_recipient(&self) -> Option<Pubkey> {
        self.fee_recipients
            .iter()
            .find(|key| **key != Pubkey::default())
            .copied()
    }

    /// Two DISTINCT initialised buyback fee recipients, for the two trailing
    /// accounts every trade instruction demands beyond its own IDL. Every live
    /// trade replayed while building this venue used a different pair from
    /// these eight slots; only the second position ever received lamports, but
    /// the first must still name a valid, distinct recipient.
    pub fn two_buyback_recipients(&self) -> Option<(Pubkey, Pubkey)> {
        let mut distinct = self
            .buyback_fee_recipients
            .iter()
            .filter(|key| **key != Pubkey::default());
        let first = *distinct.next()?;
        let second = *distinct.next()?;
        Some((first, second))
    }
}

/// A decoded, quotable pump.fun legacy bonding curve.
#[derive(Debug, Clone)]
pub struct PumpLegacyMarket {
    curve: BondingCurve,
    token_program: Pubkey,
    mint_decimals: u8,
    transfer_fee: Option<TransferFeeSchedule>,
    fee_recipient: Pubkey,
    buyback_wallet: Pubkey,
    buyback_paid: Pubkey,
    tiers: Option<FeeTierTable>,
}

impl PumpLegacyMarket {
    /// Build a market directly from decoded parts, for the offline test tier.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        curve: BondingCurve,
        token_program: Pubkey,
        mint_decimals: u8,
        transfer_fee: Option<TransferFeeSchedule>,
        fee_recipient: Pubkey,
        buyback_wallet: Pubkey,
        buyback_paid: Pubkey,
        tiers: Option<FeeTierTable>,
    ) -> Self {
        Self {
            curve,
            token_program,
            mint_decimals,
            transfer_fee,
            fee_recipient,
            buyback_wallet,
            buyback_paid,
            tiers,
        }
    }

    fn is_base(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.curve.mint {
            Some(true)
        } else if *mint == *WSOL_MINT {
            Some(false)
        } else {
            None
        }
    }

    /// Market cap in lamports, the fee table's own key: `total_supply *
    /// virtual_sol_reserves / virtual_token_reserves`.
    fn market_cap(&self) -> Option<u128> {
        if self.curve.virtual_token_reserves == 0 {
            return None;
        }
        Some(
            (self.curve.token_total_supply as u128) * (self.curve.virtual_sol_reserves as u128)
                / (self.curve.virtual_token_reserves as u128),
        )
    }

    /// The rates this curve charges right now, from the shared `FeeConfig`.
    fn fees(&self) -> PumpFees {
        match (&self.tiers, self.market_cap()) {
            (Some(table), Some(cap)) => table.fees_for(cap),
            (Some(table), None) => table.most_expensive(),
            (None, _) => PumpFees::default(),
        }
    }

    /// Total bps actually charged, creator share included only when the curve
    /// has a creator to pay it to.
    fn total_bps(&self) -> u64 {
        self.fees().total_bps(self.curve.creator_set())
    }

    /// Fee owed on `net`, each of the (up to) two components rounded UP
    /// separately -- verified against three live trades to the lamport.
    fn fee_on(&self, net: u64) -> u64 {
        let fees = self.fees();
        let mut total = 0u128;
        let mut charge = |bps: u64| {
            if bps > 0 {
                total += ((net as u128) * (bps as u128)).div_ceil(BPS as u128);
            }
        };
        charge(fees.lp_bps);
        charge(fees.protocol_bps);
        if self.curve.creator_set() {
            charge(fees.creator_bps);
        }
        total.min(net as u128) as u64
    }

    /// What actually reaches the curve on a `buy_exact_sol_in`: one raw unit
    /// below the largest `net` satisfying `net + fee_on(net) <= spendable`.
    ///
    /// Verified EXACTLY against a real `buy_exact_sol_in` on a curve with no
    /// creator
    /// (`QDtXrArfEXdJJDfh7iVQfcZynEz9E2xqrkHFQCYWKFr4kUAebUXipXLdC5mLYMahyj2CKwNKBPwiDZN3qJACXbr`,
    /// vault delta `591_381_872` for `spendable_sol_in = 597_000_000`), and
    /// proved at zero slippage on live curves both with and without a creator.
    /// See the module docs for why the result is one unit below the largest
    /// affordable amount.
    fn net_for_spend(&self, spendable: u64) -> u64 {
        let (mut low, mut high) = (0u64, spendable);
        while low < high {
            let mid = low + (high - low + 1) / 2;
            let cost = (mid as u128) + (self.fee_on(mid) as u128);
            if cost <= spendable as u128 {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        // One raw unit below the largest affordable amount, the same
        // correction `pumpfun_amm.rs::swappable_from_spend` applies to the
        // identical search against the identical fee mechanism in the same
        // programme family. Being one unit low costs a couple of raw units of
        // output; being one unit high is an unsatisfiable `min_out`.
        low.saturating_sub(1)
    }

    fn creator_vault(&self) -> Pubkey {
        Pubkey::find_program_address(
            &[CREATOR_VAULT_SEED, self.curve.creator.as_ref()],
            &pump_legacy_program_id(),
        )
        .0
    }

    /// The named accounts shared by every trade instruction: the 14 fields
    /// `sell`'s IDL entry lists (the smallest of the three), in the order the
    /// programme reads them. `buy`/`buy_exact_sol_in` insert two more --
    /// `token_program` moves and the volume accumulators are added -- so they
    /// build their own list rather than extend this one positionally.
    fn associated_bonding_curve(&self) -> Pubkey {
        get_associated_token_address_with_program_id(
            &self.curve.pool,
            &self.curve.mint,
            &self.token_program,
        )
    }

    /// The `bonding_curve_v2` account the deployed programme insists on for a
    /// curve WITH a creator -- undocumented in the classic `buy`/`sell`/
    /// `buy_exact_sol_in` IDL entries, recovered from the deployed programme's
    /// own binary (`strings` on `ProgramData`, HYPHENATED like pump-swap AMM's
    /// `pool-v2`) after a live simulation failed `InvalidBondingCurveV2`
    /// (`0x17ba`, "bonding_curve_v2 remaining account is missing or invalid")
    /// on a curve WITH a creator when this account was omitted. It need not
    /// exist; only its address is checked.
    fn bonding_curve_v2(&self) -> Pubkey {
        Pubkey::find_program_address(
            &[BONDING_CURVE_V2_SEED, self.curve.mint.as_ref()],
            &pump_legacy_program_id(),
        )
        .0
    }
}

impl PoolMarket for PumpLegacyMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::PumpFunLegacy
    }

    fn pool(&self) -> Pubkey {
        self.curve.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.curve.mint, *WSOL_MINT)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_base(mint).map(|base| {
            if base {
                self.token_program
            } else {
                crate::chains::solana::spl_token::id()
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_base(mint)
            .map(|base| if base { self.mint_decimals } else { 9 })
    }

    fn settles_native_sol(&self) -> bool {
        true
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_base = self
            .is_base(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.curve.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;
        if self.curve.virtual_token_reserves == 0 || self.curve.virtual_sol_reserves == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.curve.pool,
                amount_in,
                detail: "the bonding curve has no reserves".to_owned(),
            });
        }

        let (expected_out, lp_fee) = if input_is_base {
            // SELL: tokens in, native SOL out. Pump's fees come OFF the gross
            // output, verified exactly against two live sells.
            let sold = super::token2022::net_of_fee(self.transfer_fee.as_ref(), amount_in);
            let gross = constant_product_out(
                self.curve.virtual_token_reserves,
                self.curve.virtual_sol_reserves,
                sold,
            )
            // The vault can never pay out more than it actually holds -- the
            // same cap pump-swap AMM applies to a boosted pool's real vault.
            .min(self.curve.real_sol_reserves);
            let fee = self.fee_on(gross);
            (gross.saturating_sub(fee), fee)
        } else {
            // BUY: native SOL in, tokens out. Fees sit ON TOP of the amount
            // that reaches the curve, so `amount_in` (already net of the
            // platform fee -- this is `buy_exact_sol_in`'s `spendable_sol_in`)
            // is searched down to the swapped amount first.
            let net = self.net_for_spend(amount_in);
            let fee = amount_in.saturating_sub(net);
            let gross = constant_product_out(
                self.curve.virtual_sol_reserves,
                self.curve.virtual_token_reserves,
                net,
            )
            .min(self.curve.real_token_reserves);
            let out = super::token2022::net_of_fee(self.transfer_fee.as_ref(), gross);
            (out, fee)
        };

        if expected_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.curve.pool,
                amount_in,
                detail: "the curve returns nothing at this size".to_owned(),
            });
        }

        let (reserve_in, reserve_out) = if input_is_base {
            (
                self.curve.virtual_token_reserves,
                self.curve.virtual_sol_reserves,
            )
        } else {
            (
                self.curve.virtual_sol_reserves,
                self.curve.virtual_token_reserves,
            )
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
                    pool: self.curve.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_base =
            self.is_base(&accounts.output_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.curve.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        if input_is_base == output_is_base {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.curve.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        let associated_bonding_curve = self.associated_bonding_curve();
        let creator_vault = self.creator_vault();

        let mut data = Vec::with_capacity(24);
        let mut metas = if input_is_base {
            // SELL: base in, native SOL out. `sell`'s own 14 accounts.
            data.extend_from_slice(&SELL);
            data.extend_from_slice(&amount_in.to_le_bytes());
            data.extend_from_slice(&min_out.to_le_bytes());
            vec![
                AccountMeta::new_readonly(global_address(), false),
                AccountMeta::new(self.fee_recipient, false),
                AccountMeta::new_readonly(self.curve.mint, false),
                AccountMeta::new(self.curve.pool, false),
                AccountMeta::new(associated_bonding_curve, false),
                AccountMeta::new(accounts.input_token_account, false),
                AccountMeta::new(accounts.owner, true),
                AccountMeta::new_readonly(system_program_id(), false),
                AccountMeta::new(creator_vault, false),
                AccountMeta::new_readonly(self.token_program, false),
                AccountMeta::new_readonly(event_authority_address(), false),
                AccountMeta::new_readonly(pump_legacy_program_id(), false),
                AccountMeta::new_readonly(fee_config_address(), false),
                AccountMeta::new_readonly(fee_program_id(), false),
            ]
        } else {
            // BUY: native SOL in, base out. `buy_exact_sol_in`'s own 16
            // accounts (track_volume omitted -- a live trade sent zero bytes
            // for it, and setting it would require the user's volume
            // accumulator to already exist).
            data.extend_from_slice(&BUY_EXACT_SOL_IN);
            data.extend_from_slice(&amount_in.to_le_bytes());
            data.extend_from_slice(&min_out.to_le_bytes());
            vec![
                AccountMeta::new_readonly(global_address(), false),
                AccountMeta::new(self.fee_recipient, false),
                AccountMeta::new_readonly(self.curve.mint, false),
                AccountMeta::new(self.curve.pool, false),
                AccountMeta::new(associated_bonding_curve, false),
                AccountMeta::new(accounts.output_token_account, false),
                AccountMeta::new(accounts.owner, true),
                AccountMeta::new_readonly(system_program_id(), false),
                AccountMeta::new_readonly(self.token_program, false),
                AccountMeta::new(creator_vault, false),
                AccountMeta::new_readonly(event_authority_address(), false),
                AccountMeta::new_readonly(pump_legacy_program_id(), false),
                AccountMeta::new_readonly(global_volume_address(), false),
                AccountMeta::new(user_volume_address(&accounts.owner), false),
                AccountMeta::new_readonly(fee_config_address(), false),
                AccountMeta::new_readonly(fee_program_id(), false),
            ]
        };

        // Undocumented trailing accounts every trade instruction demands
        // beyond its own IDL entry, in this order: `bonding_curve_v2` only
        // when the curve has a creator (pump's own coins), then the buyback
        // pair unconditionally -- the same shape pump-swap AMM's `pool_v2` +
        // buyback pair takes, confirmed by a live simulation failure without it.
        if self.curve.creator_set() {
            metas.push(AccountMeta::new(self.bonding_curve_v2(), false));
        }
        metas.push(AccountMeta::new(self.buyback_wallet, false));
        metas.push(AccountMeta::new(self.buyback_paid, false));

        Ok(Instruction {
            program_id: pump_legacy_program_id(),
            accounts: metas,
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

fn pump_legacy_program_id() -> Pubkey {
    Pubkey::from_str(PUMP_FUN_LEGACY_PROGRAM_ID).expect("pump legacy program id constant is valid")
}

fn fee_program_id() -> Pubkey {
    Pubkey::from_str(FEE_PROGRAM_ID).expect("pump fee program id constant is valid")
}

fn system_program_id() -> Pubkey {
    Pubkey::from_str(SYSTEM_PROGRAM_ID).expect("system program id constant is valid")
}

fn global_address() -> Pubkey {
    Pubkey::find_program_address(&[GLOBAL_SEED], &pump_legacy_program_id()).0
}

fn event_authority_address() -> Pubkey {
    Pubkey::find_program_address(&[EVENT_AUTHORITY_SEED], &pump_legacy_program_id()).0
}

fn global_volume_address() -> Pubkey {
    Pubkey::find_program_address(&[GLOBAL_VOLUME_SEED], &pump_legacy_program_id()).0
}

fn user_volume_address(user: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[USER_VOLUME_SEED, user.as_ref()],
        &pump_legacy_program_id(),
    )
    .0
}

/// The fee table's address: a PDA of the FEE programme, seeded with pump
/// legacy's own programme id. Confirmed against the live account
/// `8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt`.
fn fee_config_address() -> Pubkey {
    Pubkey::find_program_address(
        &[FEE_CONFIG_SEED, pump_legacy_program_id().as_ref()],
        &fee_program_id(),
    )
    .0
}

/// Bonding curve PDA for `mint`: `["bonding-curve", mint]`.
pub fn bonding_curve_address(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[BONDING_CURVE_SEED, mint.as_ref()],
        &pump_legacy_program_id(),
    )
    .0
}
