// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Typed Solana chain facts both access backends return, and the own-RPC decoders that produce them from raw accounts.
//!
//! The service backend receives these facts already decoded; the own-RPC
//! backend reads raw accounts and decodes them here, so the two backends hand
//! their callers the same values.

use crate::chains::solana::constants::{SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID};
use crate::chains::solana::solana_program::program_option::COption;
use crate::chains::solana::solana_program::program_pack::Pack;
use crate::chains::solana::solana_sdk::{account::Account, pubkey::Pubkey};
use crate::chains::solana::spl_token::state::Mint as SplMint;
use crate::chains::solana::spl_token_2022::extension::{
    transfer_fee::{TransferFee, TransferFeeConfig},
    BaseStateWithExtensions, StateWithExtensions,
};
use crate::chains::solana::spl_token_2022::state::Mint as Mint2022;

/// The token program that owns a mint or token account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenProgram {
    SplToken,
    SplToken2022,
}

impl TokenProgram {
    /// The program's address.
    pub fn id(self) -> &'static str {
        match self {
            TokenProgram::SplToken => SPL_TOKEN_PROGRAM_ID,
            TokenProgram::SplToken2022 => TOKEN_2022_PROGRAM_ID,
        }
    }
}

/// One transfer-fee setting of a Token-2022 mint, effective from `epoch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferFeeEpoch {
    pub epoch: u64,
    pub basis_points: u16,
    pub maximum_fee: u64,
}

/// The older and newer transfer-fee settings of a Token-2022 mint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferFeeSchedule {
    pub older: TransferFeeEpoch,
    pub newer: TransferFeeEpoch,
}

impl TransferFeeSchedule {
    /// The fee charged on a transfer of `amount` at `epoch`, computed by the
    /// token program's own rule; `None` when it cannot be computed.
    pub fn fee_at(&self, epoch: u64, amount: u64) -> Option<u64> {
        let fee = |setting: TransferFeeEpoch| TransferFee {
            epoch: setting.epoch.into(),
            maximum_fee: setting.maximum_fee.into(),
            transfer_fee_basis_points: setting.basis_points.into(),
        };
        let config = TransferFeeConfig {
            older_transfer_fee: fee(self.older),
            newer_transfer_fee: fee(self.newer),
            ..TransferFeeConfig::default()
        };
        config.calculate_epoch_fee(epoch, amount)
    }

    fn from_config(config: &TransferFeeConfig) -> Self {
        let setting = |fee: &TransferFee| TransferFeeEpoch {
            epoch: u64::from(fee.epoch),
            basis_points: u16::from(fee.transfer_fee_basis_points),
            maximum_fee: u64::from(fee.maximum_fee),
        };
        Self {
            older: setting(&config.older_transfer_fee),
            newer: setting(&config.newer_transfer_fee),
        }
    }
}

/// Name, symbol and URI of a mint's token metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

/// What a mint account records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintFacts {
    pub mint: String,
    pub program: TokenProgram,
    pub decimals: u8,
    pub supply: u64,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    /// Present only for a Token-2022 mint with the transfer-fee extension.
    pub transfer_fee: Option<TransferFeeSchedule>,
    /// Present only when requested and the mint has metadata.
    pub metadata: Option<TokenMetadata>,
}

/// The outcome of reading one mint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MintLookup {
    Mint(MintFacts),
    /// No account at the address.
    Absent,
    /// An account that is not a mint this build decodes.
    NotAMint {
        detail: String,
    },
}

impl MintLookup {
    /// The facts, `None` when there is no decodable mint.
    pub fn into_facts(self) -> Option<MintFacts> {
        match self {
            MintLookup::Mint(facts) => Some(facts),
            MintLookup::Absent | MintLookup::NotAMint { .. } => None,
        }
    }
}

/// Who controls an account, as chain state records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountControl {
    /// An account owned by this program, which alone can move its lamports.
    Program(String),
    /// A token account of `program`, closable only by `authority`.
    TokenAccount { program: String, authority: String },
}

/// The network's position and constants, read together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkFacts {
    pub slot: u64,
    pub epoch: u64,
    pub confirmed_height: u64,
    pub finalized_height: u64,
}

/// How fresh a token-account read must be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// Any cached read the backend holds.
    Any,
    /// A read at or after this slot (after an own swap landed there).
    AtLeastSlot(u64),
}

/// Decodes a mint account, auto-detecting SPL Token vs Token-2022 and reading
/// the Token-2022 transfer-fee extension.
pub fn decode_mint(mint: &str, account: &Account) -> MintLookup {
    if account.data.is_empty() {
        return MintLookup::NotAMint {
            detail: "account data is empty".to_owned(),
        };
    }

    if account.owner == crate::chains::solana::spl_token::id() {
        return match SplMint::unpack(&account.data) {
            Ok(state) => MintLookup::Mint(from_spl_mint(mint, &state)),
            Err(error) => MintLookup::NotAMint {
                detail: format!("spl token mint: {error}"),
            },
        };
    }

    if account.owner == crate::chains::solana::spl_token_2022::id() {
        // A mint with extensions is longer than the base layout, so the plain
        // unpack refuses it and the extensions-aware parser reads it.
        let state = match StateWithExtensions::<Mint2022>::unpack(&account.data) {
            Ok(state) => state,
            Err(error) => {
                return match Mint2022::unpack(&account.data) {
                    Ok(base) => MintLookup::Mint(from_2022_mint(mint, &base, None)),
                    Err(_) => MintLookup::NotAMint {
                        detail: format!("token-2022 mint with extensions: {error}"),
                    },
                }
            }
        };
        let transfer_fee = state
            .get_extension::<TransferFeeConfig>()
            .ok()
            .map(TransferFeeSchedule::from_config);
        return MintLookup::Mint(from_2022_mint(mint, &state.base, transfer_fee));
    }

    MintLookup::NotAMint {
        detail: format!("owner {} is not a supported token program", account.owner),
    }
}

fn from_spl_mint(mint: &str, state: &SplMint) -> MintFacts {
    MintFacts {
        mint: mint.to_owned(),
        program: TokenProgram::SplToken,
        decimals: state.decimals,
        supply: state.supply,
        mint_authority: coption_to_string(state.mint_authority),
        freeze_authority: coption_to_string(state.freeze_authority),
        transfer_fee: None,
        metadata: None,
    }
}

fn from_2022_mint(
    mint: &str,
    state: &Mint2022,
    transfer_fee: Option<TransferFeeSchedule>,
) -> MintFacts {
    MintFacts {
        mint: mint.to_owned(),
        program: TokenProgram::SplToken2022,
        decimals: state.decimals,
        supply: state.supply,
        mint_authority: coption_to_string(state.mint_authority),
        freeze_authority: coption_to_string(state.freeze_authority),
        transfer_fee,
        metadata: None,
    }
}

fn coption_to_string(value: COption<Pubkey>) -> Option<String> {
    match value {
        COption::Some(pk) => Some(pk.to_string()),
        COption::None => None,
    }
}

/// Who controls an account owned by `program` with `data`: the authority of a
/// token account, otherwise the program itself.
pub fn account_control(program: String, data: &[u8]) -> AccountControl {
    let authority = match program.as_str() {
        SPL_TOKEN_PROGRAM_ID => crate::chains::solana::spl_token::state::Account::unpack(data)
            .ok()
            .map(|account| account.owner.to_string()),
        TOKEN_2022_PROGRAM_ID => StateWithExtensions::<
            crate::chains::solana::spl_token_2022::state::Account,
        >::unpack(data)
        .ok()
        .map(|account| account.base.owner.to_string()),
        _ => None,
    };
    match authority {
        Some(authority) => AccountControl::TokenAccount { program, authority },
        None => AccountControl::Program(program),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(owner: Pubkey, data: Vec<u8>) -> Account {
        Account {
            lamports: 1_461_600,
            data,
            owner,
            executable: false,
            rent_epoch: 0,
        }
    }

    #[test]
    fn spl_mint_maps_decimals_supply_and_authorities() {
        let authority = Pubkey::new_unique();
        let state = SplMint {
            mint_authority: COption::Some(authority),
            supply: 1_000_000_000,
            decimals: 6,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        let mut data = vec![0u8; SplMint::LEN];
        SplMint::pack(state, &mut data).unwrap();

        let MintLookup::Mint(facts) =
            decode_mint("M", &account(crate::chains::solana::spl_token::id(), data))
        else {
            panic!("an spl mint decodes");
        };
        assert_eq!(facts.program, TokenProgram::SplToken);
        assert_eq!(facts.decimals, 6);
        assert_eq!(facts.supply, 1_000_000_000);
        assert_eq!(facts.mint_authority, Some(authority.to_string()));
        assert_eq!(facts.freeze_authority, None);
        assert_eq!(facts.transfer_fee, None);
    }

    #[test]
    fn token_2022_mint_maps_decimals_supply_and_authorities() {
        let mint_authority = Pubkey::new_unique();
        let freeze_authority = Pubkey::new_unique();
        let state = Mint2022 {
            mint_authority: COption::Some(mint_authority),
            supply: 42,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::Some(freeze_authority),
        };
        let mut data = vec![0u8; Mint2022::LEN];
        Mint2022::pack(state, &mut data).unwrap();

        let MintLookup::Mint(facts) = decode_mint(
            "M",
            &account(crate::chains::solana::spl_token_2022::id(), data),
        ) else {
            panic!("a token-2022 mint decodes");
        };
        assert_eq!(facts.program, TokenProgram::SplToken2022);
        assert_eq!(facts.decimals, 9);
        assert_eq!(facts.supply, 42);
        assert_eq!(facts.mint_authority, Some(mint_authority.to_string()));
        assert_eq!(facts.freeze_authority, Some(freeze_authority.to_string()));
    }

    #[test]
    fn an_account_of_another_program_is_not_a_mint() {
        let lookup = decode_mint("M", &account(Pubkey::new_unique(), vec![1, 2, 3]));
        assert!(matches!(lookup, MintLookup::NotAMint { .. }));
        assert!(lookup.into_facts().is_none());
    }

    #[test]
    fn coption_to_string_round_trips_some_and_none() {
        let pk = Pubkey::new_unique();
        assert_eq!(coption_to_string(COption::Some(pk)), Some(pk.to_string()));
        assert_eq!(coption_to_string(COption::None), None);
    }

    /// The fee the typed schedule computes equals the token program's own
    /// computation on the raw extension, for both settings and the cap.
    #[test]
    fn the_typed_fee_schedule_matches_the_token_program_rule() {
        let older = TransferFeeEpoch {
            epoch: 700,
            basis_points: 100,
            maximum_fee: 5_000_000,
        };
        let newer = TransferFeeEpoch {
            epoch: 720,
            basis_points: 50,
            maximum_fee: 4_000,
        };
        let schedule = TransferFeeSchedule { older, newer };
        // Before the newer setting: 1% of 1,000,000 = 10,000.
        assert_eq!(schedule.fee_at(710, 1_000_000), Some(10_000));
        // From the newer setting: 0.5%, capped at 4,000.
        assert_eq!(schedule.fee_at(720, 1_000_000), Some(4_000));
        assert_eq!(schedule.fee_at(723, 100_000), Some(500));
        assert_eq!(schedule.fee_at(723, 0), Some(0));
    }
}
