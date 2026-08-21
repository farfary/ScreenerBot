//! Canonical conversion of a `PoolDescriptor`'s reserve `AccountId`s to Solana `Pubkey`s.
//!
//! Solana-owned: the chain-neutral `PoolDescriptor` (`crate::pools::types`) carries
//! reserve accounts as chain-qualified `AccountId` strings. This is the sole place
//! that parses those strings into `Pubkey`s for pricing — it is all-or-error: a
//! single malformed address, or a descriptor that is not a Solana pool, rejects the
//! whole conversion. Callers must never fall back to a partial/filtered list, since
//! that would silently shrink the required reserve set and let a pool price off an
//! incomplete account bundle.

use std::str::FromStr;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::{ChainId, PoolId};
use crate::pools::types::PoolDescriptor;

/// Why a `PoolDescriptor`'s reserve accounts could not be converted to `Pubkey`s.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ReserveAccountError {
    /// The descriptor's pool identity is not on the Solana chain.
    #[error("pool {pool_id} is not a Solana pool")]
    WrongChain { pool_id: PoolId },
    /// One reserve address did not parse as a Solana `Pubkey`.
    #[error("pool {pool_id} has an invalid reserve address at index {index}")]
    InvalidAddress { pool_id: PoolId, index: usize },
}

/// Converts every reserve `AccountId` on `descriptor` into a `Pubkey`, preserving
/// order and length exactly. Rejects the whole descriptor (never drops an entry)
/// if any address fails to parse or the descriptor is not a Solana pool.
pub fn reserve_pubkeys(descriptor: &PoolDescriptor) -> Result<Vec<Pubkey>, ReserveAccountError> {
    if descriptor.pool_id.chain() != ChainId::Solana {
        return Err(ReserveAccountError::WrongChain {
            pool_id: descriptor.pool_id.clone(),
        });
    }

    descriptor
        .reserve_accounts
        .iter()
        .enumerate()
        .map(|(index, account)| {
            Pubkey::from_str(account.address()).map_err(|_| ReserveAccountError::InvalidAddress {
                pool_id: descriptor.pool_id.clone(),
                index,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::{AccountId, AssetId};
    use crate::pools::types::ProtocolId;
    use std::time::Instant;

    fn descriptor(reserves: Vec<&str>) -> PoolDescriptor {
        PoolDescriptor {
            pool_id: PoolId::new(ChainId::Solana, "PoolAddr111").unwrap(),
            program_kind: ProtocolId::new("RAYDIUM CPMM"),
            base_mint: AssetId::new(ChainId::Solana, "TokenMint111").unwrap(),
            quote_mint: AssetId::new(
                ChainId::Solana,
                "So11111111111111111111111111111111111111112",
            )
            .unwrap(),
            reserve_accounts: reserves
                .into_iter()
                .map(|addr| AccountId::new(ChainId::Solana, addr).unwrap())
                .collect(),
            liquidity_usd: 0.0,
            volume_h24_usd: 0.0,
            last_updated: Instant::now(),
        }
    }

    #[test]
    fn all_valid_addresses_preserve_exact_order_and_length() {
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        let d = descriptor(vec![&a.to_string(), &b.to_string()]);

        let pubkeys = reserve_pubkeys(&d).expect("all-valid conversion succeeds");

        assert_eq!(pubkeys, vec![a, b]);
    }

    #[test]
    fn one_malformed_address_rejects_the_whole_descriptor() {
        let a = Pubkey::new_unique();
        let d = descriptor(vec![&a.to_string(), "not-a-valid-pubkey"]);

        let err = reserve_pubkeys(&d).expect_err("malformed reserve must reject, not shrink");

        assert!(matches!(
            err,
            ReserveAccountError::InvalidAddress { index: 1, .. }
        ));
    }

    #[test]
    fn empty_reserves_produce_empty_vec_not_an_error() {
        let d = descriptor(vec![]);
        assert_eq!(reserve_pubkeys(&d).unwrap(), Vec::<Pubkey>::new());
    }
}
