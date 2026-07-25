//! Batch token identity lookup — the ONE cheap way the dashboard resolves a mint
//! to its symbol, name, logo and decimals.
//!
//! Deliberately separate from `get_token_detail`: that route falls back to an
//! EXTERNAL DexScreener/Jupiter fetch when a mint is unknown, which is far too
//! expensive for identity resolution (a single transaction can reference several
//! mints we never traded). This route is cache-first, DB-second and NEVER touches
//! the network — an unknown mint simply comes back missing from the map, and the
//! caller renders the mint itself.

use axum::extract::Query;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cap on mints per request — a dialog resolves a handful, never a page of them.
const MAX_IDENTITIES: usize = 50;

#[derive(Debug, Deserialize)]
pub struct IdentitiesQuery {
    /// Comma-separated mint list.
    pub mints: String,
}

#[derive(Debug, Serialize)]
pub struct TokenIdentity {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub decimals: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct IdentitiesResponse {
    /// Keyed by mint; a mint we know nothing about is simply absent.
    pub identities: HashMap<String, TokenIdentity>,
}

pub async fn get_token_identities(
    Query(query): Query<IdentitiesQuery>,
) -> Json<IdentitiesResponse> {
    let mut identities = HashMap::new();
    let mut seen = Vec::new();

    for mint in query
        .mints
        .split(',')
        .map(str::trim)
        .filter(|mint| !mint.is_empty())
    {
        if seen.iter().any(|existing| existing == mint) {
            continue;
        }
        seen.push(mint.to_string());
        if seen.len() > MAX_IDENTITIES {
            break;
        }

        if let Ok(Some(token)) = crate::tokens::get_full_token_async(mint).await {
            identities.insert(
                mint.to_string(),
                TokenIdentity {
                    mint: token.mint,
                    symbol: non_empty(token.symbol),
                    name: non_empty(token.name),
                    logo_url: token.image_url.and_then(non_empty),
                    decimals: token.decimals,
                },
            );
        }
    }

    Json(IdentitiesResponse { identities })
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
