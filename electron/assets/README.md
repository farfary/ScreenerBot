# ScreenerBot Electron Assets

This directory contains assets for the Electron build:

## Required Files

### Icons
- `icon.icns` - macOS app icon (required for packaging)
- `icon.png` - PNG version for other uses

### DMG Background (optional)
- `dmg-background.png` - Custom background for DMG installer (540x380 recommended)

## Generating Icons

To create the `.icns` file from a PNG:

```bash
# Create iconset directory
mkdir icon.iconset

# Generate all required sizes
sips -z 16 16     icon.png --out icon.iconset/icon_16x16.png
sips -z 32 32     icon.png --out icon.iconset/icon_16x16@2x.png
sips -z 32 32     icon.png --out icon.iconset/icon_32x32.png
sips -z 64 64     icon.png --out icon.iconset/icon_32x32@2x.png
sips -z 128 128   icon.png --out icon.iconset/icon_128x128.png
sips -z 256 256   icon.png --out icon.iconset/icon_128x128@2x.png
sips -z 256 256   icon.png --out icon.iconset/icon_256x256.png
sips -z 512 512   icon.png --out icon.iconset/icon_256x256@2x.png
sips -z 512 512   icon.png --out icon.iconset/icon_512x512.png
sips -z 1024 1024 icon.png --out icon.iconset/icon_512x512@2x.png

# Convert to icns
iconutil -c icns icon.iconset

# Cleanup
rm -rf icon.iconset
```

## Notes

- Icon should be at least 1024x1024 PNG for best quality
- The forge.config.js references these files for packaging
