// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The static inspection of a third-party build: every top-level instruction
//! must be one a swap needs, with the wallet's own accounts where it names any.

use std::str::FromStr;

use super::error::BuildRefusal;
use super::BuildExpectations;
use crate::chains::solana::constants::{
    ASSOCIATED_TOKEN_PROGRAM_ID, SOL_MINT, SPL_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
};
use crate::chains::solana::rpc::types::LoadedAddresses;
use crate::chains::solana::solana_sdk::{
    compute_budget::id as compute_budget_program, instruction::CompiledInstruction,
    message::VersionedMessage, pubkey::Pubkey, transaction::VersionedTransaction,
};
use crate::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id;
use crate::chains::solana::swaps::direct::compute::MAX_COMPUTE_UNITS;
use crate::chains::solana::swaps::revenue;
use crate::chains::solana::transactions::program_ids::{
    detect_router_from_program_id, is_mev_tip_address,
};
use crate::swaps::Quote;

/// Compute-budget instruction tags.
const REQUEST_HEAP_FRAME: u8 = 1;
const SET_COMPUTE_UNIT_LIMIT: u8 = 2;
const SET_COMPUTE_UNIT_PRICE: u8 = 3;
const SET_LOADED_ACCOUNTS_DATA_SIZE_LIMIT: u8 = 4;

/// Associated-token-account instruction tags. An empty instruction is the
/// original `Create`.
const ATA_CREATE: u8 = 0;
const ATA_CREATE_IDEMPOTENT: u8 = 1;

/// The system program's `Transfer` tag, a little-endian u32 before the
/// lamports.
const SYSTEM_TRANSFER: u32 = 2;

/// Token-program instruction tags shared by SPL Token and Token-2022.
const TOKEN_CLOSE_ACCOUNT: u8 = 9;
const TOKEN_SYNC_NATIVE: u8 = 17;

/// Every account a message can name, in the order instructions index them:
/// the static keys, then the loaded writable addresses, then the loaded
/// readonly addresses. A loaded address no simulation resolved is `None`.
#[derive(Debug, Clone)]
pub struct ResolvedKeys {
    keys: Vec<Option<Pubkey>>,
}

impl ResolvedKeys {
    /// The keys of `message`, extended by `loaded` when it accounts for
    /// exactly the addresses the message's lookup tables load.
    pub fn new(message: &VersionedMessage, loaded: Option<&LoadedAddresses>) -> Self {
        let mut keys: Vec<Option<Pubkey>> = message
            .static_account_keys()
            .iter()
            .copied()
            .map(Some)
            .collect();
        let VersionedMessage::V0(v0) = message else {
            return Self { keys };
        };
        let writable: usize = v0
            .address_table_lookups
            .iter()
            .map(|lookup| lookup.writable_indexes.len())
            .sum();
        let readonly: usize = v0
            .address_table_lookups
            .iter()
            .map(|lookup| lookup.readonly_indexes.len())
            .sum();
        let resolved = loaded.filter(|loaded| {
            loaded.writable.len() == writable && loaded.readonly.len() == readonly
        });
        match resolved {
            Some(loaded) => keys.extend(
                loaded
                    .writable
                    .iter()
                    .chain(&loaded.readonly)
                    .map(|address| Pubkey::from_str(address).ok()),
            ),
            None => keys.resize(keys.len() + writable + readonly, None),
        }
        Self { keys }
    }

    /// The account at `index`, `None` when it was loaded from a lookup table
    /// no simulation resolved or lies outside the message.
    pub fn get(&self, index: u8) -> Option<Pubkey> {
        self.keys.get(usize::from(index)).copied().flatten()
    }

    /// How many accounts the message names, resolved or not.
    pub fn count(&self) -> usize {
        self.keys.len()
    }
}

/// The account in `slot` of `instruction`, or the refusal detail that names it.
fn account(
    keys: &ResolvedKeys,
    position: usize,
    instruction: &CompiledInstruction,
    slot: usize,
) -> Result<Pubkey, BuildRefusal> {
    let index = *instruction
        .accounts
        .get(slot)
        .ok_or(BuildRefusal::AccountMissing { position, slot })?;
    if usize::from(index) >= keys.count() {
        return Err(BuildRefusal::AccountOutsideMessage {
            position,
            slot,
            index,
        });
    }
    keys.get(index).ok_or(BuildRefusal::AccountUnresolved {
        position,
        slot,
        index,
    })
}

/// Refuse unless the account in `slot` is the wallet.
fn require_wallet(
    keys: &ResolvedKeys,
    position: usize,
    instruction: &CompiledInstruction,
    slot: usize,
    wallet: &Pubkey,
    role: &'static str,
) -> Result<(), BuildRefusal> {
    let found = account(keys, position, instruction, slot)?;
    if &found != wallet {
        return Err(BuildRefusal::NotTheWallet {
            position,
            found,
            role,
        });
    }
    Ok(())
}

/// A little-endian integer of `N` bytes at `data[1..]`, when the instruction
/// is exactly a tag and that integer.
fn tagged_le<const N: usize>(data: &[u8]) -> Option<[u8; N]> {
    match data.len() == N + 1 {
        true => data[1..].try_into().ok(),
        false => None,
    }
}

/// Inspect every top-level instruction of `transaction` against the
/// allowlist, naming the first thing a swap does not need.
pub fn inspect(
    transaction: &VersionedTransaction,
    keys: &ResolvedKeys,
    quote: &Quote,
    expectations: &BuildExpectations,
) -> Result<(), BuildRefusal> {
    let wallet =
        Pubkey::from_str(&quote.wallet_address).map_err(|_| BuildRefusal::WalletNotAnAddress {
            wallet: quote.wallet_address.clone(),
        })?;
    let constant = |address: &str| {
        Pubkey::from_str(address).map_err(|error| BuildRefusal::ConstantInvalid {
            detail: error.to_string(),
        })
    };
    let wrapped_sol_account = get_associated_token_address_with_program_id(
        &wallet,
        &constant(SOL_MINT)?,
        &constant(SPL_TOKEN_PROGRAM_ID)?,
    );
    let fee_wallet = revenue::fee_wallet_pubkey();
    let wrap_bound = if crate::chains::adapter().is_native_asset(&quote.input_mint) {
        u64::try_from(quote.input_amount).map_err(|_| BuildRefusal::QuoteAmountOutOfRange {
            leg: "input",
            limit: "Solana u64 limit",
        })?
    } else {
        expectations.allowance
    };

    let mut wrapped: u64 = 0;
    let mut fee_paid: u64 = 0;
    let mut tipped: u64 = 0;
    let static_keys = transaction.message.static_account_keys();
    for (position, instruction) in transaction.message.instructions().iter().enumerate() {
        let program = *static_keys
            .get(usize::from(instruction.program_id_index))
            .ok_or(BuildRefusal::ProgramOutsideMessage { position })?;
        let program_id = program.to_string();
        let data = instruction.data.as_slice();

        if program == compute_budget_program() {
            match data.first().copied() {
                Some(REQUEST_HEAP_FRAME) | Some(SET_LOADED_ACCOUNTS_DATA_SIZE_LIMIT)
                    if tagged_le::<4>(data).is_some() => {}
                Some(SET_COMPUTE_UNIT_LIMIT) => {
                    let limit = tagged_le::<4>(data).map(u32::from_le_bytes).ok_or(
                        BuildRefusal::Malformed {
                            position,
                            kind: "limit",
                        },
                    )?;
                    if limit > MAX_COMPUTE_UNITS {
                        return Err(BuildRefusal::ComputeLimitTooHigh {
                            position,
                            limit,
                            maximum: MAX_COMPUTE_UNITS,
                        });
                    }
                }
                Some(SET_COMPUTE_UNIT_PRICE) => {
                    let price = tagged_le::<8>(data).map(u64::from_le_bytes).ok_or(
                        BuildRefusal::Malformed {
                            position,
                            kind: "price",
                        },
                    )?;
                    if price > expectations.max_compute_unit_price {
                        return Err(BuildRefusal::ComputePriceTooHigh {
                            position,
                            price,
                            maximum: expectations.max_compute_unit_price,
                        });
                    }
                }
                _ => return Err(BuildRefusal::ComputeBudgetUnexpected { position }),
            }
            continue;
        }

        match program_id.as_str() {
            ASSOCIATED_TOKEN_PROGRAM_ID => {
                if !matches!(data, [] | [ATA_CREATE] | [ATA_CREATE_IDEMPOTENT]) {
                    return Err(BuildRefusal::AssociatedTokenNotCreate { position });
                }
                require_wallet(keys, position, instruction, 0, &wallet, "payer")?;
                require_wallet(keys, position, instruction, 2, &wallet, "owner")?;
            }
            SYSTEM_PROGRAM_ID => {
                let lamports = match data.split_at_checked(4) {
                    Some((tag, amount))
                        if tag == SYSTEM_TRANSFER.to_le_bytes() && amount.len() == 8 =>
                    {
                        u64::from_le_bytes(amount.try_into().map_err(|_| {
                            BuildRefusal::Malformed {
                                position,
                                kind: "transfer",
                            }
                        })?)
                    }
                    _ => return Err(BuildRefusal::SystemNotTransfer { position }),
                };
                require_wallet(keys, position, instruction, 0, &wallet, "transfer source")?;
                let destination = account(keys, position, instruction, 1)?;
                let total = if destination == wrapped_sol_account {
                    &mut wrapped
                } else if destination == fee_wallet {
                    &mut fee_paid
                } else if is_mev_tip_address(&destination.to_string()) {
                    &mut tipped
                } else {
                    return Err(BuildRefusal::TransferToStranger {
                        position,
                        lamports,
                        destination,
                    });
                };
                *total = total
                    .checked_add(lamports)
                    .ok_or(BuildRefusal::TransferSumOverflow { position })?;
            }
            SPL_TOKEN_PROGRAM_ID | TOKEN_2022_PROGRAM_ID => match data {
                [TOKEN_SYNC_NATIVE] => {}
                [TOKEN_CLOSE_ACCOUNT] => {
                    require_wallet(keys, position, instruction, 1, &wallet, "close destination")?;
                    require_wallet(keys, position, instruction, 2, &wallet, "close authority")?;
                }
                _ => return Err(BuildRefusal::TokenInstructionUnexpected { position }),
            },
            _ => {
                if detect_router_from_program_id(&program_id) != Some(quote.router_id.as_str()) {
                    return Err(BuildRefusal::ProgramNotOfRouter {
                        position,
                        program: program_id.clone(),
                        router: quote.router_id.clone(),
                    });
                }
            }
        }
    }

    if wrapped > wrap_bound {
        return Err(BuildRefusal::WrapAboveBound {
            wrapped,
            bound: wrap_bound,
        });
    }
    if fee_paid > expectations.fee_bound {
        return Err(BuildRefusal::FeeAboveBound {
            paid: fee_paid,
            bound: expectations.fee_bound,
        });
    }
    if tipped > expectations.allowance {
        return Err(BuildRefusal::TipAboveAllowance {
            tipped,
            allowance: expectations.allowance,
        });
    }
    Ok(())
}
