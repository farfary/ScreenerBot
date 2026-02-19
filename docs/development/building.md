# ScreenerBot Cross-Platform Build Guide

Complete guide for building ScreenerBot for all platforms from a single macOS machine using Parallels Desktop VMs.

**Last Updated:** December 2025

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Build Scripts Reference](#build-scripts-reference)
3. [VM Configuration](#vm-configuration)
4. [Build Performance](#build-performance)
5. [Output Files](#output-files)
6. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Build All Platforms

```bash
# Build everything (macOS + Linux + Windows)
./build-all.sh

# Build specific platforms
./build-all.sh --macos        # macOS only
./build-all.sh --linux        # Linux only (via VM)
./build-all.sh --windows      # Windows only (via VM)

# Build VMs in parallel (experimental)
./build-all.sh --linux --windows --parallel
```

### Build Individual Platforms

```bash
# macOS (native on host - Electron)
./build-macos.sh           # Current architecture
./build-macos.sh --intel   # Intel only
./build-macos.sh --arm     # ARM only

# Linux (via Ubuntu VM - requires update to Electron workflow)
./build-vm-linux.sh                 # x86_64 release

# Windows (via Windows VM - requires update to Electron workflow)
./build-vm-windows.sh               # x64 release
```

---

## Build Scripts Reference

| Script                | Runs On    | Builds For | Description                            |
| --------------------- | ---------- | ---------- | -------------------------------------- |
| `build-macos.sh`      | macOS host | macOS      | Electron macOS build (Intel, ARM)      |
| `build-vm-linux.sh`   | macOS host | Linux      | Linux build via Parallels Ubuntu VM    |
| `build-vm-windows.sh` | macOS host | Windows    | Windows build via Parallels Windows VM |
| `build-all.sh`        | macOS host | All        | Orchestrates all builds                |

**Note:** The Electron migration is in progress. macOS builds use the new Electron workflow. Linux and Windows VM builds are being updated.

---

## VM Configuration

### Host System

| Component         | Value                            |
| ----------------- | -------------------------------- |
| Machine           | MacBook Pro 16,1 (Intel Core i9) |
| macOS             | 26.1 (Tahoe)                     |
| Parallels Desktop | 26.2.0                           |

### Ubuntu VM (Linux Builds)

| Setting       | Value                    |
| ------------- | ------------------------ |
| VM Name       | `Ubuntu`                 |
| CPUs          | 8 cores                  |
| RAM           | 8192 MB (8 GB)           |
| OS            | Ubuntu 24.04 LTS         |
| Shared Folder | `/media/psf/ScreenerBot` |

#### Installed Tools

| Tool       | Version | Path                  |
| ---------- | ------- | --------------------- |
| Rust/Cargo | 1.91.1  | `/.cargo/bin/cargo`   |
| Node.js    | 22.21.0 | `node`                |
| sccache    | 0.12.0  | `/.cargo/bin/sccache` |
| clang      | 18.0    | `clang`               |
| lld        | 18.0    | `lld`                 |

#### Cargo Configuration (`/.cargo/config.toml`)

```toml
[build]
rustc-wrapper = "/.cargo/bin/sccache"
jobs = 8

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

#### Required Ubuntu Packages

```bash
# Build essentials
apt-get install -y build-essential curl wget git pkg-config libssl-dev clang lld

# Node.js 22 (for Electron)
curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
apt-get install -y nodejs

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# sccache (build cache)
cargo install sccache
```

### Windows VM (Windows Builds)

| Setting       | Value               |
| ------------- | ------------------- |
| VM Name       | `Windows`           |
| CPUs          | 8 cores             |
| RAM           | 8192 MB (8 GB)      |
| OS            | Windows 11          |
| Shared Folder | `\\psf\ScreenerBot` |

#### Installed Tools

| Tool           | Version | Path                                        |
| -------------- | ------- | ------------------------------------------- |
| Rust/Cargo     | 1.91.1  | `C:\Users\Farhad\.cargo\bin\cargo`          |
| Node.js        | 24.11.1 | `node`                                      |
| sccache        | 0.12.0  | `C:\ProgramData\chocolatey\bin\sccache.exe` |
| lld-link       | 21.1.0  | `C:\Program Files\LLVM\bin\lld-link.exe`    |
| OpenSSL        | 3.6.0   | `C:\Program Files\OpenSSL-Win64`            |
| VS Build Tools | 2022    | Desktop C++ workload                        |

#### Important: Electron Builds

For Electron builds on Windows, use the Electron Forge workflow:

```powershell
cd electron
npm install
npm run make
```

#### Cargo Configuration (`C:\Users\Farhad\.cargo\config.toml`)

```toml
[build]
rustc-wrapper = "sccache"
jobs = 8

[target.x86_64-pc-windows-msvc]
linker = "lld-link"
```

**Note:** sccache must be in PATH. lld-link is from LLVM and must be installed via `choco install llvm`.

#### Installation Commands (via Chocolatey)

```powershell
# Install Chocolatey first
Set-ExecutionPolicy Bypass -Scope Process -Force
iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

# Install all build tools
choco install -y git llvm nodejs-lts openssl cmake sccache
choco install -y visualstudio2022buildtools --package-parameters '--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive'

# Install Rust (via rustup)
winget install Rustlang.Rustup


# Set OpenSSL environment variable
[System.Environment]::SetEnvironmentVariable("OPENSSL_DIR", "C:\Program Files\OpenSSL-Win64", "User")
```

---

## Build Performance

### Typical Build Times

| Build Type   | First Build | Incremental  | With sccache |
| ------------ | ----------- | ------------ | ------------ |
| Linux x86_64 | ~14 minutes | ~2-3 minutes | ~1-2 minutes |
| Windows x64  | ~15 minutes | ~3-4 minutes | ~2-3 minutes |
| macOS Intel  | ~12 minutes | ~2-3 minutes | ~1-2 minutes |

### Performance Optimizations

1. **sccache** - Compilation cache for faster rebuilds
2. **lld linker** - Faster linking than default ld
3. **8 parallel jobs** - Uses all VM cores
4. **Local disk builds** - Projects synced to VM local disk (faster than network)
5. **Shared folders** - Direct access to project (171 MB/s write speed)

### ⚠️ Critical Rules

- **NEVER run `cargo clean`** - Destroys cache, causes 14+ minute rebuilds
- **NEVER stop VMs unnecessarily** - Use pause for quick resume
- **Scripts only clean bundle directories** - Preserves incremental compilation cache
- VM pause is faster than stop - sccache persists across VM restarts
- `CI=true` environment variable prevents DMG from opening in Finder

---

## Output Files

### Build Artifacts Location

```
builds/
├── macos/
│   ├── x86_64-apple-darwin/
│   │   ├── ScreenerBot.app
│   │   ├── ScreenerBot_x.x.x_x64.dmg
│   │   └── screenerbot
│   ├── aarch64-apple-darwin/
│   │   ├── ScreenerBot.app
│   │   ├── ScreenerBot_x.x.x_aarch64.dmg
│   │   └── screenerbot
│   └── universal-apple-darwin/
│       ├── ScreenerBot.app
│       └── screenerbot
├── linux/
│   └── x86_64-unknown-linux-gnu/
│       ├── ScreenerBot_x.x.x_amd64.deb
│       └── ScreenerBot_x.x.x_amd64.AppImage
└── windows/
    └── x86_64-pc-windows-msvc/
        ├── ScreenerBot_x.x.x_x64-setup.exe    # NSIS installer
        └── screenerbot.exe                     # Standalone EXE
```

### File Types by Platform

| Platform | File Types                                 | Size (approx)   |
| -------- | ------------------------------------------ | --------------- |
| macOS    | `.app`, `.dmg`                             | ~45 MB          |
| Windows  | `.exe` (NSIS installer), standalone `.exe` | 10 MB / 39 MB   |
| Linux    | `.deb`, `.AppImage`                        | ~16 MB / ~87 MB |

**Note:** Electron builds produce platform-native installers. See `electron/forge.config.js` for bundle configuration.

---

## Troubleshooting

### Common Issues

#### 1. Electron Build Fails

**Problem:** `npm run make` fails in the electron directory.

**Solution:** Ensure all dependencies are installed:

```bash
cd electron
npm install
npm run make
```

#### 2. VM Won't Start in Headless Mode

**Problem:** Old Parallels versions don't support `--headless` flag.

**Solution:** The build scripts configure VMs for headless mode via:

```bash
prlctl set Ubuntu --on-window-close keep-running
prlctl set Ubuntu --startup-view headless
```

After builds, they restore to `--startup-view window`.

#### 3. Slow Build Times

**Causes:**

- sccache not configured
- Low VM RAM
- Using default linker instead of lld
- Building from shared folder instead of local disk

**Solutions:**

1. Verify sccache: `/.cargo/bin/sccache --show-stats`
2. Increase VM RAM to 8GB
3. Check `/.cargo/config.toml` has lld configured
4. Scripts sync project to local VM disk before building

#### 4. "linker 'cc' not found"

**Solution:** Install clang and configure as linker:

```bash
apt-get install -y clang
```

#### 5. Shared Folder Not Accessible

**Check:**

```bash
ls -la /media/psf/ScreenerBot
```

**Solution:** In Parallels → VM Settings → Options → Sharing → Enable "Share Mac folders with Linux"

#### 6. Rust Binary Not Found

**Problem:** Electron can't find the screenerbot binary.

**Solution:** Build the Rust binary first before running Electron:

```bash
cargo build --release --bin screenerbot
cd electron
npm start
```

**Current Status:**

- ✅ macOS ARM64 works (Xcode can cross-compile natively)
- ❌ Linux ARM64 needs cross-compilation sysroot (complex setup)
- ❌ Windows ARM64 needs ARM Windows device

---

## VM Management Commands

### Start/Stop/Pause

```bash
# List all VMs
prlctl list -a

# Start VM (GUI window opens)
prlctl start Ubuntu

# Pause VM (fast resume, saves state)
prlctl pause Ubuntu

# Resume paused VM
prlctl resume Ubuntu

# Stop VM (full shutdown)
prlctl stop Ubuntu

# Force stop
prlctl stop Ubuntu --fast
```

### Execute Commands in VM

```bash
# Single command
prlctl exec Ubuntu "command here"

# With environment
prlctl exec Ubuntu "export PATH=\$PATH:/.cargo/bin && cargo --version"
```

### VM Configuration

```bash
# Check VM info
prlctl list -i Ubuntu

# Set RAM (requires stopped VM)
prlctl stop Ubuntu --fast
prlctl set Ubuntu --memsize 8192
prlctl start Ubuntu

# Set CPUs
prlctl set Ubuntu --cpus 8

# Configure headless mode
prlctl set Ubuntu --startup-view headless
prlctl set Ubuntu --on-window-close keep-running

# Restore window mode
prlctl set Ubuntu --startup-view window
prlctl set Ubuntu --on-window-close close
```

---

## Architecture Notes

### Why VMs Instead of Cross-Compilation?

Electron benefits from native platform builds because:

1. **Platform Bundlers** - Electron Forge uses platform-specific bundlers
2. **Solana SDK** - Complex native cryptography code requires platform-specific compilation
3. **OpenSSL** - Requires platform-specific configuration
4. **Native Dependencies** - Some npm packages have native bindings

### VM Approach Benefits

- ✅ 100% reliable builds
- ✅ Same environment as production
- ✅ All bundlers work (NSIS, AppImage, DMG)
- ✅ Easy debugging via VM GUI
- ✅ Fast resume from paused state

### Build Script VM Lifecycle

1. **Before build**: Save original VM settings, configure for headless (8 CPUs, 8GB RAM)
2. **Start VM**: Resume if paused, otherwise start
3. **Sync project**: Copy to local VM disk (faster than network)
4. **Build**: Run `cargo build` for Rust, then `npm run make` for Electron
5. **Copy artifacts**: Copy to builds/ folder on host
6. **After build**: Restore original VM settings, pause VM

### Alternative: GitHub Actions

For CI/CD, GitHub Actions provides native runners:

- `macos-latest` - macOS builds
- `ubuntu-latest` - Linux builds
- `windows-latest` - Windows builds

See `.github/workflows/build.yml` for automated releases.

---

## Platform Support Summary

| Platform | Architecture          | Status     | Notes                       |
| -------- | --------------------- | ---------- | --------------------------- |
| macOS    | Intel (x86_64)        | ✅ Works   | Native build                |
| macOS    | ARM64 (Apple Silicon) | ✅ Works   | Cross-compile from Intel    |
| macOS    | Universal             | ✅ Works   | Combined Intel + ARM        |
| Linux    | x86_64                | ✅ Works   | Via Ubuntu VM               |
| Linux    | ARM64                 | ❌ Limited | Needs cross-compile sysroot |
| Windows  | x64                   | ✅ Works   | Via Windows VM              |
| Windows  | ARM64                 | ❌ Limited | Needs ARM Windows device    |
