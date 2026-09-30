# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = { -brand } data is active
    .detail = Shared candles, pool registry, security reports and token identity are being served from screenerbot.io.

account-data-access-disabled = { -brand } data is switched off
    .detail = The { -brand } source is turned off in your settings, so data comes from the public providers only.

account-data-access-offline = { -brand } data is offline
    .detail = There is no network connection. Data will resume on its own once the connection returns.

account-data-access-signed-out = { -brand } data needs an account
    .detail = Charts, pools, security reports and token identity come from the public providers instead. They are slower, rate limited, and thinner on history. Signing in is free and changes nothing else about how { -brand } runs.

account-data-access-reauthorization-required = { -brand } data needs you to sign in again
    .detail = This device was authorised before { -brand } data existed. Sign in again to restore it — the public providers are being used until then.

account-data-access-version-unsupported = { -brand } data needs a newer version
    .detail = This version is no longer served. Update to { $minimum } or newer to use { -brand } data again; the public providers are being used until then.

account-data-access-unreachable = { -brand } data is not responding
    .detail = The service did not answer. The public providers are being used, and { -brand } will keep retrying.

account-data-access-unknown = { -brand } data has not been checked yet
    .detail = { -brand } has not yet needed shared data this session.

## Account panel (ui/account/panel.js), shared by Setup and Settings

# Features a signed-in account carries.
account-scope-data-read = { -brand } market data
account-scope-rpc-submit = Free signed-transaction submission
account-scope-vote = Token voting
account-scope-referral-read = Referral earnings
account-scope-account-read = Account details

account-panel-request-failed = That did not work. Please try again.
account-panel-checking = Checking account status…
account-panel-status-unavailable = Account status is unavailable.
account-panel-browser-notice = Finish signing in in your browser, then return here. This panel will update.
account-panel-browser-timeout = Browser sign-in was not completed. You can start it again.
account-panel-unavailable = Account features are unavailable right now. Continue setup without signing in.
account-panel-retry-status = Retry account status
account-panel-signed-in-fallback = Signed in
account-panel-features =
    .aria-label = Account features
account-panel-sign-out = Sign out
account-panel-signing-out = Signing out…
account-panel-sign-in = Sign in
account-panel-signing-in = Signing in…
account-panel-sign-in-wallet = Sign in with wallet
account-panel-opening-browser = Opening browser…
account-panel-continue-browser = Continue in browser
account-panel-sign-in-email = Sign in with email
account-panel-new-to = New to { -brand }?
account-panel-create-account = Create an account
account-panel-unlocks-title = Included with an account
account-panel-back-to-options = Back to sign-in options
account-panel-email-label = Email
account-panel-email-input =
    .placeholder = you@example.com
account-panel-password-label = Password
account-panel-password-input =
    .placeholder = Your password
account-panel-need-account = Need an account or forgot your password?
account-panel-open-website = Open screenerbot.io
