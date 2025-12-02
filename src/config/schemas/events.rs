use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// EVENTS SYSTEM
// ============================================================================

config_struct! {
    /// Events system configuration
    ///
    /// WARNING: The events system stores detailed operational data and can generate
    /// 5+ GB of database storage per day. It is intended for debugging and development
    /// purposes only. Disable in production to reduce disk usage.
    pub struct EventsConfig {
        // ==================== GLOBAL CONTROL ====================
        #[metadata(field_metadata! {
            label: "Enable Events System",
            hint: "WARNING: Events can generate 5+ GB/day. Only enable for debugging/development.",
            impact: "critical",
            category: "Global Control",
        })]
        enabled: bool = false,

        // ==================== CATEGORY RECORDING ====================
        #[metadata(field_metadata! {
            label: "Record Swap Events",
            hint: "Record swap execution events (Jupiter, DEX interactions)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_swap: bool = true,

        #[metadata(field_metadata! {
            label: "Record Transaction Events",
            hint: "Record blockchain transaction events (send, receive, confirm)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_transaction: bool = true,

        #[metadata(field_metadata! {
            label: "Record Pool Events",
            hint: "Record pool discovery, analysis, and price calculation events",
            impact: "medium",
            category: "Category Recording",
        })]
        record_pool: bool = true,

        #[metadata(field_metadata! {
            label: "Record Token Events",
            hint: "Record token-related events (blacklist, metadata, decimals)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_token: bool = true,

        #[metadata(field_metadata! {
            label: "Record System Events",
            hint: "Record system lifecycle events (startup, shutdown, errors)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_system: bool = true,

        #[metadata(field_metadata! {
            label: "Record Position Events",
            hint: "Record position management events (open, close, P&L updates)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_position: bool = true,

        #[metadata(field_metadata! {
            label: "Record Wallet Events",
            hint: "Record wallet events (balance changes, ATA management)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_wallet: bool = true,

        #[metadata(field_metadata! {
            label: "Record Trader Events",
            hint: "Record trader orchestration events (lifecycle, decision-making)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_trader: bool = true,

        #[metadata(field_metadata! {
            label: "Record OHLCV Events",
            hint: "Record OHLCV monitoring events (discovery, fetch, gaps, backfills)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_ohlcv: bool = true,

        #[metadata(field_metadata! {
            label: "Record RPC Events",
            hint: "Record RPC client events (requests, responses, errors)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_rpc: bool = true,

        #[metadata(field_metadata! {
            label: "Record API Events",
            hint: "Record external API events (DexScreener, GeckoTerminal, Jupiter API, etc.)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_api: bool = true,

        #[metadata(field_metadata! {
            label: "Record Security Events",
            hint: "Record security and risk assessment events",
            impact: "medium",
            category: "Category Recording",
        })]
        record_security: bool = true,

        #[metadata(field_metadata! {
            label: "Record Connectivity Events",
            hint: "Record connectivity and endpoint health monitoring events",
            impact: "medium",
            category: "Category Recording",
        })]
        record_connectivity: bool = true,

        #[metadata(field_metadata! {
            label: "Record Filtering Events",
            hint: "Record token filtering events (snapshot computation, token evaluation)",
            impact: "medium",
            category: "Category Recording",
        })]
        record_filtering: bool = true,
    }
}
