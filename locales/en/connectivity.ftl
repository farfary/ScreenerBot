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
