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
setup-error-restart-timeout = Setup is saved, but { -brand } has not reconnected yet.

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
setup-verify-save-skipped = Not saved
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
setup-error-gateway-failed = Gateway preference could not be saved.
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

setup-dialog-title = Set up wallet & RPC
setup-dialog-subtitle = Connect your Solana wallet and a premium RPC endpoint to enable trading and live on-chain data. Your private key is encrypted on this device and never leaves it.
setup-dialog-close =
    .title = Close
    .aria-label = Close
setup-dialog-wallet-label = Wallet Private Key
setup-dialog-wallet-input =
    .placeholder = Base58 string or JSON array [1,2,3,...]
setup-dialog-rpc-label = RPC endpoint(s)
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint... (one per line)
setup-dialog-rpc-hint = A premium provider ({ -helius }, { -quicknode }, { -alchemy }) is strongly recommended — the public Solana RPC is rate-limited and may not work.
setup-dialog-submit = Validate & connect
setup-dialog-working = Working…
setup-dialog-validating = Validating…
setup-dialog-saving = Saving…
setup-dialog-restarting = Restarting…
setup-dialog-saved = Setup saved — restarting { -brand } in full mode…
setup-dialog-error-missing-fields = Enter both a wallet private key and at least one RPC URL.
setup-dialog-error-validation = Validation failed.
setup-dialog-error-incomplete = Setup could not be completed.
setup-dialog-error-restart-helper = Automatic restart helper is unavailable. Reload the dashboard shortly.
setup-dialog-error-unexpected = Unexpected error.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = Setup progress
setup-wizard-step-credentials = Credentials
setup-wizard-step-verification = Verification
setup-wizard-step-complete = Complete
setup-wizard-credentials-title = Configure credentials
setup-wizard-credentials-description = Connect a local wallet and reliable Solana mainnet RPC endpoints.
setup-wizard-wallet-toggle =
    .title = Show private key
    .aria-label = Show private key
setup-wizard-wallet-security-note = Encrypted before it is saved.
setup-wizard-rpc-title = RPC Endpoints
setup-wizard-rpc-input =
    .placeholder = One HTTPS URL per line
setup-wizard-rpc-guidance = Reliable mainnet RPC recommended for continuous polling.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = recommended
setup-wizard-gateway-title = Free transaction sending
setup-wizard-gateway-hint = Available when signed in. Your RPC remains available as fallback.
setup-wizard-account-title = { -brand } account
setup-wizard-account-optional = Optional
setup-wizard-account-loading = Checking account status…
setup-wizard-verify-title = Verify and save
setup-wizard-verify-list =
    .aria-label = Setup verification status
setup-wizard-verify-wallet = Wallet
setup-wizard-verify-rpc = Solana RPC
setup-wizard-verify-save = Secure configuration
setup-wizard-complete-title = Setup saved
setup-wizard-reconnect = Retry connection
setup-wizard-reload = Reload dashboard
setup-wizard-error-title = Setup needs attention
setup-wizard-explore = Explore dashboard
setup-wizard-continue = Continue
