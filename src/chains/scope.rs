// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Which chains an aggregate read or a refresh covers.
//!
//! Aggregates (sums of counts, unions of unordered sets, readiness checks,
//! refresh triggers) take a [`ChainScope`]. Ordered or paged reads take one
//! `ChainId`; a surface whose input carries no chain yet asks the scope for
//! its single chain with [`ChainScope::sole_chain`], which fails instead of
//! choosing when more than one chain is enabled.

use crate::chains::registry::enabled_chains;
use crate::chains::{ChainId, Error, Result};

/// The chains an aggregate covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainScope {
    /// Every enabled chain.
    All,
    /// Exactly one chain.
    One(ChainId),
}

impl ChainScope {
    /// Enabled chains covered, in enabled order; `One` of a disabled chain
    /// covers none.
    pub fn chains(self) -> Vec<ChainId> {
        match self {
            Self::All => enabled_chains().to_vec(),
            Self::One(chain) => enabled_chains()
                .iter()
                .copied()
                .filter(|&enabled| enabled == chain)
                .collect(),
        }
    }

    /// The single chain this scope covers.
    ///
    /// `One` of an enabled chain is that chain, `One` of a disabled chain is
    /// [`Error::ChainNotEnabled`], and `All` is the only enabled chain, or
    /// [`Error::AmbiguousChain`] when that is not exactly one.
    pub fn sole_chain(self) -> Result<ChainId> {
        match self {
            Self::One(chain) => {
                if enabled_chains().contains(&chain) {
                    Ok(chain)
                } else {
                    Err(Error::ChainNotEnabled { chain })
                }
            }
            Self::All => match enabled_chains() {
                [only] => Ok(*only),
                candidates => Err(Error::AmbiguousChain {
                    candidates: candidates.to_vec(),
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChainScope;
    use crate::chains::{enabled_chains, ChainId};

    #[test]
    fn all_covers_every_enabled_chain() {
        assert_eq!(ChainScope::All.chains(), enabled_chains());
    }

    #[test]
    fn one_covers_its_enabled_chain() {
        assert_eq!(
            ChainScope::One(ChainId::Solana).chains(),
            vec![ChainId::Solana]
        );
        assert_eq!(
            ChainScope::One(ChainId::Solana).sole_chain().unwrap(),
            ChainId::Solana
        );
    }

    #[test]
    fn all_resolves_the_single_enabled_chain() {
        assert_eq!(enabled_chains(), [ChainId::Solana]);
        assert_eq!(ChainScope::All.sole_chain().unwrap(), ChainId::Solana);
    }
}
