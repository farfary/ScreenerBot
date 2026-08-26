//! Why a swap could not be quoted.
//!
//! Quoting is the one place where an external router's opinion becomes an
//! internal decision: whether to blacklist a token, whether to back off, and
//! what the trade dialog tells the user. Those decisions used to be taken by
//! searching the error's message for provider prose (`"no route"`,
//! `"Jupiter API error: 400"`), which meant a router rewording a response —
//! or our own code producing a friendlier message — silently changed trading
//! behaviour with nothing failing to say so.
//!
//! [`QuoteError`] is that vocabulary. A router classifies its own response
//! once, at the point where it still holds the HTTP status and the decoded
//! body, and every consumer downstream matches the variant.

use std::time::Duration;

use crate::chains::ChainId;
use crate::errors::{ErrorClass, NetworkError, ServiceError, Severity};
use crate::Error;

/// A quote could not be produced. Constructed by the router that failed, or by
/// the aggregate quote path when no router produced anything usable.
#[derive(Debug, Clone, thiserror::Error)]
pub enum QuoteError {
    /// The router registry has not been built, so nothing could be asked. This
    /// is a startup fault, not a verdict on the trade, and it carries the
    /// original `ServiceError` so callers on the crate channel still see the
    /// service that failed rather than a generic quote failure.
    #[error(transparent)]
    RegistryUnavailable(#[from] ServiceError),

    /// No router is enabled for the requested chain, so nothing was even asked.
    #[error("no swap routers are enabled for {chain}")]
    NoRoutersEnabled { chain: ChainId },

    /// The provider knows the token and says it cannot be traded at all — no
    /// pool, no liquidity, unlaunched or abandoned. A durable property of the
    /// token rather than of this request.
    #[error("{router} reports the token is not tradable: {detail}")]
    NotTradable { router: String, detail: String },

    /// The provider could route neither this pair nor this size. Unlike
    /// [`QuoteError::NotTradable`] this can be specific to the requested
    /// amount and can clear on its own, so it must not be treated as a
    /// permanent verdict on the token from a single occurrence.
    #[error("{router} found no route: {detail}")]
    NoRoute { router: String, detail: String },

    /// The provider refused us for volume.
    #[error("{router} rate limited the quote request")]
    RateLimited {
        router: String,
        retry_after: Option<Duration>,
    },

    /// The provider did not answer within its deadline.
    #[error("{router} did not answer the quote request in time")]
    Timeout { router: String },

    /// A router answered, but with something we must not trade on: a
    /// zero-output quote, or a quote for a pair we never asked for. This is
    /// our own refusal of untrusted input, not a provider failure.
    #[error("{router} returned an unusable quote: {detail}")]
    RouterRejected { router: String, detail: String },

    /// The provider failed for a reason that is none of the above — a 5xx, a
    /// transport error, an undecodable body. Carries the detail for logs and
    /// for the dialog's fallback message, and nothing reads that text to make
    /// a decision.
    #[error("{router} could not provide a quote: {detail}")]
    Unavailable { router: String, detail: String },
}

impl QuoteError {
    /// Stable machine code for the dashboard's error envelope. The trade
    /// dialog switches on this; it must not change casually.
    pub fn code(&self) -> &'static str {
        match self {
            QuoteError::RegistryUnavailable(_) => "SwapsUnavailable",
            QuoteError::NoRoutersEnabled { .. } => "NoRouters",
            QuoteError::NotTradable { .. } => "TokenNotTradable",
            QuoteError::NoRoute { .. } => "NoRoute",
            QuoteError::RateLimited { .. } => "QuoteRateLimited",
            QuoteError::Timeout { .. } => "QuoteTimeout",
            QuoteError::RouterRejected { .. } => "QuoteRejected",
            QuoteError::Unavailable { .. } => "QuoteFailed",
        }
    }

    /// Short headline for the trade dialog.
    pub fn title(&self) -> &'static str {
        match self {
            QuoteError::RegistryUnavailable(_) => "Swap routing is not ready yet",
            QuoteError::NoRoutersEnabled { .. } => "No swap providers are enabled",
            QuoteError::NotTradable { .. } => "This token isn't tradable right now",
            QuoteError::NoRoute { .. } => "No swap route available",
            QuoteError::RateLimited { .. } => "Swap providers are rate limiting us",
            QuoteError::Timeout { .. } => "Quote request timed out",
            QuoteError::RouterRejected { .. } => "The quote was refused",
            QuoteError::Unavailable { .. } => "Couldn't fetch a quote",
        }
    }

    /// What the user can actually do about it.
    pub fn hint(&self) -> &'static str {
        match self {
            QuoteError::RegistryUnavailable(_) => {
                "The swap service is still starting. Wait for services to become ready, then retry."
            }
            QuoteError::NoRoutersEnabled { .. } => {
                "Enable at least one swap router in Trader settings, then try again."
            }
            QuoteError::NotTradable { .. } => {
                "No liquidity or swap route is available. The token may be unlaunched, \
                 abandoned, or have no pool. Try again later or choose another token."
            }
            QuoteError::NoRoute { .. } => {
                "No provider could route this trade at the requested amount. Try a smaller \
                 amount, or try again in a moment."
            }
            QuoteError::RateLimited { .. } => {
                "The swap providers are throttling requests. Wait a few seconds and retry."
            }
            QuoteError::Timeout { .. } => {
                "The swap providers didn't respond in time. Check your connection and retry."
            }
            QuoteError::RouterRejected { .. } => {
                "A provider returned a quote that failed our safety checks and was discarded. \
                 Retry to fetch a fresh one."
            }
            QuoteError::Unavailable { .. } => {
                "The swap providers couldn't quote this trade. Try again in a moment."
            }
        }
    }

    /// The blacklist reason to record when this failure is a durable verdict on
    /// the token, or `None` when the token must keep its chance.
    ///
    /// Only [`QuoteError::NotTradable`] is durable on its own: the provider is
    /// telling us the token has no market at all. A [`QuoteError::NoRoute`] is
    /// deliberately excluded here — it is amount-dependent and self-clearing,
    /// and the blacklist is permanent and only removable by hand, so a single
    /// transient no-route must never retire a token for good. The opening path
    /// requires it to repeat before it counts.
    pub fn permanent_token_verdict(&self) -> Option<&'static str> {
        match self {
            QuoteError::NotTradable { .. } => Some("NotTradable"),
            _ => None,
        }
    }

    /// True when the token failed to route and the opening path should count it
    /// towards its repeat threshold.
    pub fn is_route_failure(&self) -> bool {
        matches!(
            self,
            QuoteError::NotTradable { .. } | QuoteError::NoRoute { .. }
        )
    }
}

impl ErrorClass for QuoteError {
    fn is_retryable(&self) -> bool {
        match self {
            // Throttling, deadlines and one-off provider faults clear on their
            // own. A missing market, a disabled registry and a refused quote do
            // not change by asking the same question again.
            QuoteError::RateLimited { .. }
            | QuoteError::Timeout { .. }
            | QuoteError::Unavailable { .. } => true,
            QuoteError::RegistryUnavailable(_)
            | QuoteError::NoRoutersEnabled { .. }
            | QuoteError::NotTradable { .. }
            | QuoteError::NoRoute { .. }
            | QuoteError::RouterRejected { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            QuoteError::RateLimited { retry_after, .. } => {
                Some(retry_after.unwrap_or(Duration::from_secs(2)))
            }
            QuoteError::Timeout { .. } | QuoteError::Unavailable { .. } => {
                Some(Duration::from_millis(500))
            }
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            // A token with no market is the ordinary case on a discovery feed.
            QuoteError::NotTradable { .. } | QuoteError::NoRoute { .. } => Severity::Info,
            // Ours to fix, and it stops trading entirely until it is.
            QuoteError::RegistryUnavailable(_) | QuoteError::NoRoutersEnabled { .. } => {
                Severity::Error
            }
            // A router handing us a quote we had to refuse is a money-path
            // anomaly, not routine noise.
            QuoteError::RouterRejected { .. } => Severity::Critical,
            _ => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            QuoteError::RegistryUnavailable(_) | QuoteError::NoRoutersEnabled { .. } => 503,
            QuoteError::NotTradable { .. } | QuoteError::NoRoute { .. } => 422,
            QuoteError::RateLimited { .. } => 429,
            QuoteError::Timeout { .. } => 504,
            QuoteError::RouterRejected { .. } | QuoteError::Unavailable { .. } => 502,
        }
    }
}

impl From<QuoteError> for Error {
    /// Fold a quote failure into the crate error channel for callers that only
    /// need "it failed". The classification is preserved in the mapped variant
    /// so `ErrorClass` still answers correctly after the conversion — callers
    /// that need the variant itself take `try_get_best_quote` instead.
    fn from(err: QuoteError) -> Self {
        match &err {
            QuoteError::RegistryUnavailable(e) => Error::Service(e.clone()),
            QuoteError::NoRoutersEnabled { .. } => Error::configuration_error(err.to_string()),
            QuoteError::RateLimited {
                router,
                retry_after,
            } => Error::Network(NetworkError::RateLimited {
                endpoint: router.clone(),
                retry_after_ms: retry_after.map(|d| d.as_millis() as u64),
            }),
            _ => Error::api_error(err.to_string()),
        }
    }
}

/// Result of a quote attempt.
pub type QuoteResult<T> = std::result::Result<T, QuoteError>;
