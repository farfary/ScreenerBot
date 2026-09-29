# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = ScreenerBot data is active
    .detail = Shared candles, pool registry, security reports and token identity are being served from screenerbot.io.

account-data-access-disabled = ScreenerBot data is switched off
    .detail = The ScreenerBot source is turned off in your settings, so data comes from the public providers only.

account-data-access-offline = ScreenerBot data is offline
    .detail = There is no network connection. Data will resume on its own once the connection returns.

account-data-access-signed-out = ScreenerBot data needs an account
    .detail = Charts, pools, security reports and token identity come from the public providers instead. They are slower, rate limited, and thinner on history. Signing in is free and changes nothing else about how ScreenerBot runs.

account-data-access-reauthorization-required = ScreenerBot data needs you to sign in again
    .detail = This device was authorised before ScreenerBot data existed. Sign in again to restore it — the public providers are being used until then.

account-data-access-version-unsupported = ScreenerBot data needs a newer version
    .detail = This version is no longer served. Update to { $minimum } or newer to use ScreenerBot data again; the public providers are being used until then.

account-data-access-unreachable = ScreenerBot data is not responding
    .detail = The service did not answer. The public providers are being used, and ScreenerBot will keep retrying.

account-data-access-unknown = ScreenerBot data has not been checked yet
    .detail = ScreenerBot has not yet needed shared data this session.
