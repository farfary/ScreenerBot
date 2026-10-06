// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pinned EVM contract addresses, one module per chain.
//!
//! Single source of truth for every EVM contract the app calls or trusts.
//! Each address carries its proof status: an entry stays
//! [`Verification::Pending`](super::spec::Verification::Pending) until the
//! live constants test proves its code and identity on chain, and only a
//! proven entry yields an address to money paths.

/// Base mainnet (chain id 8453).
pub mod base {
    use crate::chains::evm::spec::PinnedContract;

    /// Wrapped ether predeploy.
    pub const WETH: PinnedContract =
        PinnedContract::pending("WETH", "0x4200000000000000000000000000000000000006");
    /// Native Circle USDC.
    pub const USDC: PinnedContract =
        PinnedContract::pending("USDC", "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
    /// Multicall3.
    pub const MULTICALL3: PinnedContract =
        PinnedContract::pending("Multicall3", "0xcA11bde05977b3631167028862bE2a173976CA11");
    /// Uniswap Permit2.
    pub const PERMIT2: PinnedContract =
        PinnedContract::pending("Permit2", "0x000000000022D473030F116dDEE9F6B43aC78BA3");
    /// OP Stack gas price oracle predeploy.
    pub const GAS_PRICE_ORACLE: PinnedContract = PinnedContract::pending(
        "GasPriceOracle",
        "0x420000000000000000000000000000000000000F",
    );

    /// Uniswap V2 pair factory.
    pub const UNISWAP_V2_FACTORY: PinnedContract = PinnedContract::pending(
        "Uniswap V2 Factory",
        "0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6",
    );
    /// Uniswap V2 Router02.
    pub const UNISWAP_V2_ROUTER: PinnedContract = PinnedContract::pending(
        "Uniswap V2 Router02",
        "0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24",
    );

    /// Uniswap V3 pool factory.
    pub const UNISWAP_V3_FACTORY: PinnedContract = PinnedContract::pending(
        "Uniswap V3 Factory",
        "0x33128a8fC17869897dcE68Ed026d694621f6FDfD",
    );
    /// Uniswap V3 QuoterV2.
    pub const UNISWAP_V3_QUOTER: PinnedContract = PinnedContract::pending(
        "Uniswap V3 QuoterV2",
        "0x3d4e44Eb1374240CE5F1B871ab261CD16335B76a",
    );
    /// Uniswap V3 SwapRouter02.
    pub const UNISWAP_V3_ROUTER: PinnedContract = PinnedContract::pending(
        "Uniswap V3 SwapRouter02",
        "0x2626664c2603336E57B271c5C0b26F421741e481",
    );

    /// Uniswap V4 PoolManager.
    pub const UNISWAP_V4_POOL_MANAGER: PinnedContract = PinnedContract::pending(
        "Uniswap V4 PoolManager",
        "0x498581fF718922c3f8e6A244956aF099B2652b2b",
    );
    /// Uniswap V4 StateView.
    pub const UNISWAP_V4_STATE_VIEW: PinnedContract = PinnedContract::pending(
        "Uniswap V4 StateView",
        "0xA3c0c9b65baD0b08107Aa264b0f3dB444b867A71",
    );
    /// Uniswap V4 Quoter.
    pub const UNISWAP_V4_QUOTER: PinnedContract = PinnedContract::pending(
        "Uniswap V4 Quoter",
        "0x0d5e0F971ED27FBfF6c2837bf31316121532048D",
    );
    /// Uniswap V4 PositionManager.
    pub const UNISWAP_V4_POSITION_MANAGER: PinnedContract = PinnedContract::pending(
        "Uniswap V4 PositionManager",
        "0x7C5f5A4bBd8fD63184577525326123B519429bDc",
    );

    /// Uniswap Universal Router 2.1.2, the pinned direct swap router.
    pub const UNIVERSAL_ROUTER: PinnedContract = PinnedContract::pending(
        "Uniswap Universal Router 2.1.2",
        "0xd6145b2D3F379919E8CdEda7B97e37c4b2Ca9c40",
    );

    /// Aerodrome V2 pool factory.
    pub const AERODROME_POOL_FACTORY: PinnedContract = PinnedContract::pending(
        "Aerodrome PoolFactory",
        "0x420DD381b31aEf6683db6B902084cB0FFECe40Da",
    );
    /// Aerodrome V2 router.
    pub const AERODROME_ROUTER: PinnedContract = PinnedContract::pending(
        "Aerodrome Router",
        "0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43",
    );

    /// Aerodrome Slipstream CLFactory, the deployment served by the
    /// Slipstream router and quoter below.
    pub const SLIPSTREAM_CL_FACTORY: PinnedContract = PinnedContract::pending(
        "Slipstream CLFactory",
        "0x5e7BB104d84c7CB9B682AaC2F3d509f5F406809A",
    );
    /// Aerodrome Slipstream QuoterV2.
    pub const SLIPSTREAM_QUOTER: PinnedContract = PinnedContract::pending(
        "Slipstream QuoterV2",
        "0x254cF9E1E6e233aa1AC962CB9B05b2cfeAaE15b0",
    );
    /// Aerodrome Slipstream SwapRouter.
    pub const SLIPSTREAM_ROUTER: PinnedContract = PinnedContract::pending(
        "Slipstream SwapRouter",
        "0xBE6D8f0d05cC4be24d5167a3eF062215bE6D18a5",
    );
}
