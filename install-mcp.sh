#!/usr/bin/env bash
#
# ScreenerBot MCP Installer
# Install and configure the ScreenerBot MCP server for Claude Desktop, Cursor, etc.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/screenerbotio/ScreenerBot/main/install-mcp.sh | sh
#
set -euo pipefail

PACKAGE="@screenerbot/mcp"
MIN_NODE_VERSION=18

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()  { printf "${BLUE}ℹ${NC}  %s\n" "$1"; }
ok()    { printf "${GREEN}✓${NC}  %s\n" "$1"; }
warn()  { printf "${YELLOW}⚠${NC}  %s\n" "$1"; }
fail()  { printf "${RED}✗${NC}  %s\n" "$1"; exit 1; }

echo ""
echo "╔═══════════════════════════════════════════╗"
echo "║     ScreenerBot MCP Server Installer      ║"
echo "╚═══════════════════════════════════════════╝"
echo ""

# ── Check Node.js ──
if ! command -v node &>/dev/null; then
  fail "Node.js is not installed. Install Node.js >= ${MIN_NODE_VERSION} from https://nodejs.org"
fi

NODE_VERSION=$(node -v | sed 's/v//' | cut -d. -f1)
if [ "$NODE_VERSION" -lt "$MIN_NODE_VERSION" ]; then
  fail "Node.js ${MIN_NODE_VERSION}+ required. You have $(node -v). Update from https://nodejs.org"
fi
ok "Node.js $(node -v) detected"

# ── Check npm ──
if ! command -v npm &>/dev/null; then
  fail "npm is not installed"
fi
ok "npm $(npm -v) detected"

# ── Install the MCP package ──
info "Installing ${PACKAGE}..."
npm install -g "${PACKAGE}" --silent 2>/dev/null || npm install -g "${PACKAGE}"
ok "Installed ${PACKAGE}"

# ── Detect Claude Desktop config path ──
CLAUDE_CONFIG=""
if [ "$(uname)" = "Darwin" ]; then
  CLAUDE_CONFIG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
elif [ "$(uname)" = "Linux" ]; then
  CLAUDE_CONFIG="$HOME/.config/claude/claude_desktop_config.json"
elif [ -d "$APPDATA" ]; then
  CLAUDE_CONFIG="$APPDATA/Claude/claude_desktop_config.json"
fi

# ── Get user input ──
echo ""
info "Configuration"
echo ""

DEFAULT_URL="http://127.0.0.1:3000"
printf "  ScreenerBot URL [${DEFAULT_URL}]: "
read -r SCREENERBOT_URL
SCREENERBOT_URL="${SCREENERBOT_URL:-$DEFAULT_URL}"

printf "  Security Token (from bot dashboard, or leave empty): "
read -r SCREENERBOT_TOKEN
SCREENERBOT_TOKEN="${SCREENERBOT_TOKEN:-}"

# ── Configure Claude Desktop ──
if [ -n "$CLAUDE_CONFIG" ]; then
  echo ""
  printf "  Add to Claude Desktop config? [Y/n]: "
  read -r ADD_CLAUDE
  ADD_CLAUDE="${ADD_CLAUDE:-Y}"

  if [ "$ADD_CLAUDE" = "Y" ] || [ "$ADD_CLAUDE" = "y" ]; then
    CONFIG_DIR=$(dirname "$CLAUDE_CONFIG")
    mkdir -p "$CONFIG_DIR"

    if [ -f "$CLAUDE_CONFIG" ]; then
      # Config exists — check if screenerbot already there
      if grep -q '"screenerbot"' "$CLAUDE_CONFIG" 2>/dev/null; then
        warn "ScreenerBot already configured in Claude Desktop. Edit manually if needed:"
        info "  $CLAUDE_CONFIG"
      else
        # Try to add to existing config using node
        node -e "
          const fs = require('fs');
          const path = '$CLAUDE_CONFIG';
          const config = JSON.parse(fs.readFileSync(path, 'utf-8'));
          if (!config.mcpServers) config.mcpServers = {};
          config.mcpServers.screenerbot = {
            command: 'npx',
            args: ['-y', '${PACKAGE}'],
            env: {
              SCREENERBOT_URL: '${SCREENERBOT_URL}',
              SCREENERBOT_TOKEN: '${SCREENERBOT_TOKEN}'
            }
          };
          fs.writeFileSync(path, JSON.stringify(config, null, 2));
        " 2>/dev/null && ok "Added to Claude Desktop config" || warn "Could not update config automatically"
      fi
    else
      # Create new config
      cat > "$CLAUDE_CONFIG" << EOF
{
  "mcpServers": {
    "screenerbot": {
      "command": "npx",
      "args": ["-y", "${PACKAGE}"],
      "env": {
        "SCREENERBOT_URL": "${SCREENERBOT_URL}",
        "SCREENERBOT_TOKEN": "${SCREENERBOT_TOKEN}"
      }
    }
  }
}
EOF
      ok "Created Claude Desktop config"
    fi
  fi
fi

# ── Test connection ──
echo ""
info "Testing connection to ${SCREENERBOT_URL}..."
if command -v curl &>/dev/null; then
  HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${SCREENERBOT_URL}/api/health" --max-time 5 2>/dev/null || echo "000")
  if [ "$HTTP_CODE" = "200" ]; then
    ok "ScreenerBot is reachable!"
  elif [ "$HTTP_CODE" = "000" ]; then
    warn "Cannot reach ScreenerBot at ${SCREENERBOT_URL}"
    info "Make sure the bot is running, then the MCP server will connect automatically"
  else
    warn "ScreenerBot returned HTTP ${HTTP_CODE}"
  fi
else
  warn "curl not found — skipping connection test"
fi

# ── Done ──
echo ""
echo "╔═══════════════════════════════════════════╗"
echo "║          Installation Complete!           ║"
echo "╚═══════════════════════════════════════════╝"
echo ""
info "Usage:"
echo "  • Restart Claude Desktop to load the MCP server"
echo "  • Ask Claude: \"Check my ScreenerBot status\""
echo "  • Or run directly: npx ${PACKAGE}"
echo ""
info "Config: ${CLAUDE_CONFIG:-'Set up manually for your MCP client'}"
info "Docs:   https://github.com/screenerbotio/ScreenerBot-MCP"
echo ""
