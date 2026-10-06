// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Per-chain facts of an EVM chain as data: one `EvmChainSpec` per chain.
//!
//! Every EVM chain shares one implementation parameterized by its spec:
//! EIP-155 chain id, native currency, pinned contract addresses, venue set,
//! Uniswap V4 hook allow-list, measured public RPC limits, finality and L1
//! data-fee policy. A chain that needs code beyond data gets its own module.

use alloy::primitives::{hex, Address};

use super::constants::base;

/// Whether a pinned contract address has passed its on-chain identity test
/// (non-empty code plus the identifying call recorded for it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verification {
    /// Transcribed from a recorded source; not yet proven on chain.
    Pending,
    /// Proven on chain by the live constants test.
    ProvenOnChain,
}

/// A contract address pinned for one chain, carrying its proof status.
///
/// The address is reachable for money paths only through [`Self::proven`],
/// which stays `None` until the on-chain identity test passes for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PinnedContract {
    label: &'static str,
    eip55: &'static str,
    address: Address,
    verification: Verification,
}

impl PinnedContract {
    /// Pins the EIP-55 address `eip55` under `label` before its on-chain
    /// identity test. Pinned contracts are const items, so a literal that is
    /// not 20 hex bytes fails the build.
    pub const fn pending(label: &'static str, eip55: &'static str) -> Self {
        let address = match hex::const_decode_to_array::<20>(eip55.as_bytes()) {
            Ok(bytes) => Address::new(bytes),
            Err(_) => panic!("a pinned contract address must be 20 hex bytes"),
        };
        Self {
            label,
            eip55,
            address,
            verification: Verification::Pending,
        }
    }

    /// Human-readable contract name used in proof reports.
    pub const fn label(&self) -> &'static str {
        self.label
    }

    /// The address as written, in EIP-55 mixed-case form.
    pub const fn eip55(&self) -> &'static str {
        self.eip55
    }

    /// The proof status of this address.
    pub const fn verification(&self) -> Verification {
        self.verification
    }

    /// The address, only once its on-chain identity test has passed.
    pub const fn proven(&self) -> Option<Address> {
        match self.verification {
            Verification::ProvenOnChain => Some(self.address),
            Verification::Pending => None,
        }
    }

    /// The address as pinned, proven or not. For the on-chain identity test
    /// and proof reports only; money paths take [`Self::proven`].
    pub const fn claimed(&self) -> Address {
        self.address
    }
}

/// The chain's native gas currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeCurrency {
    /// Conventional symbol.
    pub symbol: &'static str,
    /// Fractional base-unit digits (wei per whole unit is `10^decimals`).
    pub decimals: u8,
}

/// The chain's slug at each multi-chain market-data provider. Slugs differ
/// per provider and are never derived from the internal chain id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProviderSlugs {
    /// GeckoTerminal network id.
    pub gecko_terminal: &'static str,
    /// DexScreener chain id.
    pub dex_screener: &'static str,
    /// CoinGecko asset-platform id.
    pub coin_gecko: &'static str,
}

/// Block explorer page prefixes; the address or transaction hash is appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExplorerUrls {
    /// Token page prefix.
    pub token: &'static str,
    /// Account page prefix.
    pub account: &'static str,
    /// Transaction page prefix.
    pub transaction: &'static str,
}

/// When a landed transaction counts as final.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FinalityPolicy {
    /// OP Stack rollup: a `status = 1` receipt in a canonical block confirms
    /// and its block hash is recorded; the hash is re-checked once the `safe`
    /// head covers the block, and a mismatch re-pends the transaction (a
    /// reorged-out transaction can land again). The `pending` block tag and
    /// pre-confirmation APIs are never read.
    OpStack,
}

/// How the L1 data fee of a transaction is estimated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum L1DataFee {
    /// OP Stack `GasPriceOracle` predeploy, `getL1FeeUpperBound(txSize)`.
    OpStackOracle(PinnedContract),
}

/// A DEX venue family supported on EVM chains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmVenue {
    /// Uniswap V2 constant-product pairs.
    UniswapV2,
    /// Uniswap V3 concentrated-liquidity pools.
    UniswapV3,
    /// Uniswap V4 singleton pools (hookless or allow-listed hooks).
    UniswapV4,
    /// Aerodrome V2 volatile pools; stable pools are refused.
    AerodromeV2,
    /// Aerodrome Slipstream concentrated-liquidity pools.
    AerodromeSlipstream,
}

/// The contracts one venue deployment is reached through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VenueContracts {
    /// Constant-product pair factory and its router.
    ConstantProduct {
        /// Pair factory.
        factory: PinnedContract,
        /// Swap router bound to `factory`.
        router: PinnedContract,
    },
    /// Concentrated-liquidity pool factory, its quoter and its swap router.
    ConcentratedLiquidity {
        /// Pool factory.
        factory: PinnedContract,
        /// Quoter bound to `factory`.
        quoter: PinnedContract,
        /// Swap router bound to `factory`.
        router: PinnedContract,
    },
    /// Singleton pool manager with its read lens, quoter and position manager.
    Singleton {
        /// The pool manager holding every pool's state.
        pool_manager: PinnedContract,
        /// Read lens over pool state (`getSlot0`, `getLiquidity`).
        state_view: PinnedContract,
        /// Quoter bound to `pool_manager`.
        quoter: PinnedContract,
        /// Position manager bound to `pool_manager`.
        position_manager: PinnedContract,
    },
}

impl VenueContracts {
    /// Every contract of this deployment.
    pub fn contracts(&self) -> Vec<PinnedContract> {
        match *self {
            Self::ConstantProduct { factory, router } => vec![factory, router],
            Self::ConcentratedLiquidity {
                factory,
                quoter,
                router,
            } => vec![factory, quoter, router],
            Self::Singleton {
                pool_manager,
                state_view,
                quoter,
                position_manager,
            } => vec![pool_manager, state_view, quoter, position_manager],
        }
    }
}

/// One deployment of a venue family on a chain. A venue with several
/// factory deployments lists each one separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VenueDeployment {
    /// The venue family.
    pub venue: EvmVenue,
    /// The deployment's contracts.
    pub contracts: VenueContracts,
}

/// What an RPC endpoint is used for by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndpointRole {
    /// Sends, simulations, receipts and first-choice reads.
    Primary,
    /// Reads only, when the primary endpoint does not answer.
    ReadFallback,
}

/// A keyless public RPC endpoint with its measured per-request limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RpcEndpointDefaults {
    /// HTTPS endpoint URL.
    pub url: &'static str,
    /// Default role of this endpoint.
    pub role: EndpointRole,
    /// Largest JSON-RPC batch the endpoint accepts.
    pub max_batch_calls: u16,
    /// Widest `eth_getLogs` block span the endpoint accepts.
    pub max_log_span_blocks: u64,
}

/// Everything chain-specific about one EVM chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EvmChainSpec {
    /// EIP-155 chain id.
    pub chain_id: u64,
    /// Native gas currency.
    pub native: NativeCurrency,
    /// Target block interval in milliseconds.
    pub block_time_ms: u64,
    /// The wrapped native token, the chain's native-asset policy address.
    pub wrapped_native: PinnedContract,
    /// Canonical stablecoin quote assets.
    pub stable_assets: &'static [PinnedContract],
    /// Multicall3, the batching contract for block-stamped reads.
    pub multicall3: PinnedContract,
    /// Permit2, the only spender a bought token is approved to.
    pub permit2: PinnedContract,
    /// The pinned Uniswap Universal Router of the direct swap path.
    pub universal_router: PinnedContract,
    /// Venue deployments priced and traded on this chain.
    pub venues: &'static [VenueDeployment],
    /// Uniswap V4 hooks whose pools may be priced; a pool with any other
    /// hook is refused.
    pub v4_hook_allow_list: &'static [PinnedContract],
    /// Market-data provider slugs.
    pub provider_slugs: ProviderSlugs,
    /// Block explorer page prefixes.
    pub explorer: ExplorerUrls,
    /// Finality policy.
    pub finality: FinalityPolicy,
    /// L1 data-fee model.
    pub l1_data_fee: L1DataFee,
    /// Default public RPC endpoints, primary first.
    pub rpc_endpoints: &'static [RpcEndpointDefaults],
}

impl EvmChainSpec {
    /// Every pinned contract of this chain, each listed once.
    pub fn pinned_contracts(&self) -> Vec<PinnedContract> {
        let mut contracts = vec![
            self.wrapped_native,
            self.multicall3,
            self.permit2,
            self.universal_router,
        ];
        contracts.extend_from_slice(self.stable_assets);
        for deployment in self.venues {
            contracts.extend(deployment.contracts.contracts());
        }
        contracts.extend_from_slice(self.v4_hook_allow_list);
        match self.l1_data_fee {
            L1DataFee::OpStackOracle(oracle) => contracts.push(oracle),
        }
        contracts
    }
}

/// Base mainnet (OP Stack L2).
pub const BASE: EvmChainSpec = EvmChainSpec {
    chain_id: 8453,
    native: NativeCurrency {
        symbol: "ETH",
        decimals: 18,
    },
    block_time_ms: 2_000,
    wrapped_native: base::WETH,
    stable_assets: &[base::USDC],
    multicall3: base::MULTICALL3,
    permit2: base::PERMIT2,
    universal_router: base::UNIVERSAL_ROUTER,
    venues: &[
        VenueDeployment {
            venue: EvmVenue::UniswapV2,
            contracts: VenueContracts::ConstantProduct {
                factory: base::UNISWAP_V2_FACTORY,
                router: base::UNISWAP_V2_ROUTER,
            },
        },
        VenueDeployment {
            venue: EvmVenue::UniswapV3,
            contracts: VenueContracts::ConcentratedLiquidity {
                factory: base::UNISWAP_V3_FACTORY,
                quoter: base::UNISWAP_V3_QUOTER,
                router: base::UNISWAP_V3_ROUTER,
            },
        },
        VenueDeployment {
            venue: EvmVenue::UniswapV4,
            contracts: VenueContracts::Singleton {
                pool_manager: base::UNISWAP_V4_POOL_MANAGER,
                state_view: base::UNISWAP_V4_STATE_VIEW,
                quoter: base::UNISWAP_V4_QUOTER,
                position_manager: base::UNISWAP_V4_POSITION_MANAGER,
            },
        },
        VenueDeployment {
            venue: EvmVenue::AerodromeV2,
            contracts: VenueContracts::ConstantProduct {
                factory: base::AERODROME_POOL_FACTORY,
                router: base::AERODROME_ROUTER,
            },
        },
        VenueDeployment {
            venue: EvmVenue::AerodromeSlipstream,
            contracts: VenueContracts::ConcentratedLiquidity {
                factory: base::SLIPSTREAM_CL_FACTORY,
                quoter: base::SLIPSTREAM_QUOTER,
                router: base::SLIPSTREAM_ROUTER,
            },
        },
    ],
    v4_hook_allow_list: &[],
    provider_slugs: ProviderSlugs {
        gecko_terminal: "base",
        dex_screener: "base",
        coin_gecko: "base",
    },
    explorer: ExplorerUrls {
        token: "https://basescan.org/token/",
        account: "https://basescan.org/address/",
        transaction: "https://basescan.org/tx/",
    },
    finality: FinalityPolicy::OpStack,
    l1_data_fee: L1DataFee::OpStackOracle(base::GAS_PRICE_ORACLE),
    rpc_endpoints: &[
        RpcEndpointDefaults {
            url: "https://mainnet.base.org",
            role: EndpointRole::Primary,
            max_batch_calls: 10,
            max_log_span_blocks: 1_000,
        },
        RpcEndpointDefaults {
            url: "https://base-rpc.publicnode.com",
            role: EndpointRole::ReadFallback,
            max_batch_calls: 20,
            max_log_span_blocks: 5_000,
        },
    ],
};

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn base_identity_matches_the_chain() {
        assert_eq!(BASE.chain_id, 8453);
        assert_eq!(BASE.native.symbol, "ETH");
        assert_eq!(BASE.native.decimals, 18);
        assert_eq!(BASE.finality, FinalityPolicy::OpStack);
        assert_eq!(
            BASE.provider_slugs,
            ProviderSlugs {
                gecko_terminal: "base",
                dex_screener: "base",
                coin_gecko: "base",
            }
        );
    }

    #[test]
    fn pinned_contracts_are_distinct_nonzero_and_labelled_once() {
        let contracts = BASE.pinned_contracts();
        let mut addresses = HashSet::new();
        let mut labels = HashSet::new();
        for contract in &contracts {
            assert_ne!(contract.claimed(), Address::ZERO, "{}", contract.label());
            assert!(
                addresses.insert(contract.claimed()),
                "{} repeats an address",
                contract.label()
            );
            assert!(
                labels.insert(contract.label()),
                "{} repeats a label",
                contract.label()
            );
        }
    }

    #[test]
    fn pinned_addresses_are_written_with_valid_eip55_checksums() {
        for contract in BASE.pinned_contracts() {
            assert_eq!(
                Address::parse_checksummed(contract.eip55(), None).ok(),
                Some(contract.claimed()),
                "{} is not a valid EIP-55 address: {}",
                contract.label(),
                contract.eip55()
            );
        }
    }

    #[test]
    fn every_venue_family_has_a_deployment() {
        for venue in [
            EvmVenue::UniswapV2,
            EvmVenue::UniswapV3,
            EvmVenue::UniswapV4,
            EvmVenue::AerodromeV2,
            EvmVenue::AerodromeSlipstream,
        ] {
            assert!(
                BASE.venues.iter().any(|d| d.venue == venue),
                "{venue:?} has no deployment"
            );
        }
    }

    #[test]
    fn unproven_contracts_expose_no_money_path_address() {
        for contract in BASE.pinned_contracts() {
            match contract.verification() {
                Verification::Pending => assert_eq!(contract.proven(), None),
                Verification::ProvenOnChain => {
                    assert_eq!(contract.proven(), Some(contract.claimed()))
                }
            }
        }
    }

    #[test]
    fn v4_hook_allow_list_starts_empty() {
        assert!(BASE.v4_hook_allow_list.is_empty());
    }

    #[test]
    fn rpc_defaults_have_one_primary_and_usable_limits() {
        let primaries = BASE
            .rpc_endpoints
            .iter()
            .filter(|e| e.role == EndpointRole::Primary)
            .count();
        assert_eq!(primaries, 1);
        assert_eq!(BASE.rpc_endpoints[0].role, EndpointRole::Primary);
        for endpoint in BASE.rpc_endpoints {
            assert!(endpoint.url.starts_with("https://"), "{}", endpoint.url);
            assert!(endpoint.max_batch_calls > 0, "{}", endpoint.url);
            assert!(endpoint.max_log_span_blocks > 0, "{}", endpoint.url);
        }
    }

    #[test]
    fn explorer_prefixes_end_with_a_path_separator() {
        for prefix in [
            BASE.explorer.token,
            BASE.explorer.account,
            BASE.explorer.transaction,
        ] {
            assert!(prefix.starts_with("https://"), "{prefix}");
            assert!(prefix.ends_with('/'), "{prefix}");
        }
    }
}
