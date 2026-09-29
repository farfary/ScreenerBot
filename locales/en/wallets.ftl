# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = Generated
wallets-type-imported = Imported
wallets-type-migrated = Migrated

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = Paused by you
wallets-watch-disabled-signature-budget = Paused: reached the { $limit }-signature check limit before catching up
wallets-watch-disabled-unknown = Paused: the saved watch safety reason could not be read
wallets-watch-disabled-helius-unavailable = Paused: high-activity provider is unavailable; cursor preserved
wallets-watch-disabled-processing-failed = Paused: wallet activity could not be processed; cursor preserved

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = High-activity provider is unavailable; watch paused
wallets-watch-error-provider-repeated-failure = Helius checks repeatedly failed; watch paused
wallets-watch-error-processing-repeated-failure = Wallet activity processing repeatedly failed; watch paused
wallets-watch-error-position-unreadable = Wallet watch could not read its saved position; retrying
wallets-watch-error-provider-check-failed = High-activity provider check failed; retrying
wallets-watch-error-decode-failed = High-activity transaction could not be decoded; cursor retained
wallets-watch-error-processing-failed = Wallet activity could not be processed; retrying
wallets-watch-error-position-save-failed = Wallet watch could not save its position; retrying
