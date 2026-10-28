# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

# Source: scripts/core/setup_runtime.js

## Wallet key validation

setup-wallet-required = Enter a wallet private key.
setup-wallet-json-recognized = 64-byte JSON key format recognized.
setup-wallet-json-invalid = Use a JSON array containing exactly 64 byte values (0–255).
setup-wallet-format-invalid = Use a base58 private key or a 64-byte JSON array.
setup-wallet-base58-recognized = Base58 key format recognized.

## RPC endpoint validation

setup-rpc-required = Enter at least one RPC endpoint.
setup-dialog-rpc-optional-note = Optional: leave the field empty to keep the RPC endpoints already configured.
setup-rpc-too-many = Use no more than 10 RPC endpoints.
setup-rpc-url-invalid = Every endpoint must be a valid HTTPS URL.
setup-rpc-url-credentials = RPC URLs cannot include usernames or passwords.
setup-rpc-url-fragment = RPC URLs cannot include fragments.
setup-rpc-public-endpoint = The public Solana RPC cannot support continuous polling.
setup-rpc-private-host = RPC endpoints cannot use local or private network hosts.
setup-rpc-duplicate = Remove duplicate RPC endpoints.
setup-rpc-ready =
    { $count ->
        [one] { $count } HTTPS endpoint ready to test.
       *[other] { $count } HTTPS endpoints ready to test.
    }

## Verification results

setup-wallet-verified = Wallet verified
setup-wallet-unverified = Wallet could not be verified
setup-wallet-address-detail = Address { $address }
setup-wallet-format-hint = Check the private key format.
setup-rpc-none-working = No working mainnet RPC
setup-rpc-health-failed = No endpoint passed the mainnet health checks.
setup-rpc-partial = { $working } working; { $failed } unavailable
setup-rpc-verified =
    { $count ->
        [one] { $count } mainnet endpoint verified
       *[other] { $count } mainnet endpoints verified
    }
setup-rpc-fastest = Fastest: { $url } ({ $latency } ms).
setup-error-request-failed = Request failed ({ $status })
setup-error-restart-timeout = { -brand } has not reconnected yet.

# Source: scripts/core/setup.js

## Verification steps

setup-verify-wallet-parsing = Parsing private key
setup-verify-wallet-parsing-detail = Checking the key and deriving its public address.
setup-verify-wallet-waiting = Waiting to validate
setup-verify-rpc-testing = Testing Solana mainnet
setup-verify-rpc-testing-detail =
    { $count ->
        [one] Checking { $count } endpoint.
       *[other] Checking { $count } endpoints.
    }
setup-verify-rpc-waiting = Waiting to test endpoints
setup-verify-save-waiting = Waiting to save
setup-verify-save-running = Encrypting and saving
setup-verify-save-running-detail = Writing the verified configuration on this device.
setup-verify-save-done = Configuration saved
setup-verify-save-done-detail = Private key encrypted; working RPC endpoints stored.
setup-verify-save-failed = Could not save setup
setup-verify-save-skipped = Not saved. Fix the issue above, then verify again.
setup-verify-request-failed = Verification request failed
setup-verify-summary-checking = Checking your wallet and Solana mainnet connections.
setup-verify-summary-running = Verifying the exact credentials you entered.
setup-verify-summary-saving = Credentials verified. Saving securely.
setup-verify-summary-failed = Review the issue, then verify again.

## Errors

setup-error-credentials-failed = Credential verification failed.
setup-error-save-failed = Setup could not be saved.
setup-error-verify-failed = Verification failed.
setup-error-explore-failed = Explore Mode could not be started.
setup-error-gateway-failed = The sending setting could not be saved.
setup-action-review-credentials = Review credentials

## Completion

setup-explore-opening = Opening Explore Mode…
setup-complete-restarting = Restarting { -brand } with your verified configuration.
setup-complete-finishing = Finishing restart…
setup-complete-ready = { -brand } is ready. Opening dashboard…
setup-complete-stored = Your verified configuration is safely stored on this device.

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = Show private key
setup-wallet-hide-key = Hide private key
setup-wallet-copy =
    .aria-label = Copy wallet address
    .title = Copy wallet address
setup-wallet-copy-done =
    .aria-label = Wallet address copied
    .title = Copied
setup-wallet-copy-failed =
    .aria-label = Could not copy wallet address
    .title = Copy failed

# Source: scripts/ui/setup_dialog.js

## Setup dialog

setup-dialog-close =
    .title = Close
    .aria-label = Close
setup-dialog-wallet-label = Wallet Private Key
setup-dialog-wallet-input =
    .placeholder = Base58 string or JSON array [1,2,3,...]
setup-dialog-working = Working…
setup-dialog-validating = Validating…
setup-dialog-saving = Saving…
setup-dialog-restarting = Restarting…
setup-dialog-error-validation = Validation failed.
setup-dialog-error-incomplete = Setup could not be completed.
setup-dialog-error-restart-helper = Automatic restart helper is unavailable. Reload the dashboard shortly.
setup-dialog-error-unexpected = Unexpected error.
setup-dialog-replace-title = Replace trading wallet
setup-dialog-replace-subtitle = Trade with another wallet. Its private key is encrypted on this device and never leaves it.
setup-dialog-replace-submit = Validate & replace
setup-dialog-replace-saved = Replacement staged — restarting { -brand } with the new wallet…
setup-dialog-replace-confirm-intro = Replacing the trading wallet:
setup-dialog-replace-confirm-stop = Stops the trader and copy trading. Turn them on again after the restart.
setup-dialog-replace-confirm-backup = Moves the current wallet's positions, transactions and history into a backup folder.
setup-dialog-replace-confirm-keep = Keeps the current wallet's key under Other Wallets, where you can still export it.
setup-dialog-replace-confirm-funds = Moves no funds: tokens and { -sol } stay in the current wallet.
setup-dialog-replace-confirm-rpc = Replaces the configured RPC endpoints with the ones you enter; with the field empty, the current endpoints stay.
setup-dialog-replace-confirm-restart = Restarts { -brand } with the new wallet.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = Setup progress
setup-wizard-wallet-security-note = Encrypted before it is saved.
setup-wizard-rpc-title = RPC Endpoints
setup-wizard-rpc-input =
    .placeholder = One HTTPS URL per line
setup-wizard-rpc-guidance = Use a reliable mainnet provider; public endpoints are not supported.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = recommended
setup-wizard-account-title = { -brand } account
setup-wizard-account-optional = Optional
setup-wizard-account-loading = Checking account status…
setup-wizard-verify-title = Verify and save
setup-wizard-verify-list =
    .aria-label = Setup verification status
setup-wizard-verify-wallet = Wallet
setup-wizard-verify-save = Secure configuration
setup-wizard-complete-title = Setup saved
setup-wizard-reconnect = Retry connection
setup-wizard-reload = Reload dashboard
setup-wizard-error-title = Setup needs attention
setup-wizard-explore = Explore dashboard
setup-wizard-continue = Continue

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Wallet
setup-wizard-step-connection = Connection
setup-wizard-step-verify = Verify
setup-wizard-step-ready = Ready
setup-wallet-title = Set up your trading wallet
setup-wallet-description = The wallet { -brand } trades with. Its key is encrypted on this computer and never leaves it.
setup-wallet-source =
    .aria-label = Wallet source
setup-wallet-create-title = Create a new wallet
setup-wallet-create-detail = { -brand } makes a new wallet. You save its key, then send { -sol } to it.
setup-wallet-import-title = Import a wallet
setup-wallet-import-detail = Use a private key you already have (base58 or a 64-number JSON array).
setup-wallet-creating = Creating your wallet…
setup-wallet-create-failed = The wallet could not be created. Try again.
setup-backup-title = Save your private key
setup-backup-description = Anyone with this key controls the wallet. { -brand } cannot recover it.
setup-backup-address = Address
setup-backup-key = Private key
setup-backup-copy-address = Copy address
setup-backup-download = Download
setup-backup-copied = Copied. Clear your clipboard after saving it.
setup-backup-machine-warning = The encrypted copy { -brand } keeps opens only on this computer. If the computer is lost or replaced, only the key you save now restores the wallet. Keep it offline, for example on paper or in a password manager.
setup-backup-ack-saved = I saved the private key somewhere safe, outside this computer.
setup-backup-ack-control = I understand that anyone with this key can take the funds in this wallet.
setup-backup-ack-recovery = I understand { -brand } cannot recover this key. Without it, a lost computer means lost funds.
setup-backup-create-another = Create another
setup-backup-replace-confirm = Discard this key and create a new wallet? Only do this if you have not sent anything to it.
setup-connection-title = How should { -brand } connect to { -solana }?
setup-connection-description = You can change this later in Settings → Connection.
setup-connection-choice =
    .aria-label = Connection
setup-connection-service-title = { -brand } service
setup-connection-service-badge = No RPC needed
setup-connection-service-detail = Uses your free { -brand } account.
setup-connection-service-prices = Prices refresh every few seconds
setup-connection-service-features = Copy trading, wallet watching, other wallets' balances, older history, direct pool swaps and live pool prices need your own RPC
setup-connection-service-pause = Trading pauses if you sign out or screenerbot.io can't be reached
setup-connection-service-privacy = Your private key never leaves this computer: swaps are signed here, and screenerbot.io only uses your wallet's public address to read its balances and transactions
setup-connection-own-title = Your own RPC
setup-connection-own-badge = Fastest
setup-connection-own-detail = Prices about twice a second and every feature. Needs an RPC endpoint from a provider.
setup-connection-signin-required = Sign in or create a free account to use the { -brand } service.
setup-connection-reauthorize = Sign in again to allow wallet reads for the { -brand } service.
setup-connection-sending = Send transactions through { -brand }
setup-connection-sending-hint = Available when signed in. Your RPC is used when that is unavailable.
setup-wizard-account-required = Required
setup-wizard-verify-connection = Connection
setup-verify-service-checking = Checking the { -brand } service
setup-verify-service-ready = { -brand } service ready
setup-verify-service-busy = The { -brand } service is busy right now. Try again in a moment, or use your own RPC.
setup-verify-service-daily-limit = Your account reached today's limit for the { -brand } service. Try again after 00:00 UTC, or use your own RPC.
setup-verify-service-unreachable = screenerbot.io can't be reached. Check your internet connection.
setup-verify-service-version = This version is no longer supported by the { -brand } service. Update { -brand } to continue.
setup-verify-service-signin = Sign in to use the { -brand } service.
setup-verify-save-done-service-detail = Private key encrypted; { -brand } service selected.
setup-action-try-again = Try again
setup-action-use-own-rpc = Use my own RPC
setup-action-sign-in-again = Sign in again
setup-ready-title = { -brand } is ready
setup-fund-title = Fund your trading wallet
setup-fund-waiting = Waiting for { -sol }…
setup-fund-received = Received { $amount } { -sol }
setup-fund-balance-unavailable = Reading the balance… It updates on its own.
setup-fund-open-dashboard = Open dashboard
