# Building ScreenerBot from Source

ScreenerBot is a Rust + Electron desktop application for automated Solana DeFi trading. This guide covers building the Rust trading engine and packaging it as a desktop app.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Building the Rust Engine](#building-the-rust-engine)
  - [Debug Build](#debug-build)
  - [Release Build](#release-build)
  - [Build Profiles](#build-profiles)
- [Building the Desktop App (Electron)](#building-the-desktop-app-electron)
  - [Development Mode](#development-mode)
  - [Production Packaging](#production-packaging)
- [Platform-Specific Notes](#platform-specific-notes)
  - [macOS](#macos)
  - [Windows](#windows)
  - [Linux](#linux)
- [Cross-Compilation](#cross-compilation)
- [Headless Mode (No GUI)](#headless-mode-no-gui)
- [Dependencies](#dependencies)
  - [Direct Dependencies](#direct-dependencies)
  - [Full Dependency Tree](#full-dependency-tree)
- [Build Output Sizes](#build-output-sizes)
- [Build Times](#build-times)
- [Troubleshooting](#troubleshooting)
- [Pre-built Downloads](#pre-built-downloads)

---

## Overview

ScreenerBot has two main components:

1. **Rust Trading Engine** (`screenerbot` binary) — The core trading engine that handles pool discovery, token analysis, swap execution, wallet management, and the built-in web dashboard. This is a standalone binary that can run independently (headless mode).

2. **Electron Shell** (`electron/`) — A lightweight desktop wrapper that launches the Rust engine, provides a native window with the web dashboard, system tray integration, and auto-updates.

The build process compiles the Rust binary first, then packages it inside the Electron app.

## Prerequisites

### Required

| Tool | Minimum Version | Purpose |
|------|----------------|---------|
| **Rust** | 1.75+ (2021 edition) | Compile the trading engine |
| **Cargo** | Comes with Rust | Rust package manager |
| **Node.js** | 18+ | Electron packaging |
| **npm** | 9+ | Comes with Node.js |

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Install Node.js

Use [nvm](https://github.com/nvm-sh/nvm) (recommended) or download from [nodejs.org](https://nodejs.org):

```bash
nvm install 22
nvm use 22
```

### Platform-Specific Requirements

**macOS:**
- Xcode Command Line Tools: `xcode-select --install`
- For DMG creation: `hdiutil` (included with macOS)

**Windows:**
- Visual Studio Build Tools with C++ workload
- [WiX Toolset v4+](https://wixtoolset.org/) (for MSI installer creation)
- [LLVM/LLD](https://llvm.org/) (recommended for ARM64 builds)

**Linux:**
- Build essentials: `sudo apt install build-essential pkg-config libssl-dev`
- For `.deb` packages: `dpkg-deb` (included with Debian/Ubuntu)
- For `.rpm` packages: `rpmbuild` (`sudo apt install rpm`)

## Quick Start

```bash
# Clone the repository
git clone https://github.com/screenerbotio/ScreenerBot.git
cd ScreenerBot

# Build the Rust engine (release mode)
cargo build --release

# The binary is at: target/release/screenerbot

# Run it directly (headless mode - no GUI needed)
./target/release/screenerbot

# Or package as a desktop app (see Electron section below)
```

## Building the Rust Engine

### Debug Build

Fast compilation, no optimizations. Good for development:

```bash
cargo build
```

- Output: `target/debug/screenerbot`
- Compile time: ~2-4 minutes (first build), ~10-30 seconds (incremental)
- Binary has no debug symbols (disabled for faster compilation)

### Release Build

Optimized for performance. Use this for production:

```bash
cargo build --release
```

- Output: `target/release/screenerbot`
- Compile time: ~5-15 minutes (first build), ~1-3 minutes (incremental)
- Binary is stripped of symbols for smaller size

### Build Profiles

ScreenerBot uses custom build profiles optimized for compilation speed:

| Setting | Debug | Release |
|---------|-------|---------|
| `opt-level` | 0 (no optimization) | 2 (good performance) |
| `lto` | false | false (saves 5-10 min) |
| `codegen-units` | 256 (max parallelism) | 256 (max parallelism) |
| `incremental` | true | true |
| `debug` | 0 (no debug info) | false |
| `panic` | abort | abort |
| `strip` | — | symbols (all stripped) |

> **Note:** LTO (Link-Time Optimization) is intentionally disabled to keep build times reasonable. The performance difference is minimal for this workload. `opt-level = 2` is used instead of `3` for the same reason — it provides good runtime performance with significantly faster compilation.

### Cargo Features

Optional features can be enabled during build:

```bash
# Enable tokio-console for async debugging
cargo build --release --features console

# Enable CPU flamegraph profiling
cargo build --release --features flamegraph

# Enable Electron DevTools (for development)
cargo build --release --features devtools
```

| Feature | Purpose |
|---------|---------|
| `console` | tokio-console async task inspection |
| `flamegraph` | CPU profiling with pprof |
| `devtools` | Enable Electron DevTools in development |

## Building the Desktop App (Electron)

The Electron shell wraps the Rust binary in a native desktop application with window management, system tray, and platform-specific installers.

### Development Mode

```bash
# Install Electron dependencies
cd electron
npm install

# Copy the compiled Rust binary (must build Rust first)
cp ../target/release/screenerbot .   # macOS/Linux
# or
copy ..\target\release\screenerbot.exe .   # Windows

# Start in development mode
npm start
```

### Production Packaging

Packaging creates distributable installers for each platform:

```bash
cd electron
npm install

# Copy the release binary to the expected location
# (Electron Forge looks for it in target/release/)

# Create platform-specific packages
npm run make
```

Output locations:
- **macOS**: `electron/out/make/` — `.dmg` and `.zip`
- **Windows**: `electron/out/make/wix/` — `.msi`, and `electron/out/make/zip/` — `.zip`
- **Linux**: `electron/out/make/deb/` — `.deb`, and `electron/out/make/zip/` — `.zip`

### Electron Forge Configuration

The packaging is configured in `electron/forge.config.js`:

- **macOS**: DMG with drag-to-Applications layout, dark mode support
- **Windows**: WiX MSI installer with custom install directory, VC++ redistributable bundled
- **Linux**: `.deb` package with proper desktop integration, categorized under Finance

## Platform-Specific Notes

### macOS

- Supports both **x64** (Intel) and **arm64** (Apple Silicon) natively
- DMG is created with `UDZO` compression format
- Dark mode is supported by default
- Code signing and notarization can be enabled in `forge.config.js` (requires Apple Developer account)
- macOS-specific dependencies: `objc2` crates for native window management

```bash
# Build for current architecture
cargo build --release

# Cross-compile for the other architecture (if needed)
rustup target add aarch64-apple-darwin   # on Intel Mac
rustup target add x86_64-apple-darwin    # on Apple Silicon
cargo build --release --target aarch64-apple-darwin
```

### Windows

- Supports both **x64** and **arm64** architectures
- OpenSSL is vendored (compiled from source) — no system OpenSSL needed
- ARM64 builds use LLD linker to handle large link lines
- MSI installer is created with WiX Toolset
- VC++ Redistributable is bundled automatically

```bash
# Standard build
cargo build --release

# ARM64 cross-compilation (requires LLVM/LLD)
rustup target add aarch64-pc-windows-msvc
cargo build --release --target aarch64-pc-windows-msvc
```

### Linux

- Supports both **x64** and **arm64** architectures
- OpenSSL is vendored for maximum compatibility
- GLIBC requirement: **2.29+** (compatible with Ubuntu 20.04+, Debian 11+, Fedora 34+)
- Cross-compilation supported via [cross](https://github.com/cross-rs/cross) tool with Docker

```bash
# Standard build
cargo build --release

# Cross-compile for ARM64 (requires cross tool)
cargo install cross
cross build --release --target aarch64-unknown-linux-gnu
```

#### Linux Compatibility

| Distribution | Minimum Version | GLIBC |
|-------------|----------------|-------|
| Ubuntu | 20.04 | 2.31 |
| Debian | 11 | 2.31 |
| Fedora | 34 | 2.34 |
| Linux Mint | 20 | 2.31 |
| Pop!_OS | 20.04 | 2.31 |

## Cross-Compilation

ScreenerBot uses the [cross](https://github.com/cross-rs/cross) tool for Linux cross-compilation. Configuration is in `Cross.toml`:

```bash
# Install cross
cargo install cross

# Build for Linux x64 (from macOS or other platform)
cross build --release --target x86_64-unknown-linux-gnu

# Build for Linux ARM64
cross build --release --target aarch64-unknown-linux-gnu
```

Docker resource limits are configured in `Cross.toml`:
- Max 8 CPUs
- Max 8 GB RAM

## Headless Mode (No GUI)

The Rust binary can run without Electron for server deployments or headless environments:

```bash
# Build just the Rust binary
cargo build --release

# Run directly
./target/release/screenerbot

# Or with options
./target/release/screenerbot --help
```

In headless mode, the web dashboard is accessible via browser at `http://localhost:3333` (default port). All trading functionality works identically — the Electron shell is only a convenience wrapper.

**Headless binary size is significantly smaller** (~16 MB compressed) since it doesn't include Electron.

## Dependencies

### Direct Dependencies (90 crates)

ScreenerBot uses ~90 direct Rust crate dependencies, organized by category:

#### Core Runtime
| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.x | Async runtime (multi-threaded) |
| `tokio-metrics` | 0.4 | Runtime performance monitoring |
| `anyhow` | 1.0 | Error handling |
| `thiserror` | 2.0 | Custom error types |
| `serde` / `serde_json` | 1.0 | Serialization |
| `chrono` | 0.4 | Date/time handling |

#### Blockchain (Solana)
| Crate | Version | Purpose |
|-------|---------|---------|
| `solana-sdk` | 2.3.1 | Solana core SDK |
| `solana-client` | 2.1.0 | RPC client |
| `solana-program` | 2.3.0 | On-chain program interfaces |
| `solana-transaction-status` | 2.3.3 | Transaction parsing |
| `solana-account-decoder` | 2.3.1 | Account data decoding |
| `spl-token` / `spl-token-2022` | 8.0 | SPL token operations |
| `spl-associated-token-account` | 6.0.0 | ATA management |

#### Networking
| Crate | Version | Purpose |
|-------|---------|---------|
| `reqwest` | 0.11 | HTTP client (rustls TLS) |
| `tokio-tungstenite` | 0.21 | WebSocket client |
| `axum` | 0.7 | Web server (dashboard API) |
| `tower` / `tower-http` | 0.4/0.5 | HTTP middleware (CORS, compression) |
| `hyper` | 1.0 | HTTP primitives |

#### Database
| Crate | Version | Purpose |
|-------|---------|---------|
| `rusqlite` | 0.37 | SQLite (bundled, no system dep) |
| `r2d2` / `r2d2_sqlite` | 0.8/0.31 | Connection pooling |

#### Security & Crypto
| Crate | Version | Purpose |
|-------|---------|---------|
| `aes-gcm` | 0.10 | AES-256-GCM encryption |
| `sha2` | 0.10 | SHA-256 hashing |
| `blake3` | 1.5 | Fast hashing |
| `bs58` / `base64` | 0.5/0.22 | Encoding |
| `totp-rs` | 5.6 | 2FA TOTP authentication |

#### Telegram Bot
| Crate | Version | Purpose |
|-------|---------|---------|
| `teloxide` | 0.13 | Telegram bot framework |

#### System
| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4.0 | CLI argument parsing |
| `sysinfo` | 0.30 | System information |
| `dirs` | 6.0.0 | Platform-specific directories |
| `governor` | 0.6 | Rate limiting (GCRA algorithm) |
| `dashmap` | 5.5 | Concurrent hash maps |

#### Platform-Specific
| Crate | Platform | Purpose |
|-------|----------|---------|
| `objc2` / `objc2-app-kit` | macOS | Native window management |
| `machine-uid` | macOS/Win/Linux | Unique machine identifier |
| `openssl` (vendored) | Windows/Linux | TLS (compiled from source) |

### Full Dependency Tree

The complete resolved dependency tree contains **~833 crates** (including transitive dependencies). The Solana SDK alone brings in ~110 sub-crates. You can inspect the full tree:

```bash
# View full dependency tree
cargo tree

# View tree for a specific dependency
cargo tree -i solana-sdk

# Count total dependencies
grep -c '^\[\[package\]\]' Cargo.lock
```

### Electron Dependencies

The Electron shell has minimal dependencies (all dev-only):

| Package | Purpose |
|---------|---------|
| `electron` ^33.2.1 | Desktop runtime |
| `@electron-forge/cli` ^7.6.0 | Build tooling |
| `@electron-forge/maker-dmg` | macOS DMG creator |
| `@electron-forge/maker-deb` | Linux DEB creator |
| `@electron-forge/maker-rpm` | Linux RPM creator |
| `@electron-forge/maker-wix` | Windows MSI creator |
| `@electron-forge/maker-zip` | ZIP archive creator |

## Build Output Sizes

Sizes from version 0.1.110 builds (all platforms, both architectures):

### Desktop App (with Electron)

| Platform | Architecture | DMG/MSI | ZIP |
|----------|-------------|---------|-----|
| macOS | arm64 (Apple Silicon) | 178 MB | 178 MB |
| macOS | x64 (Intel) | 183 MB | 183 MB |
| Windows | arm64 | 234 MB | 238 MB |
| Windows | x64 | 236 MB | 236 MB |
| Linux | arm64 | 151 MB (.deb) | 193 MB |
| Linux | x64 | 152 MB (.deb) | 188 MB |

### Headless Binary (Rust only, no Electron)

| Platform | Architecture | Compressed (.tar.gz) |
|----------|-------------|---------------------|
| Linux | arm64 | ~16 MB |
| Linux | x64 | ~16 MB |

> **Why are desktop builds large?** The bulk of the size (~120-150 MB) comes from bundling the Electron runtime (Chromium). The Rust binary itself is ~30-40 MB. Headless builds that skip Electron are dramatically smaller.

### Build Artifact Directory

During compilation, the `target/` directory can grow to **25 GB or more** due to intermediate build artifacts, cached dependencies, incremental compilation data, and multiple build profiles (debug + release). This is normal for large Rust projects with many dependencies like the Solana SDK.

```bash
# Check target directory size
du -sh target/

# Clean all build artifacts
cargo clean

# Clean only release artifacts
cargo clean --release
```

## Build Times

Build times vary significantly based on hardware. These are approximate times:

### First Build (clean, no cache)

| Hardware | Debug | Release |
|----------|-------|---------|
| Apple Silicon (M1/M2/M3) | 2-4 min | 5-10 min |
| Modern x64 (8+ cores) | 3-6 min | 8-15 min |
| Older hardware (4 cores) | 8-15 min | 15-30 min |

### Incremental Build (after code changes)

| Hardware | Debug | Release |
|----------|-------|---------|
| Apple Silicon (M1/M2/M3) | 5-15 sec | 30-90 sec |
| Modern x64 (8+ cores) | 10-30 sec | 1-3 min |

> **Tip:** Use debug builds during development for faster iteration. Only build in release mode for testing performance or creating distributable packages.

### Speeding Up Builds

```bash
# Use all CPU cores (default, but ensure it's set)
export CARGO_BUILD_JOBS=$(nproc 2>/dev/null || sysctl -n hw.ncpu)

# Use mold linker on Linux (much faster linking)
# Install: sudo apt install mold
RUSTFLAGS="-C link-arg=-fuse-ld=mold" cargo build --release

# Use sccache for caching across builds
cargo install sccache
export RUSTC_WRAPPER=sccache
```

## Troubleshooting

### Common Issues

**OpenSSL errors on Windows/Linux:**
OpenSSL is vendored (compiled from source). If you see OpenSSL build errors, ensure you have a C compiler installed:
- Windows: Visual Studio Build Tools
- Linux: `sudo apt install build-essential`

**Linker errors with many symbols:**
On Windows ARM64, if you see linker errors about command-line length, ensure LLD is installed and configured in `.cargo/config.toml`.

**Out of memory during compilation:**
The Solana SDK is large. Ensure at least 4 GB of free RAM. Reduce parallel compilation if needed:
```bash
CARGO_BUILD_JOBS=2 cargo build --release
```

**SQLite build errors:**
SQLite is bundled (compiled from source via `rusqlite`). Ensure a C compiler is available. No system SQLite installation is needed.

**Electron packaging fails:**
```bash
cd electron
rm -rf node_modules out
npm install
npm run make
```

### Clean Build

If you encounter persistent issues, do a full clean build:

```bash
# Clean Rust artifacts
cargo clean

# Clean Electron artifacts
cd electron
rm -rf node_modules out
npm install

# Rebuild everything
cd ..
cargo build --release
cd electron
npm run make
```

## Pre-built Downloads

If you don't want to build from source, pre-built binaries are available for all platforms:

**[Download ScreenerBot →](https://screenerbot.io/download)**

Available formats:
- **macOS**: `.dmg` (Intel & Apple Silicon)
- **Windows**: `.msi` installer and `.zip` (x64 & ARM64)
- **Linux**: `.deb` package and `.zip` (x64 & ARM64)
- **Linux Headless**: `.tar.gz` (x64 & ARM64) — Rust binary only, no GUI
