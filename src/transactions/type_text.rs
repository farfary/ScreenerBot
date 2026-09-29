//! Catalog text for a transaction's type.
//!
//! `TransactionType::kind()` is the stable discriminant; each kind has one
//! `transactions-type-<kind>` message (underscores written as hyphens).

use super::types::TransactionType;
use crate::i18n::{ids, UiArg, UiText};

/// The label of a stored `kind()` value. A value no kind produces reads as
/// unclassified.
pub fn kind_text(kind: &str) -> UiText {
    UiText::new(match kind {
        "buy" => ids::TRANSACTIONS_TYPE_BUY,
        "sell" => ids::TRANSACTIONS_TYPE_SELL,
        "swap" => ids::TRANSACTIONS_TYPE_SWAP,
        "sol_transfer" => ids::TRANSACTIONS_TYPE_SOL_TRANSFER,
        "token_transfer" => ids::TRANSACTIONS_TYPE_TOKEN_TRANSFER,
        "transfer" => ids::TRANSACTIONS_TYPE_TRANSFER,
        "dust" => ids::TRANSACTIONS_TYPE_DUST,
        "spam" => ids::TRANSACTIONS_TYPE_SPAM,
        "ata_create" => ids::TRANSACTIONS_TYPE_ATA_CREATE,
        "ata_close" => ids::TRANSACTIONS_TYPE_ATA_CLOSE,
        "ata" => ids::TRANSACTIONS_TYPE_ATA,
        "liquidity_add" => ids::TRANSACTIONS_TYPE_LIQUIDITY_ADD,
        "liquidity_remove" => ids::TRANSACTIONS_TYPE_LIQUIDITY_REMOVE,
        "nft" => ids::TRANSACTIONS_TYPE_NFT,
        "program" => ids::TRANSACTIONS_TYPE_PROGRAM,
        "compute" => ids::TRANSACTIONS_TYPE_COMPUTE,
        "failed" => ids::TRANSACTIONS_TYPE_FAILED,
        _ => ids::TRANSACTIONS_TYPE_UNKNOWN,
    })
}

fn with_detail(base: UiText, detail: &str) -> UiText {
    UiText::new(ids::TRANSACTIONS_TYPE_WITH_DETAIL)
        .arg("label", UiArg::Nested(Box::new(base)))
        .arg("detail", UiArg::Text(detail.to_owned()))
}

impl TransactionType {
    /// The type's label followed by the payload that identifies the entry.
    pub fn ui_text(&self) -> UiText {
        let base = kind_text(self.kind());
        match self {
            Self::SwapSolToToken { router, .. }
            | Self::SwapTokenToSol { router, .. }
            | Self::SwapTokenToToken { router, .. }
            | Self::LiquidityAdd { router, .. }
            | Self::LiquidityRemove { router, .. }
                if !router.is_empty() =>
            {
                with_detail(base, router)
            }
            Self::TokenTransfer { mint, amount, .. } => {
                UiText::new(ids::TRANSACTIONS_TYPE_TOKEN_TRANSFER_DETAIL)
                    .arg("label", UiArg::Nested(Box::new(base)))
                    .arg("mint", UiArg::Text(mint.clone()))
                    .arg("amount", UiArg::Text(format!("{amount:.4}")))
            }
            Self::SpamAirdrop { mint, .. } => UiText::new(ids::TRANSACTIONS_TYPE_SPAM_DETAIL)
                .arg("mint", UiArg::Text(mint.clone())),
            Self::AtaClose { token_mint, .. } | Self::AtaCreate { token_mint, .. }
                if !token_mint.is_empty() =>
            {
                with_detail(base, token_mint)
            }
            Self::NftOperation { detail, .. } | Self::ProgramInteraction { detail, .. }
                if !detail.is_empty() =>
            {
                with_detail(base, detail)
            }
            Self::Other { description, .. } => UiText::new(ids::TRANSACTIONS_TYPE_DESCRIBED)
                .arg("description", UiArg::Text(description.clone())),
            _ => base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{format_en, LanguageIdentifier};

    /// One value of every variant, built through an exhaustive match so a new
    /// variant fails to compile until it is listed here and given a kind.
    fn every_variant() -> Vec<TransactionType> {
        let s = |value: &str| value.to_owned();
        let listed = |value: TransactionType| match value {
            TransactionType::Buy
            | TransactionType::Sell
            | TransactionType::Transfer
            | TransactionType::Compute
            | TransactionType::AtaOperation
            | TransactionType::Failed
            | TransactionType::Unknown
            | TransactionType::SwapSolToToken { .. }
            | TransactionType::SwapTokenToSol { .. }
            | TransactionType::SwapTokenToToken { .. }
            | TransactionType::SolTransfer { .. }
            | TransactionType::TokenTransfer { .. }
            | TransactionType::AtaClose { .. }
            | TransactionType::AtaCreate { .. }
            | TransactionType::Dust { .. }
            | TransactionType::SpamAirdrop { .. }
            | TransactionType::LiquidityAdd { .. }
            | TransactionType::LiquidityRemove { .. }
            | TransactionType::NftOperation { .. }
            | TransactionType::ProgramInteraction { .. }
            | TransactionType::Other { .. } => value,
        };
        vec![
            TransactionType::Buy,
            TransactionType::Sell,
            TransactionType::Transfer,
            TransactionType::Compute,
            TransactionType::AtaOperation,
            TransactionType::Failed,
            TransactionType::Unknown,
            TransactionType::SwapSolToToken {
                token_mint: s("mint"),
                sol_amount: 1.0,
                token_amount: 2.0,
                router: s("Jupiter"),
            },
            TransactionType::SwapTokenToSol {
                token_mint: s("mint"),
                token_amount: 2.0,
                sol_amount: 1.0,
                router: s("Jupiter"),
            },
            TransactionType::SwapTokenToToken {
                from_mint: s("a"),
                to_mint: s("b"),
                from_amount: 1.0,
                to_amount: 2.0,
                router: s("Jupiter"),
            },
            TransactionType::SolTransfer {
                amount: 1.0,
                from: s("a"),
                to: s("b"),
            },
            TransactionType::TokenTransfer {
                mint: s("mint"),
                amount: 1.5,
                from: s("a"),
                to: s("b"),
            },
            TransactionType::AtaClose {
                recovered_sol: 0.002,
                token_mint: s("mint"),
            },
            TransactionType::AtaCreate {
                rent_paid: 0.002,
                token_mint: s("mint"),
            },
            TransactionType::Dust {
                sol_amount: 0.000001,
                from: s("a"),
            },
            TransactionType::SpamAirdrop {
                mint: s("mint"),
                amount: 1.0,
                from: s("a"),
            },
            TransactionType::LiquidityAdd {
                pool: s("pool"),
                router: s("Raydium"),
            },
            TransactionType::LiquidityRemove {
                pool: s("pool"),
                router: s("Raydium"),
            },
            TransactionType::NftOperation {
                program: s("program"),
                detail: s("mint"),
            },
            TransactionType::ProgramInteraction {
                program: s("program"),
                detail: s("approve"),
            },
            TransactionType::Other {
                description: s("custom"),
                details: s("x"),
            },
        ]
        .into_iter()
        .map(listed)
        .collect()
    }

    #[test]
    fn every_kind_has_a_catalog_label_named_after_it() {
        for value in every_variant() {
            let kind = value.kind();
            let text = kind_text(kind);
            assert_eq!(
                text.id,
                format!("transactions-type-{}", kind.replace('_', "-"))
            );
            assert_ne!(format_en(&text.id, None), text.id, "missing {}", text.id);
        }
    }

    #[test]
    fn detail_wording_matches_the_activity_feed() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        let render = |value: TransactionType| value.ui_text().render_plain(&en);
        let variants = every_variant();
        let by_kind = |kind: &str| {
            variants
                .iter()
                .find(|value| value.kind() == kind)
                .unwrap()
                .clone()
        };
        assert_eq!(render(TransactionType::Buy), "Buy");
        assert_eq!(render(by_kind("swap")), "Swap (Jupiter)");
        assert_eq!(
            render(by_kind("token_transfer")),
            "Token transfer mint (1.5000)"
        );
        assert_eq!(render(by_kind("spam")), "Spam airdrop (mint)");
        assert_eq!(render(by_kind("ata_close")), "Rent reclaimed (mint)");
        assert_eq!(render(by_kind("nft")), "NFT (mint)");
        assert_eq!(render(by_kind("program")), "Program call (approve)");
        assert_eq!(render(by_kind("sol_transfer")), "SOL transfer");
    }
}
