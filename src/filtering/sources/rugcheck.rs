use std::cmp::Ordering;

use crate::config::schemas::RugCheckFilters;
use crate::filtering::sources::FilterRejectionReason;
use crate::tokens::types::{SecurityRisk, Token};

pub fn evaluate(token: &Token, config: &RugCheckFilters) -> Result<(), FilterRejectionReason> {
    if !config.enabled {
        return Ok(());
    }

    if config.block_rugged_tokens && token.is_rugged {
        return Err(FilterRejectionReason::RugcheckRuggedToken);
    }

    if config.risk_score_enabled {
        if let Some(score) = token.security_score {
            if score > config.max_risk_score {
                return Err(FilterRejectionReason::RugcheckRiskScoreTooHigh);
            }
        }
    }

    if config.risk_level_enabled && config.block_danger_level {
        if token
            .security_risks
            .iter()
            .any(|risk| risk.level.eq_ignore_ascii_case("danger"))
        {
            return Err(FilterRejectionReason::RugcheckRiskLevelDanger);
        }
    }

    if config.authority_checks_enabled && config.require_authorities_safe {
        if !config.allow_mint_authority && token.mint_authority.is_some() {
            return Err(FilterRejectionReason::RugcheckMintAuthorityBlocked);
        }

        if !config.allow_freeze_authority && token.freeze_authority.is_some() {
            return Err(FilterRejectionReason::RugcheckFreezeAuthorityBlocked);
        }
    }

    if let Some(reason) = check_holder_distribution(token, config) {
        return Err(reason);
    }

    if config.insider_holder_checks_enabled {
        let insider_count = token
            .top_holders
            .iter()
            .take(10)
            .filter(|holder| holder.insider)
            .count() as u32;
        if insider_count > config.max_insider_holders_in_top_10 {
            return Err(FilterRejectionReason::RugcheckInsiderHolderCount);
        }

        let insider_total_pct: f64 = token
            .top_holders
            .iter()
            .filter(|holder| holder.insider)
            .map(|holder| holder.pct)
            .sum();
        if insider_total_pct > config.max_insider_total_pct {
            return Err(FilterRejectionReason::RugcheckInsiderTotalPct);
        }
    }

    if config.max_graph_insiders > 0 {
        if let Some(count) = token.graph_insiders_detected {
            if count as i32 > config.max_graph_insiders {
                return Err(FilterRejectionReason::RugcheckGraphInsidersTooHigh);
            }
        }
    }

    if config.max_creator_balance_pct > 0.0 {
        if let Some(creator_pct) = token.creator_balance_pct {
            if creator_pct > config.max_creator_balance_pct {
                return Err(FilterRejectionReason::RugcheckCreatorBalanceTooHigh);
            }
        }
    }

    if config.transfer_fee_enabled {
        match token.transfer_fee_pct {
            Some(fee_pct) => {
                if config.block_transfer_fee_tokens && fee_pct > 0.0 {
                    return Err(FilterRejectionReason::RugcheckTransferFeePresent);
                }

                if fee_pct > config.max_transfer_fee_pct {
                    return Err(FilterRejectionReason::RugcheckTransferFeeTooHigh);
                }
            }
            None => {
                if config.block_transfer_fee_tokens {
                    return Err(FilterRejectionReason::RugcheckTransferFeeMissing);
                }
            }
        }
    }

    if config.min_lp_providers > 0 {
        match token.lp_provider_count {
            Some(count) => {
                if count < config.min_lp_providers as i64 {
                    return Err(FilterRejectionReason::RugcheckLpProvidersTooLow);
                }
            }
            None => {
                return Err(FilterRejectionReason::RugcheckLpProvidersMissing);
            }
        }
    }

    if let Some(reason) = check_lp_lock(token, config) {
        return Err(reason);
    }

    Ok(())
}

fn check_holder_distribution(
    token: &Token,
    config: &RugCheckFilters,
) -> Option<FilterRejectionReason> {
    if !config.holder_distribution_enabled {
        return None;
    }

    if let Some(total) = token.total_holders {
        if total < config.min_unique_holders as i64 {
            return Some(FilterRejectionReason::RugcheckNotEnoughHolders);
        }
    }

    let mut holders = token.top_holders.clone();
    holders.sort_by(|a, b| match b.pct.partial_cmp(&a.pct) {
        Some(Ordering::Greater) | Some(Ordering::Equal) => Ordering::Greater,
        Some(Ordering::Less) => Ordering::Less,
        None => Ordering::Equal,
    });

    if let Some(first) = holders.first() {
        if first.pct > config.max_top_holder_pct {
            return Some(FilterRejectionReason::RugcheckTopHolderTooHigh);
        }
    }

    let top_three_sum: f64 = holders.iter().take(3).map(|holder| holder.pct).sum();
    if top_three_sum > config.max_top_3_holders_pct {
        return Some(FilterRejectionReason::RugcheckTop3HoldersTooHigh);
    }

    None
}

fn check_lp_lock(token: &Token, config: &RugCheckFilters) -> Option<FilterRejectionReason> {
    if !config.lp_lock_enabled {
        return None;
    }

    let expect_lock_data = is_pumpfun_token(token);

    match extract_lp_lock_percentage(token) {
        Some(lock_pct) => {
            let required = if expect_lock_data {
                config.min_pumpfun_lp_lock_pct
            } else {
                config.min_regular_lp_lock_pct
            };

            if lock_pct < required {
                Some(FilterRejectionReason::RugcheckLpLockTooLow)
            } else {
                None
            }
        }
        None => {
            if expect_lock_data {
                Some(FilterRejectionReason::RugcheckLpLockMissing)
            } else {
                None
            }
        }
    }
}

fn is_pumpfun_token(token: &Token) -> bool {
    token
        .token_type
        .as_ref()
        .map(|value| value.to_ascii_lowercase().contains("pump"))
        .unwrap_or(false)
}

fn extract_lp_lock_percentage(token: &Token) -> Option<f64> {
    token
        .security_risks
        .iter()
        .find_map(|risk| extract_percentage_from_risk(risk))
}

/// Inspect Rugcheck risk metadata and best-effort parse an LP lock percentage.
fn extract_percentage_from_risk(risk: &SecurityRisk) -> Option<f64> {
    let name = risk.name.to_ascii_lowercase();
    let description = risk.description.to_ascii_lowercase();

    if !(name.contains("lp") && name.contains("lock"))
        && !(description.contains("liquidity") && description.contains("lock"))
    {
        return None;
    }

    extract_percentage_from_text(&risk.value)
        .or_else(|| extract_percentage_from_text(&risk.description))
        .or_else(|| infer_percentage_from_keywords(&risk.value))
        .or_else(|| infer_percentage_from_keywords(&risk.description))
}

fn extract_percentage_from_text(text: &str) -> Option<f64> {
    let cleaned = text.replace('%', " ");
    for part in cleaned.split_whitespace() {
        if let Ok(value) = part.parse::<f64>() {
            return Some(value);
        }
    }
    None
}

fn infer_percentage_from_keywords(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();

    if lower.contains("unlock") {
        return Some(0.0);
    }

    if lower.contains("locked") && !lower.contains("unlock") && !lower.contains("partial") {
        return Some(100.0);
    }

    None
}
