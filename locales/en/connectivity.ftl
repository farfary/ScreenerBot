# Connectivity endpoint health text. Probe ids come from ProbeFailure in
# src/connectivity/types.rs; the technical values are not translated.
connectivity-probe-client-setup-failed = Failed to create client: { $detail }
# $status is an HTTP status line such as 503 Service Unavailable.
connectivity-probe-http-status = HTTP { $status }
connectivity-probe-timeout = Timeout after { $seconds }s
connectivity-probe-request-failed = Request failed: { $detail }
connectivity-probe-rpc-none = No RPC providers configured
# $unhealthy lists the unhealthy providers with their circuit state.
connectivity-probe-rpc-partial = { $healthy }/{ $total } RPC providers healthy. Unhealthy: { $unhealthy }
connectivity-probe-rpc-all-unreachable = All { $total } RPC providers unreachable: { $unhealthy }
connectivity-probe-internet-dns-failed = DNS check failed but HTTP works: { $detail }
connectivity-probe-internet-all-failed = DNS and HTTP checks failed. DNS: { $dns }. HTTP: { $http }
connectivity-probe-unknown = Unknown error

# Endpoint that has not been probed yet; used by src/webserver/routes/connectivity/types.rs.
connectivity-endpoint-not-checked = Not checked yet

# Service chain access state, from ServiceState in src/chains/access.rs. The
# message is the headline; .detail says what happens instead.
connectivity-service-checking = Connecting to the { -brand } service
    .detail = Trading starts when the connection is confirmed.
connectivity-service-ready = { -brand } service connected
    .detail = Wallet reads and trades go through screenerbot.io.
connectivity-service-signed-out = Trading paused · Signed out
    .detail = Sign in to resume. Open positions are not sold while trading is paused.
connectivity-service-reauthorize = Trading paused · Sign in again needed
    .detail = This device's sign-in does not allow wallet reads yet. Sign in again to resume.
connectivity-service-busy = Trading paused · Service busy, retrying
    .detail = Trading resumes automatically. To avoid pauses, use your own RPC.
connectivity-service-daily-limit = Trading paused · Daily limit reached
    .detail = Your account reached today's limit for the { -brand } service. Trading resumes at 00:00 UTC, or right away with your own RPC.
connectivity-service-unreachable = Trading paused · screenerbot.io unreachable
    .detail = Trading resumes when the connection returns. Check your internet connection.
connectivity-service-version = Trading paused · Update needed
    .detail = This version is no longer supported by the { -brand } service. Update to { $minimum } or newer.
