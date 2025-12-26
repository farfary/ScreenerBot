# ScreenerBot Electron Edition

Electron wrapper for ScreenerBot that packages the Rust binary as a native desktop application.

## Prerequisites

- Node.js 18+ and npm
- Rust toolchain (for building the backend)
- macOS for building .app/.dmg

## Development

### 1. Build the Rust binary first

```bash
# From the root directory
cargo build --release
```

### 2. Install dependencies

```bash
cd electron
npm install
```

### 3. Run in development mode

```bash
npm start
```

This will:
1. Start Electron
2. Spawn the screenerbot binary from `target/release/`
3. Wait for the webserver to be ready
4. Load the dashboard in the window

## Production Build

### Package the application

```bash
npm run package
```

This creates the `.app` bundle in `out/` directory.

### Create distributable (DMG + ZIP)

```bash
npm run make
```

This creates:
- `out/make/ScreenerBot-x.x.x-arm64.dmg` - DMG installer
- `out/make/zip/darwin/arm64/ScreenerBot-darwin-arm64-x.x.x.zip` - ZIP archive

## Project Structure

```
electron/
├── package.json        # Dependencies and scripts
├── forge.config.js     # Electron Forge configuration
├── src/
│   ├── main.js        # Main process (spawns backend, creates window)
│   ├── preload.js     # Preload script (contextBridge API)
│   └── index.html     # Loading page shown during startup
├── assets/
│   ├── icon.icns      # macOS app icon
│   └── dmg-background.png  # DMG background image
└── out/               # Build output (gitignored)
```

## How It Works

1. **Startup**: Electron spawns the `screenerbot` binary from resources
2. **Health Check**: Polls `http://127.0.0.1:8080/api/status` until ready
3. **Load App**: Once backend is ready, loads the dashboard URL
4. **Shutdown**: On quit, sends SIGTERM to gracefully stop the backend

## Configuration

Edit `src/main.js` to change:

- `CONFIG.port` - Backend webserver port (default: 8080)
- `CONFIG.healthEndpoint` - Health check endpoint
- `CONFIG.maxWaitTime` - Max time to wait for backend (default: 2 minutes)
- Window dimensions and behavior

## Troubleshooting

### Backend fails to start

1. Check that the binary exists: `ls -la ../target/release/screenerbot`
2. Ensure it's executable: `chmod +x ../target/release/screenerbot`
3. Try running it directly to check for errors

### Window shows loading forever

1. Check console output for backend errors
2. Verify the health endpoint is correct
3. Increase `CONFIG.maxWaitTime` if needed

### Icons not showing

1. Ensure `assets/icon.icns` exists
2. Follow the README in `assets/` to generate icons
