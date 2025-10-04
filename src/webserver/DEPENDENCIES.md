# Webserver Module - Dependencies to Add

## Required Dependencies

Add these to `Cargo.toml` when ready to implement:

```toml
[dependencies]
# Web framework (Phase 1)
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "compression"] }
hyper = "1.0"

# System information (Phase 1)
sysinfo = "0.30"

# Additional utilities (Phase 2+)
axum-extra = { version = "0.9", features = ["typed-header"] }
mime = "0.3"
uuid = { version = "1.6", features = ["v4", "serde"] }

# Authentication (Phase 3 - optional)
jsonwebtoken = "9.2"
bcrypt = "0.15"

# Rate limiting (Phase 3 - optional)
governor = "0.6"

# Metrics (Phase 3 - optional)
prometheus = "0.13"
```

## Already Available

These are already in the project:

- `tokio` - Async runtime ✅
- `serde` - Serialization ✅
- `serde_json` - JSON support ✅
- `chrono` - Date/time ✅
- `reqwest` - HTTP client ✅

## Installation

When ready to start Phase 1 implementation:

```bash
cargo add axum@0.7
cargo add tower@0.4
cargo add tower-http@0.5 --features cors,trace,compression
cargo add hyper@1.0
cargo add sysinfo@0.30
```

## Verification

After adding dependencies:

```bash
# Check compilation
cargo check --lib

# Run tests
cargo test --lib webserver
```
