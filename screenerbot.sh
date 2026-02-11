#!/bin/bash
# ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
# ┃                                                                                                  ┃
# ┃ ███████╗ ██████╗██████╗ ███████╗███████╗███╗   ██╗███████╗██████╗ ██████╗  ██████╗ ████████╗ ┃
# ┃ ██╔════╝██╔════╝██╔══██╗██╔════╝██╔════╝████╗  ██║██╔════╝██╔══██╗██╔══██╗██╔═══██╗╚══██╔══╝ ┃
# ┃ ███████╗██║     ██████╔╝█████╗  █████╗  ██╔██╗ ██║█████╗  ██████╔╝██████╔╝██║   ██║   ██║    ┃
# ┃ ╚════██║██║     ██╔══██╗██╔══╝  ██╔══╝  ██║╚██╗██║██╔══╝  ██╔══██╗██╔══██╗██║   ██║   ██║    ┃
# ┃ ███████║╚██████╗██║  ██║███████╗███████╗██║ ╚████║███████╗██║  ██║██████╔╝╚██████╔╝   ██║    ┃
# ┃ ╚══════╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝╚═════╝  ╚═════╝    ╚═╝    ┃
# ┃                                                                                                  ┃
# ┃                                         SCREENERBOT                                              ┃
# ┃                                                                                                  ┃
# ┃              ScreenerBot VPS Manager - Installation, Update & Management Tool                    ┃
# ┃              https://screenerbot.io                                                              ┃
# ┃                                                                                                  ┃
# ┃                            ◆ Automated Solana DeFi Trading Bot ◆                                 ┃
# ┃              Copyright © 2025 ScreenerBot. All rights reserved.                                  ┃
# ┃                                                                                                  ┃
# ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
#
# USAGE:
#   curl -fsSL https://screenerbot.io/install.sh | bash
#   OR
#   bash <(curl -fsSL https://raw.githubusercontent.com/screenerbotio/ScreenerBot/main/screenerbot.sh)
#   OR
#   wget -qO- https://screenerbot.io/install.sh | bash
#
# FEATURES:
#   • Install/Update/Uninstall ScreenerBot
#   • Systemd service management
#   • Backup and restore data
#   • Auto-update notifications via Telegram
#   • Version selection with API integration
#   • Architecture auto-detection (x64/arm64)
#
# =============================================================================

# Note: We intentionally don't use "set -e" because this is an interactive
# script where some commands may return non-zero (e.g., checking if file exists,
# grep not finding matches, optional features not configured, etc.)

# Ensure we have root privileges or can acquire them
ensure_root() {
    if [ "$EUID" -ne 0 ]; then
        # Check if sudo is available
        if command -v sudo &>/dev/null; then
            echo "This script requires root privileges."
            echo "Please enter your password if prompted."
            
            # Update sudo timestamp
            if sudo -v; then
                # Re-run script with sudo
                exec sudo bash "$0" "$@"
            else
                echo "Failed to acquire root privileges."
                exit 1
            fi
        else
            echo "Error: This script requires root privileges and sudo is not installed."
            echo "Please run as root."
            exit 1
        fi
    fi
}

# Call ensure_root immediately
ensure_root

# =============================================================================
# Configuration & Constants
# =============================================================================

readonly SCRIPT_VERSION="1.1.2"
readonly API_BASE="https://screenerbot.io/api"
readonly GITHUB_RAW="https://raw.githubusercontent.com/screenerbotio/ScreenerBot/main"
readonly INSTALL_DIR="/opt/screenerbot"
readonly SYMLINK_PATH="/usr/local/bin/screenerbot"
readonly MANAGER_PATH="/usr/local/bin/screenerbot-manager"
readonly SERVICE_NAME="screenerbot"
readonly SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
readonly UPDATE_TIMER_FILE="/etc/systemd/system/${SERVICE_NAME}-update.timer"
readonly UPDATE_SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}-update.service"

# Data directory detection (follows XDG spec)
get_data_dir() {
    local user="${SUDO_USER:-$USER}"
    local home_dir
    if [ -n "$SUDO_USER" ]; then
        home_dir=$(getent passwd "$SUDO_USER" | cut -d: -f6)
        # Ignore XDG_DATA_HOME when running as sudo to avoid using root's env
        echo "${home_dir}/.local/share/ScreenerBot"
    else
        home_dir="$HOME"
        if [ -n "${XDG_DATA_HOME:-}" ]; then
            echo "${XDG_DATA_HOME}/ScreenerBot"
        else
            echo "${home_dir}/.local/share/ScreenerBot"
        fi
    fi
}

# =============================================================================
# Colors & Formatting
# =============================================================================

# Check if terminal supports colors
if [ -t 1 ] && command -v tput &>/dev/null; then
    readonly RED=$(tput setaf 1)
    readonly GREEN=$(tput setaf 2)
    readonly YELLOW=$(tput setaf 3)
    readonly BLUE=$(tput setaf 4)
    readonly MAGENTA=$(tput setaf 5)
    readonly CYAN=$(tput setaf 6)
    readonly WHITE=$(tput setaf 7)
    readonly BOLD=$(tput bold)
    readonly DIM=$(tput dim)
    readonly RESET=$(tput sgr0)
else
    readonly RED=""
    readonly GREEN=""
    readonly YELLOW=""
    readonly BLUE=""
    readonly MAGENTA=""
    readonly CYAN=""
    readonly WHITE=""
    readonly BOLD=""
    readonly DIM=""
    readonly RESET=""
fi

# Icons (ASCII for maximum compatibility)
readonly ICON_CHECK="[+]"
readonly ICON_CROSS="[x]"
readonly ICON_ARROW="->"
readonly ICON_BULLET="*"
readonly ICON_INFO="[i]"
readonly ICON_WARN="[!]"
readonly ICON_ROCKET="*"
readonly ICON_PACKAGE="[+]"
readonly ICON_DOWNLOAD="[-]"
readonly ICON_UPDATE="[~]"
readonly ICON_TRASH="[x]"
readonly ICON_BACKUP="[S]"
readonly ICON_RESTORE="[R]"
readonly ICON_SERVICE="[=]"
readonly ICON_STATUS="[i]"
readonly ICON_BELL="[!]"
readonly ICON_HELP="[?]"
readonly ICON_EXIT="[Q]"
readonly ICON_BACK="<-"
readonly ICON_START=">"
readonly ICON_STOP="#"
readonly ICON_RESTART="~"
readonly ICON_LOGS="[L]"
readonly ICON_TELEGRAM="[T]"
readonly ICON_LOCK="[*]"
readonly ICON_MONITOR="[M]"

# =============================================================================
# Logging & UI Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}${ICON_INFO}${RESET} $1"
}

log_success() {
    echo -e "${GREEN}${ICON_CHECK}${RESET} $1"
}

log_warn() {
    echo -e "${YELLOW}${ICON_WARN}${RESET} $1"
}

log_error() {
    echo -e "${RED}${ICON_CROSS}${RESET} $1" >&2
}

log_step() {
    echo -e "\n${CYAN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo -e "${CYAN}${BOLD}▶ $1${RESET}"
    echo -e "${CYAN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
}

# Spinner animation for long-running tasks
spinner() {
    local pid=$1
    local message="${2:-Processing...}"
    local spinchars='|/-\'
    local i=0
    
    # Hide cursor if possible
    tput civis >&2 2>/dev/null || true
    
    while kill -0 "$pid" 2>/dev/null; do
        local char="${spinchars:$i:1}"
        printf "\r${CYAN}%s${RESET} %s" "$char" "$message" >&2
        i=$(( (i + 1) % ${#spinchars} ))
        sleep 0.1
    done
    printf "\r\033[K" >&2  # Clear line
    
    # Show cursor
    tput cnorm >&2 2>/dev/null || true
}

# Progress bar animation
progress_bar() {
    local current=$1
    local total=$2
    local width=40
    local percent=$((current * 100 / total))
    local filled=$((current * width / total))
    local empty=$((width - filled))
    
    printf "\r  [" >&2
    printf "%${filled}s" '' | tr ' ' '█' >&2
    printf "%${empty}s" '' | tr ' ' '░' >&2
    printf "] %3d%%" "$percent" >&2
}

# Interactive menu with number selection
# Usage: select_menu "option1" "option2" "option3"
# Returns: selected index (0-based) in $MENU_RESULT (internally converts from 1-based user input)
select_menu() {
    local options=("$@")
    local count=${#options[@]}
    
    # Ensure we have a terminal for input
    if [ ! -r /dev/tty ]; then
        echo "Error: No terminal available for interactive menu" >&2
        MENU_RESULT=0
        return 1
    fi
    
    # Print options (1-based for user-friendly display)
    echo ""
    for i in "${!options[@]}"; do
        printf "  [${CYAN}%d${RESET}] %s\n" "$((i + 1))" "${options[$i]}"
    done
    echo "  [${CYAN}Q${RESET}] Quit"
    echo ""
    
    while true; do
        local selection
        printf "  Select option [1-%d]: " "$count"
        
        # Read input from /dev/tty
        if ! read -r selection < /dev/tty; then
            # If read fails (EOF), exit loop
            MENU_RESULT=-1
            return 1
        fi
        
        # Handle Quit
        if [[ "$selection" =~ ^[qQ]$ ]]; then
            MENU_RESULT=-1
            return 0
        fi
        
        # Validate number (1-based input, convert to 0-based internally)
        if [[ "$selection" =~ ^[0-9]+$ ]] && [ "$selection" -ge 1 ] && [ "$selection" -le "$count" ]; then
            MENU_RESULT=$((selection - 1))
            return 0
        fi
        
        echo "  ${YELLOW}Invalid selection. Please try again.${RESET}"
    done
}

print_banner() {
    # Single source of truth for banner - call this function everywhere.
    
    # Cyan + Bold + Italic
    echo -e "${CYAN}${BOLD}\e[3m"

    # ANSI Shadow font (Robotic/Cyberpunk style) - 93 chars wide
    printf "   %s\n" "███████╗ ██████╗██████╗ ███████╗███████╗███╗   ██╗███████╗██████╗ ██████╗  ██████╗ ████████╗"
    printf "   %s\n" "██╔════╝██╔════╝██╔══██╗██╔════╝██╔════╝████╗  ██║██╔════╝██╔══██╗██╔══██╗██╔═══██╗╚══██╔══╝"
    printf "   %s\n" "███████╗██║     ██████╔╝█████╗  █████╗  ██╔██╗ ██║█████╗  ██████╔╝██████╔╝██║   ██║   ██║   "
    printf "   %s\n" "╚════██║██║     ██╔══██╗██╔══╝  ██╔══╝  ██║╚██╗██║██╔══╝  ██╔══██╗██╔══██╗██║   ██║   ██║   "
    printf "   %s\n" "███████║╚██████╗██║  ██║███████╗███████╗██║ ╚████║███████╗██║  ██║██████╔╝╚██████╔╝   ██║   "
    printf "   %s\n" "╚══════╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝╚═════╝  ╚═════╝    ╚═╝   "

    echo ""
    # SCREENERBOT (11 chars)
    printf "   %42s%s\n" "" "SCREENERBOT"
    echo ""
    # Subtitle (37 chars)
    printf "   %29s%s\n" "" "◆ Automated Solana DeFi Trading Bot ◆"
    echo ""
    
    # Social Links - Minimal Design
    # Left column: Website & Docs & X
    # Right column: Telegram Channel, Group & Support
    printf "   %15s %-30s %15s %-30s\n" "Website:" "screenerbot.io" "Channel:" "t.me/screenerbotio"
    printf "   %15s %-30s %15s %-30s\n" "Docs:" "screenerbot.io/docs" "Group:" "t.me/screenerbotio_talk"
    printf "   %15s %-30s %15s %-30s\n" "X:" "x.com/screenerbotio" "Support:" "t.me/screenerbotio_support"
    echo ""

    echo -e "${RESET}"
}

print_separator() {
    echo -e "${DIM}─────────────────────────────────────────────────────────────────────────────────${RESET}"
}

confirm() {
    local prompt="${1:-Are you sure?}"
    local default="${2:-n}"
    
    local yn_prompt
    if [ "$default" = "y" ]; then
        yn_prompt="[Y/n]"
    else
        yn_prompt="[y/N]"
    fi
    
    echo -en "${YELLOW}${ICON_WARN}${RESET} ${prompt} ${yn_prompt}: "
    read -r response < /dev/tty
    
    if [ -z "$response" ]; then
        response="$default"
    fi
    
    case "$response" in
        [yY][eE][sS]|[yY]) return 0 ;;
        *) return 1 ;;
    esac
}

press_enter() {
    echo ""
    echo -en "${DIM}Press Enter to continue...${RESET}"
    read -r < /dev/tty
}

# =============================================================================
# System Detection & Validation
# =============================================================================

detect_arch() {
    local arch
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)
            echo "x64"
            ;;
        aarch64|arm64)
            echo "arm64"
            ;;
        *)
            echo ""
            ;;
    esac
}

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo "$ID"
    elif [ -f /etc/debian_version ]; then
        echo "debian"
    elif [ -f /etc/redhat-release ]; then
        echo "rhel"
    else
        echo "unknown"
    fi
}

install_dependencies() {
    local pkgs=("$@")
    local pkg_manager=""
    local install_cmd=""
    
    if command -v apt-get &>/dev/null; then
        pkg_manager="apt-get"
        install_cmd="apt-get install -y"
        # Update package list first
        apt-get update -qq >/dev/null 2>&1 || true
    elif command -v dnf &>/dev/null; then
        pkg_manager="dnf"
        install_cmd="dnf install -y"
    elif command -v yum &>/dev/null; then
        pkg_manager="yum"
        install_cmd="yum install -y"
    elif command -v pacman &>/dev/null; then
        pkg_manager="pacman"
        install_cmd="pacman -S --noconfirm"
    else
        log_error "Unsupported package manager. Please install dependencies manually."
        return 1
    fi
    
    log_info "Using package manager: ${pkg_manager}"
    
    for pkg in "${pkgs[@]}"; do
        # Map command names to package names if needed
        local pkg_name="$pkg"
        case "$pkg" in
            "systemctl") pkg_name="systemd" ;;
            # Add other mappings if necessary
        esac
        
        log_info "Installing ${pkg_name}..."
        if $install_cmd "$pkg_name" >/dev/null 2>&1; then
            log_success "Installed ${pkg_name}"
        else
            log_error "Failed to install ${pkg_name}"
            return 1
        fi
    done
    
    return 0
}

get_glibc_version() {
    if command -v ldd &>/dev/null; then
        ldd --version 2>&1 | head -1 | grep -oE '[0-9]+\.[0-9]+' | head -1
    else
        echo "0.0"
    fi
}

check_requirements() {
    log_step "Checking System Requirements"
    
    local errors=0
    
    # Check if running as root or with sudo
    if [ "$EUID" -ne 0 ]; then
        log_warn "This script requires root privileges for installation"
        log_info "Please run with: sudo screenerbot"
        echo ""
        if ! confirm "Continue anyway? (some features may not work)"; then
            exit 1
        fi
    fi
    
    # Check architecture
    local arch
    arch=$(detect_arch)
    if [ -z "$arch" ]; then
        log_error "Unsupported architecture: $(uname -m)"
        log_info "ScreenerBot supports x86_64 (Intel/AMD) and aarch64 (ARM64)"
        errors=$((errors + 1))
    else
        log_success "Architecture: ${BOLD}$(uname -m)${RESET} (${arch})"
    fi
    
    # Check GLIBC version
    local glibc_version
    glibc_version=$(get_glibc_version)
    if [ -n "$glibc_version" ]; then
        local required_glibc="2.29"
        if printf '%s\n%s\n' "$required_glibc" "$glibc_version" | sort -V -C; then
            log_success "GLIBC version: ${BOLD}${glibc_version}${RESET} (≥${required_glibc} required)"
        else
            log_error "GLIBC version ${glibc_version} is too old (≥${required_glibc} required)"
            log_info "Please upgrade your system or use a newer distribution"
            errors=$((errors + 1))
        fi
    fi
    
    # Check available memory
    local total_mem_kb
    total_mem_kb=$(grep MemTotal /proc/meminfo | awk '{print $2}')
    local total_mem_gb=$((total_mem_kb / 1024 / 1024))
    if [ "$total_mem_gb" -lt 3 ]; then
        log_warn "RAM: ${BOLD}${total_mem_gb}GB${RESET} (4GB+ recommended)"
    else
        log_success "RAM: ${BOLD}${total_mem_gb}GB${RESET}"
    fi
    
    # Check CPU cores
    local cpu_cores
    cpu_cores=$(nproc)
    if [ "$cpu_cores" -lt 4 ]; then
        log_warn "CPU cores: ${BOLD}${cpu_cores}${RESET} (4+ recommended)"
    else
        log_success "CPU cores: ${BOLD}${cpu_cores}${RESET}"
    fi
    
    # Check disk space
    local free_space_gb
    free_space_gb=$(df -BG "${INSTALL_DIR%/*}" 2>/dev/null | tail -1 | awk '{print $4}' | tr -d 'G')
    if [ -n "$free_space_gb" ] && [ "$free_space_gb" -lt 5 ]; then
        log_warn "Free disk space: ${BOLD}${free_space_gb}GB${RESET} (5GB+ recommended)"
    elif [ -n "$free_space_gb" ]; then
        log_success "Free disk space: ${BOLD}${free_space_gb}GB${RESET}"
    fi
    
    # Check required commands
    local required_cmds=("curl" "tar" "systemctl" "jq")
    local missing_cmds=()
    
    for cmd in "${required_cmds[@]}"; do
        if command -v "$cmd" &>/dev/null; then
            log_success "Required command: ${BOLD}${cmd}${RESET}"
        else
            log_warn "Missing required command: ${BOLD}${cmd}${RESET}"
            missing_cmds+=("$cmd")
        fi
    done
    
    # Check optional commands
    local optional_cmds=()
    for cmd in "${optional_cmds[@]}"; do
        if command -v "$cmd" &>/dev/null; then
            log_success "Optional command: ${BOLD}${cmd}${RESET}"
        else
            log_info "Optional command not found: ${BOLD}${cmd}${RESET} (will attempt to install)"
            missing_cmds+=("$cmd")
        fi
    done
    
    # Attempt to install missing commands
    if [ ${#missing_cmds[@]} -gt 0 ]; then
        log_info "Attempting to install missing dependencies: ${missing_cmds[*]}"
        if install_dependencies "${missing_cmds[@]}"; then
            log_success "Dependencies installed successfully"
            # Re-check errors after installation
            errors=0
            for cmd in "${required_cmds[@]}"; do
                if ! command -v "$cmd" &>/dev/null; then
                    log_error "Failed to install required command: ${BOLD}${cmd}${RESET}"
                    errors=$((errors + 1))
                fi
            done
        else
            log_error "Failed to install dependencies automatically"
            errors=$((errors + 1))
        fi
    fi
    
    echo ""
    if [ $errors -gt 0 ]; then
        log_error "System check failed with $errors error(s)"
        return 1
    else
        log_success "All system requirements met!"
        return 0
    fi
}

# =============================================================================
# API Functions
# =============================================================================

# Fetch JSON from API with error handling (with spinner)
api_fetch() {
    local endpoint="$1"
    local url="${API_BASE}${endpoint}"
    local response
    local temp_file
    temp_file=$(mktemp)
    
    # Run curl in background with spinner
    curl -fsSL --connect-timeout 10 --max-time 30 "$url" > "$temp_file" &
    local curl_pid=$!
    spinner "$curl_pid" "Connecting to API..."
    wait "$curl_pid"
    local exit_code=$?
    
    if [ $exit_code -ne 0 ]; then
        rm -f "$temp_file"
        return 1
    fi
    
    response=$(cat "$temp_file")
    rm -f "$temp_file"
    echo "$response"
}

# Parse JSON (with jq or fallback)
# Usage: json_get "$json" "field_name" (without . prefix)
json_get() {
    local json="$1"
    local key="$2"
    
    if command -v jq &>/dev/null; then
        # Add . prefix for jq path syntax
        echo "$json" | jq -r ".$key" 2>/dev/null
    else
        # Fallback: extract simple key values from JSON
        # Handle both "key": "value" and "key": value (numbers/booleans)
        # First try quoted value, then unquoted
        local result
        result=$(echo "$json" | sed -n "s/.*\"${key}\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" | head -1)
        if [ -z "$result" ]; then
            # Try unquoted value (numbers, booleans)
            result=$(echo "$json" | sed -n "s/.*\"${key}\"[[:space:]]*:[[:space:]]*\([^,}[:space:]]*\).*/\1/p" | head -1)
        fi
        echo "$result"
    fi
}

# Get latest release info
get_latest_release() {
    local response
    if ! response=$(api_fetch "/releases/latest"); then
        return 1
    fi
    
    if command -v jq &>/dev/null; then
        local success
        success=$(echo "$response" | jq -r '.success' 2>/dev/null)
        # Trim whitespace
        success=$(echo "$success" | tr -d '[:space:]')
        
        if [ "$success" != "true" ]; then
            log_error "API returned error. Response:"
            echo "$response" | head -n 5 >&2
            return 1
        fi
    else
        # Fallback check if jq fails or is missing
        if ! echo "$response" | grep -q '"success"\s*:\s*true'; then
             log_error "API returned error (fallback check). Response:"
             echo "$response" | head -n 5 >&2
             return 1
        fi
    fi
    
    echo "$response"
}

# Check for updates
check_update_available() {
    local current_version="$1"
    local platform="$2"
    
    local response
    if ! response=$(api_fetch "/releases/check?version=${current_version}&platform=${platform}"); then
        return 1
    fi
    
    echo "$response"
}

# Get download URL for specific platform
get_download_url() {
    local version="$1"
    local platform="$2"
    
    echo "${API_BASE}/releases/download?version=${version}&platform=${platform}&mode=update"
}

# Get remote script version from GitHub
get_remote_script_version() {
    local remote_version
    remote_version=$(curl -fsSL --connect-timeout 5 "${GITHUB_RAW}/screenerbot.sh" 2>/dev/null | \
        grep -m1 'SCRIPT_VERSION=' | sed 's/.*SCRIPT_VERSION="\([^"]*\)".*/\1/')
    echo "$remote_version"
}

# Check if script update is available (silent check)
check_script_update() {
    local remote_version
    remote_version=$(get_remote_script_version)
    
    if [ -z "$remote_version" ]; then
        return 1  # Couldn't check
    fi
    
    if [ "$remote_version" != "$SCRIPT_VERSION" ]; then
        echo "$remote_version"
        return 0  # Update available
    fi
    
    return 1  # No update
}

# Auto-update check on startup (silent, non-blocking)
auto_check_script_update() {
    # Skip if running non-interactively or with arguments
    if [ -n "${1:-}" ] || [ ! -t 0 ]; then
        return
    fi
    
    local remote_version
    remote_version=$(check_script_update)
    
    if [ -n "$remote_version" ]; then
        echo ""
        echo -e "${YELLOW}${ICON_WARN} Management script update available: v${SCRIPT_VERSION} → v${remote_version}${RESET}"
        echo -e "${DIM}   Run option [12] to update, or use: curl -fsSL https://screenerbot.io/install.sh | bash${RESET}"
        echo ""
        sleep 2
    fi
}

# Install manager script to system
install_manager_script() {
    log_step "Installing Management Script"
    
    local current_script
    current_script=$(readlink -f "$0")
    
    # Skip if already installed at the correct location
    if [ "$current_script" = "$MANAGER_PATH" ]; then
        log_success "Manager already installed at ${MANAGER_PATH}"
        return 0
    fi
    
    log_info "Installing to ${MANAGER_PATH}..."
    
    # Download fresh copy to ensure we have latest
    local temp_script
    temp_script=$(mktemp)
    
    if curl -fsSL "${GITHUB_RAW}/screenerbot.sh" -o "$temp_script"; then
        if head -n 1 "$temp_script" | grep -q "^#!/bin/bash"; then
            chmod +x "$temp_script"
            mv "$temp_script" "$MANAGER_PATH"
            log_success "Manager installed: ${MANAGER_PATH}"
            log_info "You can now run: ${BOLD}screenerbot-manager${RESET}"
            return 0
        fi
    fi
    
    rm -f "$temp_script"
    log_error "Failed to install manager script"
    return 1
}

# Self-update function
self_update() {
    log_step "Updating Management Script"
    
    local current_script
    current_script=$(readlink -f "$0")
    local temp_script
    temp_script=$(mktemp)
    
    log_info "Downloading latest script..."
    if curl -fsSL "${GITHUB_RAW}/screenerbot.sh" -o "$temp_script"; then
        # Check if file is valid bash script
        if ! head -n 1 "$temp_script" | grep -q "^#!/bin/bash"; then
            log_error "Downloaded file is not a valid script"
            rm -f "$temp_script"
            return 1
        fi
        
        # Get version from downloaded script
        local new_version
        new_version=$(grep -m1 'SCRIPT_VERSION=' "$temp_script" | sed 's/.*SCRIPT_VERSION="\([^"]*\)".*/\1/')
        
        # Compare versions
        if [ "$new_version" = "$SCRIPT_VERSION" ]; then
            log_success "Script is already up to date (v${SCRIPT_VERSION})"
            rm -f "$temp_script"
            return 0
        fi
        
        log_info "Updating: v${SCRIPT_VERSION} → v${new_version}"
        
        # Update current script
        chmod +x "$temp_script"
        mv "$temp_script" "$current_script"
        
        # Also update manager path if different
        if [ "$current_script" != "$MANAGER_PATH" ] && [ -f "$MANAGER_PATH" ]; then
            curl -fsSL "${GITHUB_RAW}/screenerbot.sh" -o "$MANAGER_PATH" 2>/dev/null
            chmod +x "$MANAGER_PATH" 2>/dev/null
        fi
        
        log_success "Script updated to v${new_version}!"
        echo ""
        log_info "Restarting script..."
        sleep 1
        exec "$current_script" "$@"
    else
        log_error "Failed to download update"
        rm -f "$temp_script"
        return 1
    fi
}

# =============================================================================
# Version Management
# =============================================================================

get_installed_version() {
    if [ -x "${SYMLINK_PATH}" ] || [ -x "${INSTALL_DIR}/screenerbot" ]; then
        local binary="${SYMLINK_PATH}"
        if [ ! -x "$binary" ]; then
            binary="${INSTALL_DIR}/screenerbot"
        fi
        "$binary" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1
    else
        echo ""
    fi
}

compare_versions() {
    local v1="$1"
    local v2="$2"
    
    # Returns: 0 if v1 == v2, 1 if v1 > v2, 2 if v1 < v2
    if [ "$v1" = "$v2" ]; then
        return 0
    fi
    
    local IFS='.'
    read -ra v1_parts <<< "$v1"
    read -ra v2_parts <<< "$v2"
    
    for i in 0 1 2; do
        local p1="${v1_parts[$i]:-0}"
        local p2="${v2_parts[$i]:-0}"
        
        if [ "$p1" -gt "$p2" ]; then
            return 1
        elif [ "$p1" -lt "$p2" ]; then
            return 2
        fi
    done
    
    return 0
}

# =============================================================================
# Installation Functions
# =============================================================================

download_and_install() {
    local version="$1"
    local arch
    arch=$(detect_arch)
    
    if [ -z "$arch" ]; then
        log_error "Could not detect system architecture"
        return 1
    fi
    
    local platform="linux-${arch}-headless"
    
    log_step "Installing ScreenerBot v${version}"
    
    log_info "Platform: ${BOLD}${platform}${RESET}"
    log_info "Target directory: ${BOLD}${INSTALL_DIR}${RESET}"
    
    # Create install directory
    if ! mkdir -p "${INSTALL_DIR}"; then
        log_error "Failed to create installation directory"
        return 1
    fi
    
    # Create temp directory for download
    local temp_dir
    temp_dir=$(mktemp -d)
    trap "rm -rf '${temp_dir}'" EXIT
    
    local download_url
    download_url=$(get_download_url "$version" "$platform")
    local tarball="${temp_dir}/screenerbot.tar.gz"
    
    log_info "Downloading from: ${DIM}${download_url}${RESET}"
    
    # Download with progress
    echo ""
    if ! curl -fSL --connect-timeout 30 --max-time 300 \
        --progress-bar \
        -o "$tarball" \
        "$download_url"; then
        log_error "Download failed"
        return 1
    fi
    echo ""
    
    log_success "Download complete"
    
    # Verify tarball
    if [ ! -f "$tarball" ] || [ ! -s "$tarball" ]; then
        log_error "Downloaded file is empty or missing"
        return 1
    fi
    
    local file_size
    file_size=$(du -h "$tarball" | cut -f1)
    log_info "Downloaded: ${BOLD}${file_size}${RESET}"
    
    # Backup existing installation
    if [ -x "${INSTALL_DIR}/screenerbot" ]; then
        local old_version
        old_version=$(get_installed_version)
        log_info "Backing up existing installation (v${old_version})..."
        cp "${INSTALL_DIR}/screenerbot" "${INSTALL_DIR}/screenerbot.backup.${old_version}" 2>/dev/null || true
    fi
    
    # Extract (suppress Docker xattr warnings)
    log_info "Extracting..."
    local tar_output
    tar_output=$(tar -xzf "$tarball" -C "${INSTALL_DIR}" 2>&1) || {
        # Check if tar actually failed (not just warning)
        if ! tar -tzf "$tarball" >/dev/null 2>&1; then
            log_error "Failed to extract tarball"
            return 1
        fi
    }
    # Filter out Docker xattr warnings for display if any
    if [ -n "$tar_output" ]; then
        echo "$tar_output" | grep -v "LIBARCHIVE.xattr" || true
    fi
    
    # Make executable
    chmod +x "${INSTALL_DIR}/screenerbot"
    
    # Create symlink
    if [ ! -L "${SYMLINK_PATH}" ] || [ "$(readlink -f "${SYMLINK_PATH}")" != "${INSTALL_DIR}/screenerbot" ]; then
        log_info "Creating symlink: ${SYMLINK_PATH} -> ${INSTALL_DIR}/screenerbot"
        ln -sf "${INSTALL_DIR}/screenerbot" "${SYMLINK_PATH}"
    fi
    
    # Verify installation
    local installed_version
    installed_version=$(get_installed_version)
    if [ -z "$installed_version" ]; then
        log_error "Installation verification failed"
        return 1
    fi
    
    log_success "ScreenerBot v${installed_version} installed successfully!"
    
    # Install manager script if not already installed
    if [ ! -x "$MANAGER_PATH" ]; then
        echo ""
        log_info "Installing management script..."
        install_manager_script
    fi
    
    return 0
}

uninstall() {
    log_step "Uninstalling ScreenerBot"
    
    # Stop service if running
    if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
        log_info "Stopping service..."
        systemctl stop "${SERVICE_NAME}"
    fi
    
    # Disable service
    if systemctl is-enabled --quiet "${SERVICE_NAME}" 2>/dev/null; then
        log_info "Disabling service..."
        systemctl disable "${SERVICE_NAME}"
    fi
    
    # Remove service files
    if [ -f "${SERVICE_FILE}" ]; then
        log_info "Removing service file..."
        rm -f "${SERVICE_FILE}"
    fi
    
    if [ -f "${UPDATE_TIMER_FILE}" ]; then
        rm -f "${UPDATE_TIMER_FILE}"
    fi
    
    if [ -f "${UPDATE_SERVICE_FILE}" ]; then
        rm -f "${UPDATE_SERVICE_FILE}"
    fi
    
    systemctl daemon-reload 2>/dev/null || true
    
    # Remove symlink
    if [ -L "${SYMLINK_PATH}" ]; then
        log_info "Removing symlink..."
        rm -f "${SYMLINK_PATH}"
    fi
    
    # Remove installation directory
    if [ -d "${INSTALL_DIR}" ]; then
        log_info "Removing installation directory..."
        rm -rf "${INSTALL_DIR}"
    fi
    
    log_success "ScreenerBot uninstalled successfully!"
    
    # Ask about data directory
    local data_dir
    data_dir=$(get_data_dir)
    if [ -d "$data_dir" ]; then
        echo ""
        log_warn "Data directory still exists: ${data_dir}"
        if confirm "Remove data directory? (This will delete all configs and databases)"; then
            rm -rf "$data_dir"
            log_success "Data directory removed"
        else
            log_info "Data directory preserved at: ${data_dir}"
        fi
    fi
}

# =============================================================================
# Backup & Restore Functions
# =============================================================================

create_backup() {
    local data_dir
    data_dir=$(get_data_dir)
    
    if [ ! -d "$data_dir" ]; then
        log_error "Data directory not found: $data_dir"
        return 1
    fi
    
    log_step "Create Backup"
    
    # Get the actual user's home directory (not root if using sudo)
    local user_home
    if [ -n "$SUDO_USER" ]; then
        user_home=$(getent passwd "$SUDO_USER" | cut -d: -f6)
    else
        user_home="$HOME"
    fi
    
    # Show what will be backed up
    local data_size
    data_size=$(du -sh "$data_dir" 2>/dev/null | cut -f1)
    local file_count
    file_count=$(find "$data_dir" -type f 2>/dev/null | wc -l | tr -d ' ')
    
    echo ""
    echo "  ${BOLD}Source Data:${RESET}"
    echo "    Directory: ${data_dir}"
    echo "    Size: ${data_size}"
    echo "    Files: ${file_count}"
    echo ""
    
    local backup_name="screenerbot-backup-$(date +%Y%m%d-%H%M%S).tar.gz"
    local backup_path="${user_home}/${backup_name}"
    
    echo "  ${BOLD}Backup will be saved to:${RESET}"
    echo "    ${backup_path}"
    echo ""
    
    if ! confirm "Create backup now?"; then
        log_info "Backup cancelled"
        return 0
    fi
    
    echo ""
    log_info "Creating backup: ${backup_name}"
    
    # Create backup with progress indicator
    if tar -czf "$backup_path" -C "$(dirname "$data_dir")" "$(basename "$data_dir")"; then
        local backup_size
        backup_size=$(du -h "$backup_path" | cut -f1)
        
        # Verify the backup
        if tar -tzf "$backup_path" >/dev/null 2>&1; then
            log_success "Backup created and verified!"
            echo ""
            echo "  ${BOLD}Backup Details:${RESET}"
            echo "    File: ${backup_path}"
            echo "    Size: ${backup_size}"
            echo ""
            
            # Fix ownership if created as root
            if [ -n "$SUDO_USER" ]; then
                chown "$SUDO_USER:$SUDO_USER" "$backup_path" 2>/dev/null || true
            fi
        else
            log_error "Backup created but verification failed!"
            return 1
        fi
    else
        log_error "Failed to create backup"
        return 1
    fi
    
    return 0
}

restore_backup() {
    log_step "Restore Backup"
    
    # Get the actual user's home directory (not root if using sudo)
    local user_home
    if [ -n "$SUDO_USER" ]; then
        user_home=$(getent passwd "$SUDO_USER" | cut -d: -f6)
    else
        user_home="$HOME"
    fi
    
    # List available backups (include pre-restore backups too)
    local backups=()
    while IFS= read -r -d '' file; do
        backups+=("$file")
    done < <(find "${user_home}" -maxdepth 1 \( -name "screenerbot-backup-*.tar.gz" -o -name "screenerbot-pre-restore-*.tar.gz" \) -print0 2>/dev/null | sort -rz)
    
    local backup_path=""
    
    if [ ${#backups[@]} -eq 0 ]; then
        log_warn "No backup files found in ${user_home}"
        echo ""
        echo -n "Enter path to backup file (or Q to cancel): "
        read -r backup_path < /dev/tty
        
        # Handle quit/cancel
        if [[ "$backup_path" =~ ^[qQcC]$ ]] || [ -z "$backup_path" ]; then
            log_info "Restore cancelled"
            return 0
        fi
        
        if [ ! -f "$backup_path" ]; then
            log_error "File not found: $backup_path"
            return 1
        fi
    else
        echo ""
        echo "Available backups:"
        echo ""
        local i=1
        for backup in "${backups[@]}"; do
            local size
            size=$(du -h "$backup" | cut -f1)
            local name
            name=$(basename "$backup")
            # Extract date from filename for better display
            local date_part=""
            if [[ "$name" =~ ([0-9]{8})-([0-9]{6}) ]]; then
                local d="${BASH_REMATCH[1]}"
                local t="${BASH_REMATCH[2]}"
                date_part="${d:0:4}-${d:4:2}-${d:6:2} ${t:0:2}:${t:2:2}"
            fi
            # Mark pre-restore backups
            local label=""
            if [[ "$name" == *"pre-restore"* ]]; then
                label=" ${YELLOW}(pre-restore)${RESET}"
            fi
            echo "  ${CYAN}[$i]${RESET} $name ${DIM}($size, $date_part)${RESET}${label}"
            ((i++))
        done
        echo "  ${CYAN}[Q]${RESET} Cancel"
        echo ""
        echo -n "Select backup [1-${#backups[@]}] or Q to cancel: "
        read -r selection < /dev/tty
        
        # Handle quit/cancel
        if [[ "$selection" =~ ^[qQcC]$ ]] || [ -z "$selection" ]; then
            log_info "Restore cancelled"
            return 0
        fi
        
        # Validate number
        if ! [[ "$selection" =~ ^[0-9]+$ ]] || [ "$selection" -lt 1 ] || [ "$selection" -gt ${#backups[@]} ]; then
            log_error "Invalid selection"
            return 1
        fi
        
        backup_path="${backups[$((selection-1))]}"
    fi
    
    # Verify backup integrity
    log_info "Verifying backup integrity..."
    if ! tar -tzf "$backup_path" >/dev/null 2>&1; then
        log_error "Backup file is corrupted or invalid"
        return 1
    fi
    
    # Show backup summary
    local file_count
    file_count=$(tar -tzf "$backup_path" 2>/dev/null | wc -l | tr -d ' ')
    local backup_size
    backup_size=$(du -h "$backup_path" | cut -f1)
    echo ""
    echo "  ${BOLD}Backup Summary:${RESET}"
    echo "    File: $(basename "$backup_path")"
    echo "    Size: ${backup_size}"
    echo "    Files: ${file_count} entries"
    echo ""
    
    local data_dir
    data_dir=$(get_data_dir)
    
    # Check if service is running (don't stop yet)
    local service_was_running=false
    if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
        service_was_running=true
        log_warn "Service is currently running and will be stopped"
    fi
    
    # Backup current data if exists
    if [ -d "$data_dir" ]; then
        log_warn "Current data directory will be replaced"
        echo ""
        echo "  ${BOLD}Target:${RESET} ${data_dir}"
        echo ""
        if ! confirm "Continue with restore? This will replace your current data"; then
            log_info "Restore cancelled"
            return 0
        fi
        
        # Now stop the service
        if [ "$service_was_running" = true ]; then
            log_info "Stopping service before restore..."
            systemctl stop "${SERVICE_NAME}"
        fi
        
        local current_backup="${user_home}/screenerbot-pre-restore-$(date +%Y%m%d-%H%M%S).tar.gz"
        log_info "Backing up current data to: $(basename "$current_backup")"
        if tar -czf "$current_backup" -C "$(dirname "$data_dir")" "$(basename "$data_dir")"; then
            # Fix ownership if created as root
            if [ -n "$SUDO_USER" ]; then
                chown "$SUDO_USER:$SUDO_USER" "$current_backup" 2>/dev/null || true
            fi
            log_success "Current data backed up"
        else
            log_error "Failed to backup current data"
            # Restart service if it was running
            if [ "$service_was_running" = true ]; then
                systemctl start "${SERVICE_NAME}" 2>/dev/null || true
            fi
            return 1
        fi
        rm -rf "$data_dir"
    else
        echo ""
        echo "  ${BOLD}Target:${RESET} ${data_dir}"
        echo ""
        if ! confirm "Restore backup to this location?"; then
            log_info "Restore cancelled"
            return 0
        fi
    fi
    
    # Restore (suppress Docker xattr warnings)
    log_info "Restoring from backup..."
    mkdir -p "$(dirname "$data_dir")"
    
    local tar_output
    if tar_output=$(tar -xzf "$backup_path" -C "$(dirname "$data_dir")" 2>&1); then
        # Filter out Docker xattr warnings for display if any
        if [ -n "$tar_output" ]; then
            echo "$tar_output" | grep -v "LIBARCHIVE.xattr" || true
        fi
        log_success "Backup restored successfully!"
        echo ""
        
        # Offer to restart service
        if systemctl is-enabled --quiet "${SERVICE_NAME}" 2>/dev/null; then
            if confirm "Start ScreenerBot service now?"; then
                if systemctl start "${SERVICE_NAME}"; then
                    log_success "Service started"
                else
                    log_error "Failed to start service"
                fi
            fi
        elif [ "$service_was_running" = true ]; then
            if confirm "Service was running before. Start it now?"; then
                if systemctl start "${SERVICE_NAME}"; then
                    log_success "Service started"
                else
                    log_error "Failed to start service"
                fi
            fi
        fi
    else
        log_error "Failed to restore backup"
        return 1
    fi
    
    return 0
}

# =============================================================================
# Service Management Functions
# =============================================================================

create_service() {
    log_step "Creating Systemd Service"
    
    local user="${SUDO_USER:-$USER}"
    local group="${SUDO_USER:-$USER}"
    local home_dir
    if [ -n "$SUDO_USER" ]; then
        home_dir=$(getent passwd "$SUDO_USER" | cut -d: -f6)
    else
        home_dir="$HOME"
    fi
    
    log_info "Service will run as user: ${BOLD}${user}${RESET}"
    log_info "Working directory: ${BOLD}${home_dir}${RESET}"
    
    if [ "$user" = "root" ]; then
        echo ""
        log_warn "Service will run as ROOT user!"
        log_warn "This is generally not recommended for security reasons."
        if ! confirm "Are you sure you want to continue running as root?"; then
            log_info "Service creation cancelled"
            return 1
        fi
    fi
    
    cat > "${SERVICE_FILE}" << EOF
[Unit]
Description=ScreenerBot - Automated Solana Trading Bot
Documentation=https://screenerbot.io/docs
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${user}
Group=${group}
WorkingDirectory=${home_dir}
ExecStart=${SYMLINK_PATH}
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true

# Environment
Environment="HOME=${home_dir}"
Environment="USER=${user}"

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    
    log_success "Service file created: ${SERVICE_FILE}"
    
    echo ""
    if confirm "Enable service to start on boot?" "y"; then
        systemctl enable "${SERVICE_NAME}"
        log_success "Service enabled for auto-start"
    fi
    
    if confirm "Start service now?" "y"; then
        systemctl start "${SERVICE_NAME}"
        sleep 2
        if systemctl is-active --quiet "${SERVICE_NAME}"; then
            log_success "Service started successfully!"
            echo ""
            log_info "Dashboard available at: ${CYAN}http://localhost:8080${RESET}"
            log_info "For remote access, use SSH tunnel:"
            echo ""
            echo "  ${DIM}ssh -L 8080:localhost:8080 ${user}@your-server-ip${RESET}"
        else
            log_error "Service failed to start"
            echo ""
            log_info "Check logs with: ${CYAN}journalctl -u ${SERVICE_NAME} -f${RESET}"
        fi
    fi
}

service_status() {
    echo ""
    echo "${BOLD}Service Status:${RESET}"
    echo ""
    
    if ! systemctl list-unit-files | grep -q "${SERVICE_NAME}"; then
        echo "  ${DIM}Service not installed${RESET}"
        return
    fi
    
    local status
    status=$(systemctl is-active "${SERVICE_NAME}" 2>/dev/null || echo "inactive")
    local enabled
    enabled=$(systemctl is-enabled "${SERVICE_NAME}" 2>/dev/null || echo "disabled")
    
    local status_color
    case "$status" in
        active)
            status_color="${GREEN}"
            ;;
        inactive)
            status_color="${YELLOW}"
            ;;
        failed)
            status_color="${RED}"
            ;;
        *)
            status_color="${DIM}"
            ;;
    esac
    
    printf "  %-13s %b\n" "Status:" "${status_color}${BOLD}${status}${RESET}"
    printf "  %-13s %s\n" "Auto-start:" "${enabled}"
    
    if [ "$status" = "active" ]; then
        local pid
        pid=$(systemctl show "${SERVICE_NAME}" --property=MainPID --value)
        local uptime
        uptime=$(systemctl show "${SERVICE_NAME}" --property=ActiveEnterTimestamp --value)
        local mem
        mem=$(ps -o rss= -p "$pid" 2>/dev/null | awk '{print int($1/1024)"MB"}')
        
        printf "  %-13s %s\n" "PID:" "${pid}"
        printf "  %-13s %s\n" "Memory:" "${mem:-unknown}"
        printf "  %-13s %s\n" "Started:" "${uptime:-unknown}"
    fi
    echo ""
}

service_menu() {
    while true; do
        print_banner
        echo "${BOLD}  ${ICON_SERVICE}  Service Management${RESET}"
        echo ""
        
        service_status
        
        local options=(
            "${ICON_START} Start Service"
            "${ICON_STOP} Stop Service"
            "${ICON_RESTART} Restart Service"
            "${ICON_LOGS} View Logs"
            "${GREEN}+${RESET} Enable Auto-Start"
            "${RED}-${RESET} Disable Auto-Start"
            "${ICON_SERVICE} Create/Recreate Service"
            "${ICON_BACK} Back to Main Menu"
        )
        
        select_menu "${options[@]}"
        local choice=$MENU_RESULT
        
        case "$choice" in
            0)
                if systemctl start "${SERVICE_NAME}" 2>/dev/null; then
                    log_success "Service started"
                else
                    log_error "Failed to start service"
                fi
                press_enter
                ;;
            1)
                if systemctl stop "${SERVICE_NAME}" 2>/dev/null; then
                    log_success "Service stopped"
                else
                    log_error "Failed to stop service"
                fi
                press_enter
                ;;
            2)
                if systemctl restart "${SERVICE_NAME}" 2>/dev/null; then
                    log_success "Service restarted"
                else
                    log_error "Failed to restart service"
                fi
                press_enter
                ;;
            3)
                echo ""
                log_info "Showing last 50 log lines (Ctrl+C to exit live view)..."
                echo ""
                journalctl -u "${SERVICE_NAME}" -n 50 -f 2>/dev/null || log_error "Failed to get logs"
                ;;
            4)
                if systemctl enable "${SERVICE_NAME}" 2>/dev/null; then
                    log_success "Auto-start enabled"
                else
                    log_error "Failed to enable auto-start"
                fi
                press_enter
                ;;
            5)
                if systemctl disable "${SERVICE_NAME}" 2>/dev/null; then
                    log_success "Auto-start disabled"
                else
                    log_error "Failed to disable auto-start"
                fi
                press_enter
                ;;
            6)
                create_service
                press_enter
                ;;
            7|-1)
                break
                ;;
        esac
    done
}

# =============================================================================
# Telegram Notification Functions
# =============================================================================

get_telegram_config() {
    local data_dir
    data_dir=$(get_data_dir)
    local config_file="${data_dir}/data/config.toml"
    
    if [ ! -f "$config_file" ]; then
        echo ""
        return 1
    fi
    
    local bot_token
    local chat_id
    
    # Parse TOML for telegram settings
    bot_token=$(grep -A 20 '^\[telegram\]' "$config_file" 2>/dev/null | grep '^bot_token' | head -1 | sed 's/.*= *"\([^"]*\)".*/\1/')
    chat_id=$(grep -A 20 '^\[telegram\]' "$config_file" 2>/dev/null | grep '^chat_id' | head -1 | sed 's/.*= *"\([^"]*\)".*/\1/')
    
    if [ -n "$bot_token" ] && [ -n "$chat_id" ]; then
        echo "${bot_token}:${chat_id}"
        return 0
    fi
    
    echo ""
    return 1
}

send_telegram_message() {
    local message="$1"
    local config
    config=$(get_telegram_config)
    
    if [ -z "$config" ]; then
        return 1
    fi
    
    local bot_token="${config%%:*}"
    local chat_id="${config#*:}"
    
    curl -fsSL -X POST \
        "https://api.telegram.org/bot${bot_token}/sendMessage" \
        -d "chat_id=${chat_id}" \
        -d "text=${message}" \
        -d "parse_mode=HTML" \
        &>/dev/null
}

setup_update_notifications() {
    log_step "Setup Auto-Update Notifications"
    
    local config
    config=$(get_telegram_config)
    
    if [ -z "$config" ]; then
        log_warn "Telegram not configured in ScreenerBot"
        log_info "Please configure Telegram in the ScreenerBot dashboard first:"
        log_info "  Settings → Telegram → Configure bot token and chat ID"
        press_enter
        return 1
    fi
    
    log_success "Telegram configuration found"
    
    # Test notification
    if confirm "Send test notification?"; then
        if send_telegram_message "🤖 <b>ScreenerBot VPS Manager</b>%0A%0ATest notification from your VPS! Auto-update notifications are working."; then
            log_success "Test message sent!"
        else
            log_error "Failed to send test message"
            return 1
        fi
    fi
    
    # Create update check service
    log_info "Creating update check timer..."
    
    local arch
    arch=$(detect_arch)
    local platform="linux-${arch}-headless"
    
    # Resolve home directory for the actual user (not root if using sudo)
    local user="${SUDO_USER:-$USER}"
    local user_home
    if [ -n "$SUDO_USER" ]; then
        user_home=$(getent passwd "$SUDO_USER" | cut -d: -f6)
    else
        user_home="$HOME"
    fi
    local config_path="${user_home}/.local/share/ScreenerBot/data/config.toml"
    
    # Use non-quoted heredoc to allow variable substitution for config path
    cat > "${UPDATE_SERVICE_FILE}" << EOF
[Unit]
Description=ScreenerBot Update Checker
After=network-online.target

[Service]
Type=oneshot
ExecStart=/bin/bash -c '\\
    CURRENT=\$(/usr/local/bin/screenerbot --version 2>/dev/null | grep -oE "[0-9]+\\\\.[0-9]+\\\\.[0-9]+" | head -1); \\
    if [ -z "\$CURRENT" ]; then exit 0; fi; \\
    ARCH=\$(uname -m | sed "s/x86_64/x64/;s/aarch64/arm64/;s/amd64/x64/"); \\
    RESPONSE=\$(curl -fsSL "https://screenerbot.io/api/releases/check?version=\${CURRENT}&platform=linux-\${ARCH}-headless" 2>/dev/null); \\
    if echo "\$RESPONSE" | grep -q "updateAvailable.*true"; then \\
        LATEST=\$(echo "\$RESPONSE" | sed -n "s/.*\\\\\"latestVersion\\\\\"[[:space:]]*:[[:space:]]*\\\\\"\\\\([^\\\\\",]*\\\\)\\\\\".*/\\\\1/p"); \\
        CONFIG_FILE="${config_path}"; \\
        if [ -f "\$CONFIG_FILE" ]; then \\
            BOT_TOKEN=\$(grep -A 20 "^\\\\[telegram\\\\]" "\$CONFIG_FILE" | grep "^bot_token" | head -1 | sed "s/.*= *\\"\\\\([^\\"]*\\\\)\\".*/\\\\1/"); \\
            CHAT_ID=\$(grep -A 20 "^\\\\[telegram\\\\]" "\$CONFIG_FILE" | grep "^chat_id" | head -1 | sed "s/.*= *\\"\\\\([^\\"]*\\\\)\\".*/\\\\1/"); \\
            if [ -n "\$BOT_TOKEN" ] && [ -n "\$CHAT_ID" ]; then \\
                MSG="[UPDATE] <b>ScreenerBot Update Available</b>%0A%0ACurrent: v\${CURRENT}%0ALatest: v\${LATEST}%0A%0ARun: <code>sudo screenerbot</code> to update"; \\
                curl -fsSL -X POST "https://api.telegram.org/bot\${BOT_TOKEN}/sendMessage" -d "chat_id=\${CHAT_ID}" -d "text=\${MSG}" -d "parse_mode=HTML" &>/dev/null; \\
            fi; \\
        fi; \\
    fi'
EOF

    cat > "${UPDATE_TIMER_FILE}" << EOF
[Unit]
Description=ScreenerBot Update Checker Timer

[Timer]
OnBootSec=5min
OnUnitActiveSec=6h
Persistent=true

[Install]
WantedBy=timers.target
EOF

    systemctl daemon-reload
    systemctl enable "${SERVICE_NAME}-update.timer"
    systemctl start "${SERVICE_NAME}-update.timer"
    
    log_success "Update notifications configured!"
    log_info "Checks for updates every 6 hours"
    log_info "Sends Telegram notification when update available"
    
    press_enter
}

# =============================================================================
# Dashboard Security Functions
# =============================================================================

get_auth_status() {
    # Check if service is running and get auth status from API
    if ! systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
        echo ""
        return 1
    fi
    
    local response
    response=$(curl -fsSL "http://127.0.0.1:8080/api/auth/status" 2>/dev/null)
    
    if [ -n "$response" ]; then
        echo "$response"
        return 0
    fi
    
    echo ""
    return 1
}

set_dashboard_password() {
    log_step "Set Dashboard Password"
    
    # Check if service is running
    if ! systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
        log_error "ScreenerBot service is not running"
        log_info "Start the service first: ${CYAN}systemctl start ${SERVICE_NAME}${RESET}"
        press_enter
        return 1
    fi
    
    # Get current auth status
    local auth_status
    auth_status=$(get_auth_status)
    
    if [ -z "$auth_status" ]; then
        log_error "Could not connect to ScreenerBot dashboard"
        log_info "Make sure the service is running and healthy"
        press_enter
        return 1
    fi
    
    local has_password
    has_password=$(json_get "$auth_status" "has_password")
    local auth_enabled
    auth_enabled=$(json_get "$auth_status" "auth_enabled")
    local totp_enabled
    totp_enabled=$(json_get "$auth_status" "totp_enabled")
    
    echo ""
    echo "  ${BOLD}Current Status:${RESET}"
    if [ "$auth_enabled" = "true" ]; then
        echo "    Authentication: ${GREEN}Enabled${RESET}"
    else
        echo "    Authentication: ${YELLOW}Disabled${RESET}"
    fi
    if [ "$has_password" = "true" ]; then
        echo "    Password: ${GREEN}Set${RESET}"
    else
        echo "    Password: ${DIM}Not set${RESET}"
    fi
    if [ "$totp_enabled" = "true" ]; then
        echo "    2FA (TOTP): ${GREEN}Enabled${RESET}"
    else
        echo "    2FA (TOTP): ${DIM}Not enabled${RESET}"
    fi
    echo ""
    
    # Ask for current password if one exists
    local current_password=""
    if [ "$has_password" = "true" ]; then
        echo -n "Enter current password (or Q to cancel): "
        read -rs current_password < /dev/tty
        echo ""
        
        if [[ "$current_password" =~ ^[qQcC]$ ]]; then
            log_info "Cancelled"
            return 0
        fi
    fi
    
    # Ask for new password
    echo -n "Enter new password (min 4 chars, empty to disable): "
    read -rs new_password < /dev/tty
    echo ""
    
    if [ -n "$new_password" ]; then
        echo -n "Confirm new password: "
        read -rs confirm_password < /dev/tty
        echo ""
        
        if [ "$new_password" != "$confirm_password" ]; then
            log_error "Passwords do not match"
            press_enter
            return 1
        fi
        
        if [ ${#new_password} -lt 4 ]; then
            log_error "Password must be at least 4 characters"
            press_enter
            return 1
        fi
    fi
    
    # Build JSON payload
    local json_payload
    if [ "$has_password" = "true" ] && [ -n "$current_password" ]; then
        json_payload="{\"current_password\":\"$current_password\",\"new_password\":\"$new_password\"}"
    else
        json_payload="{\"new_password\":\"$new_password\"}"
    fi
    
    # Send request
    log_info "Updating password..."
    local response
    response=$(curl -fsSL -X POST "http://127.0.0.1:8080/api/auth/set-password" \
        -H "Content-Type: application/json" \
        -d "$json_payload" 2>/dev/null)
    
    if echo "$response" | grep -q '"success":\s*true'; then
        if [ -z "$new_password" ]; then
            log_success "Password cleared and authentication disabled"
        else
            log_success "Password set successfully!"
            echo ""
            log_info "Dashboard authentication is now ${GREEN}enabled${RESET}"
            log_info "You will need this password to access the dashboard"
        fi
    else
        local error
        error=$(json_get "$response" "message")
        log_error "Failed to set password: ${error:-Unknown error}"
    fi
    
    press_enter
}

manage_dashboard_security() {
    while true; do
        print_banner
        echo "${BOLD}  ${ICON_LOCK}  Dashboard Security${RESET}"
        echo ""
        print_separator
        
        # Get auth status
        local auth_status
        auth_status=$(get_auth_status)
        
        if [ -z "$auth_status" ]; then
            echo ""
            log_warn "Cannot connect to ScreenerBot dashboard"
            log_info "Make sure the service is running"
            echo ""
            echo "  ${CYAN}[1]${RESET} Start Service"
            echo "  ${CYAN}[Q]${RESET} Back"
            echo ""
            echo -n "  Select option: "
            read -r opt < /dev/tty
            
            case "$opt" in
                1)
                    systemctl start "${SERVICE_NAME}" 2>/dev/null
                    sleep 3
                    ;;
                [qQ])
                    break
                    ;;
            esac
            continue
        fi
        
        local has_password
        has_password=$(json_get "$auth_status" "has_password")
        local auth_enabled
        auth_enabled=$(json_get "$auth_status" "auth_enabled")
        local totp_enabled
        totp_enabled=$(json_get "$auth_status" "totp_enabled")
        
        echo ""
        echo "  ${BOLD}Current Status:${RESET}"
        echo ""
        if [ "$auth_enabled" = "true" ]; then
            echo "    Authentication: ${GREEN}● Enabled${RESET}"
        else
            echo "    Authentication: ${YELLOW}○ Disabled${RESET}"
        fi
        if [ "$has_password" = "true" ]; then
            echo "    Password:       ${GREEN}● Set${RESET}"
        else
            echo "    Password:       ${DIM}○ Not set${RESET}"
        fi
        if [ "$totp_enabled" = "true" ]; then
            echo "    2FA (TOTP):     ${GREEN}● Enabled${RESET}"
        else
            echo "    2FA (TOTP):     ${DIM}○ Not enabled${RESET}"
        fi
        echo ""
        print_separator
        echo ""
        
        if [ "$has_password" = "true" ]; then
            echo "  ${CYAN}[1]${RESET} Change Password"
            echo "  ${CYAN}[2]${RESET} ${RED}Remove Password${RESET} (disable auth)"
        else
            echo "  ${CYAN}[1]${RESET} Set Password (enable auth)"
        fi
        echo ""
        echo "  ${DIM}Note: 2FA (TOTP) can be configured in the web dashboard${RESET}"
        echo ""
        echo "  ${CYAN}[Q]${RESET} Back"
        echo ""
        echo -n "  Select option: "
        read -r opt < /dev/tty
        
        case "$opt" in
            1)
                set_dashboard_password
                ;;
            2)
                if [ "$has_password" = "true" ]; then
                    echo ""
                    log_warn "This will disable dashboard authentication"
                    if confirm "Remove password and disable auth?"; then
                        # Need current password to remove
                        echo -n "Enter current password: "
                        read -rs current_password < /dev/tty
                        echo ""
                        
                        local response
                        response=$(curl -fsSL -X POST "http://127.0.0.1:8080/api/auth/set-password" \
                            -H "Content-Type: application/json" \
                            -d "{\"current_password\":\"$current_password\",\"new_password\":\"\"}" 2>/dev/null)
                        
                        if echo "$response" | grep -q '"success":\s*true'; then
                            log_success "Password removed, authentication disabled"
                        else
                            local error
                            error=$(json_get "$response" "message")
                            log_error "Failed: ${error:-Unknown error}"
                        fi
                        press_enter
                    fi
                fi
                ;;
            [qQ])
                break
                ;;
        esac
    done
}

# =============================================================================
# Live System Monitor
# =============================================================================

# Get CPU usage percentage
get_cpu_usage() {
    if command -v mpstat &>/dev/null; then
        mpstat 1 1 2>/dev/null | awk '/Average:/ {print 100 - $NF}' | head -1
    elif [ -f /proc/stat ]; then
        # Read CPU stats twice with 1 second interval
        local cpu1 cpu2
        cpu1=$(head -1 /proc/stat | awk '{print $2+$3+$4, $2+$3+$4+$5}')
        sleep 0.5
        cpu2=$(head -1 /proc/stat | awk '{print $2+$3+$4, $2+$3+$4+$5}')
        echo "$cpu1 $cpu2" | awk '{
            active = $3 - $1
            total = $4 - $2
            if (total > 0) printf "%.1f", (active / total) * 100
            else print "0.0"
        }'
    else
        echo "N/A"
    fi
}

# Get memory usage
get_memory_info() {
    if command -v free &>/dev/null; then
        free -h | awk '/^Mem:/ {print $3 "/" $2 " (" int($3/$2*100) "%)"}'
    else
        echo "N/A"
    fi
}

# Get swap usage
get_swap_info() {
    if command -v free &>/dev/null; then
        local swap_line
        swap_line=$(free -h 2>/dev/null | awk '/^Swap:/')
        local total used
        total=$(echo "$swap_line" | awk '{print $2}')
        used=$(echo "$swap_line" | awk '{print $3}')
        if [ "$total" = "0B" ] || [ -z "$total" ]; then
            echo "Not configured"
        else
            local pct
            pct=$(free 2>/dev/null | awk '/^Swap:/ {if($2>0) print int($3/$2*100); else print 0}')
            echo "${used}/${total} (${pct}%)"
        fi
    else
        echo "N/A"
    fi
}

# Get disk usage for root partition
get_disk_info() {
    df -h / 2>/dev/null | awk 'NR==2 {print $3 "/" $2 " (" $5 ")"}'
}

# Get system uptime
get_uptime_info() {
    uptime -p 2>/dev/null | sed 's/^up //' || uptime | awk -F'( |,|:)+' '{print $6" days "$8"h "$9"m"}'
}

# Get load averages
get_load_avg() {
    cat /proc/loadavg 2>/dev/null | awk '{print $1 ", " $2 ", " $3}'
}

# Get network I/O (bytes received/transmitted)
get_network_io() {
    local iface
    iface=$(ip route get 1 2>/dev/null | awk '{for(i=1;i<=NF;i++)if($i=="dev"){print $(i+1);exit}}')
    if [ -z "$iface" ]; then
        iface=$(ip link 2>/dev/null | awk -F: '$0!~"lo|vir|docker|br-"{print $2;exit}' | tr -d ' ')
    fi
    
    if [ -n "$iface" ] && [ -f "/sys/class/net/${iface}/statistics/rx_bytes" ]; then
        local rx tx
        rx=$(cat "/sys/class/net/${iface}/statistics/rx_bytes" 2>/dev/null)
        tx=$(cat "/sys/class/net/${iface}/statistics/tx_bytes" 2>/dev/null)
        
        # Convert to human readable
        local rx_h tx_h
        rx_h=$(numfmt --to=iec-i --suffix=B "$rx" 2>/dev/null || echo "${rx}B")
        tx_h=$(numfmt --to=iec-i --suffix=B "$tx" 2>/dev/null || echo "${tx}B")
        echo "RX: ${rx_h}  TX: ${tx_h} (${iface})"
    else
        echo "N/A"
    fi
}

# Get process count
get_process_count() {
    ps aux 2>/dev/null | wc -l | awk '{print $1 - 1}'
}

# Get bot-specific stats from API
get_bot_stats() {
    local response
    response=$(curl -fsSL --connect-timeout 2 "http://127.0.0.1:8080/api/status" 2>/dev/null)
    if [ -n "$response" ]; then
        echo "$response"
    else
        echo ""
    fi
}

# Get bot positions from API
get_bot_positions() {
    local response
    response=$(curl -fsSL --connect-timeout 2 "http://127.0.0.1:8080/api/positions" 2>/dev/null)
    if [ -n "$response" ]; then
        echo "$response"
    else
        echo ""
    fi
}

# Get public IP
get_public_ip() {
    if [ -f /tmp/.screenerbot_ip ]; then
        local age
        age=$(( $(date +%s) - $(stat -c %Y /tmp/.screenerbot_ip 2>/dev/null || echo 0) ))
        if [ "$age" -lt 3600 ]; then
            cat /tmp/.screenerbot_ip
            return
        fi
    fi
    
    local ip
    ip=$(curl -fsSL --connect-timeout 2 https://api.ipify.org 2>/dev/null)
    if [ -n "$ip" ]; then
        echo "$ip" | tee /tmp/.screenerbot_ip
    else
        echo "N/A"
    fi
}

# Draw a usage bar
draw_bar() {
    local percent=$1
    local width=${2:-30}
    local filled=$((percent * width / 100))
    local empty=$((width - filled))
    
    # Determine color based on percentage
    local color="$GREEN"
    if [ "$percent" -ge 80 ]; then
        color="$RED"
    elif [ "$percent" -ge 60 ]; then
        color="$YELLOW"
    fi
    
    printf "["
    printf "${color}%${filled}s" '' | tr ' ' '#'
    printf "${RESET}%${empty}s" '' | tr ' ' '-'
    printf "] %3d%%" "$percent"
}

# Live system monitor display
system_monitor() {
    local refresh_interval=2
    
    # Hide cursor
    tput civis 2>/dev/null || true
    
    # Cleanup on exit
    trap 'tput cnorm 2>/dev/null; return' INT TERM
    
    while true; do
        clear
        
        # Compact header
        echo -e "${CYAN}${BOLD}"
        echo "  ============================================================================"
        echo "  |               SCREENERBOT SYSTEM MONITOR                                |"
        echo "  ============================================================================${RESET}"
        echo ""
        
        local now
        now=$(date '+%Y-%m-%d %H:%M:%S')
        echo -e "  ${DIM}Last update: ${now}  |  Refresh: ${refresh_interval}s  |  Press Q to exit${RESET}"
        echo ""
        echo -e "  ${CYAN}----------------------------------------------------------------------------${RESET}"
        
        # === SYSTEM SECTION ===
        echo -e "  ${BOLD}SYSTEM${RESET}"
        echo -e "  ${CYAN}----------------------------------------------------------------------------${RESET}"
        
        # CPU
        local cpu_pct
        cpu_pct=$(get_cpu_usage)
        if [ "$cpu_pct" != "N/A" ]; then
            local cpu_int=${cpu_pct%.*}
            printf "  %-12s " "CPU:"
            draw_bar "$cpu_int" 30
            echo ""
        else
            echo "  CPU:         N/A"
        fi
        
        # Memory
        local mem_info
        mem_info=$(get_memory_info)
        local mem_pct
        mem_pct=$(echo "$mem_info" | grep -oE '[0-9]+%' | head -1 | tr -d '%')
        printf "  %-12s " "Memory:"
        if [ -n "$mem_pct" ]; then
            draw_bar "$mem_pct" 30
            echo "  ${mem_info%(*}"
        else
            echo "$mem_info"
        fi
        
        # Swap
        local swap_info
        swap_info=$(get_swap_info)
        if [[ "$swap_info" != *"Not configured"* ]] && [[ "$swap_info" != "N/A" ]]; then
            local swap_pct
            swap_pct=$(echo "$swap_info" | grep -oE '[0-9]+%' | head -1 | tr -d '%')
            printf "  %-12s " "Swap:"
            if [ -n "$swap_pct" ]; then
                draw_bar "$swap_pct" 30
                echo "  ${swap_info%(*}"
            else
                echo "$swap_info"
            fi
        else
            echo "  Swap:        ${DIM}Not configured${RESET}"
        fi
        
        # Disk
        local disk_info
        disk_info=$(get_disk_info)
        local disk_pct
        disk_pct=$(echo "$disk_info" | grep -oE '[0-9]+%' | head -1 | tr -d '%')
        printf "  %-12s " "Disk (/):"
        if [ -n "$disk_pct" ]; then
            draw_bar "$disk_pct" 30
            echo "  ${disk_info%(*}"
        else
            echo "$disk_info"
        fi
        
        echo ""
        
        # Load & Uptime
        local load_avg uptime_info proc_count
        load_avg=$(get_load_avg)
        uptime_info=$(get_uptime_info)
        proc_count=$(get_process_count)
        
        echo "  Load Avg:    ${load_avg}"
        echo "  Uptime:      ${uptime_info}"
        echo "  Processes:   ${proc_count}"
        
        # Network
        local net_info public_ip
        net_info=$(get_network_io)
        public_ip=$(get_public_ip)
        echo "  Network:     ${net_info}"
        echo "  Public IP:   ${public_ip}"
        
        echo ""
        echo -e "  ${CYAN}----------------------------------------------------------------------------${RESET}"
        
        # === SCREENERBOT SECTION ===
        echo -e "  ${BOLD}SCREENERBOT${RESET}"
        echo -e "  ${CYAN}----------------------------------------------------------------------------${RESET}"
        
        # Service status
        local service_status_text service_pid service_mem
        if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
            service_status_text="${GREEN}RUNNING${RESET}"
            service_pid=$(systemctl show -p MainPID "${SERVICE_NAME}" 2>/dev/null | cut -d= -f2)
            if [ -n "$service_pid" ] && [ "$service_pid" != "0" ]; then
                # Get process memory usage
                service_mem=$(ps -o rss= -p "$service_pid" 2>/dev/null | awk '{printf "%.1f MB", $1/1024}')
            fi
        elif systemctl list-unit-files 2>/dev/null | grep -q "${SERVICE_NAME}"; then
            service_status_text="${YELLOW}STOPPED${RESET}"
        else
            service_status_text="${DIM}NOT INSTALLED${RESET}"
        fi
        
        local installed_version
        installed_version=$(get_installed_version)
        
        echo "  Status:      ${service_status_text}"
        if [ -n "$installed_version" ]; then
            echo "  Version:     v${installed_version}"
        fi
        if [ -n "$service_pid" ] && [ "$service_pid" != "0" ]; then
            echo "  PID:         ${service_pid}"
        fi
        if [ -n "$service_mem" ]; then
            echo "  Memory:      ${service_mem}"
        fi
        
        # Try to get bot stats from API
        local bot_stats
        bot_stats=$(get_bot_stats)
        
        if [ -n "$bot_stats" ]; then
            # Parse wallet balance if available
            local wallet_sol
            wallet_sol=$(json_get "$bot_stats" "sol_balance")
            if [ -n "$wallet_sol" ]; then
                echo "  Wallet:      ${GREEN}${wallet_sol} SOL${RESET}"
            fi
            
            # Parse positions count
            local positions_count
            positions_count=$(json_get "$bot_stats" "open_positions")
            if [ -n "$positions_count" ]; then
                echo "  Positions:   ${positions_count} open"
            fi
            
            # Parse trader status
            local trading_enabled
            trading_enabled=$(json_get "$bot_stats" "trading_enabled")
            if [ "$trading_enabled" = "true" ]; then
                echo "  Trader:      ${GREEN}ENABLED${RESET}"
            elif [ "$trading_enabled" = "false" ]; then
                echo "  Trader:      ${YELLOW}DISABLED${RESET}"
            fi
        else
            if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
                echo "  Dashboard:   ${DIM}Waiting for API...${RESET}"
            fi
        fi
        
        # Data directory info
        local data_dir
        data_dir=$(get_data_dir)
        if [ -d "$data_dir" ]; then
            local data_size
            data_size=$(du -sh "$data_dir" 2>/dev/null | cut -f1)
            echo "  Data Size:   ${data_size:-N/A}"
        fi
        
        echo ""
        echo -e "  ${CYAN}----------------------------------------------------------------------------${RESET}"
        echo ""
        echo "  ${DIM}[Q] Quit  |  [R] Refresh now  |  [+] Faster  |  [-] Slower${RESET}"
        
        # Check for key press (non-blocking)
        local key=""
        read -rsn1 -t "$refresh_interval" key 2>/dev/null || true
        
        case "$key" in
            q|Q)
                tput cnorm 2>/dev/null || true
                trap - INT TERM
                return
                ;;
            r|R)
                # Immediate refresh
                continue
                ;;
            +|=)
                if [ "$refresh_interval" -gt 1 ]; then
                    refresh_interval=$((refresh_interval - 1))
                fi
                ;;
            -|_)
                if [ "$refresh_interval" -lt 10 ]; then
                    refresh_interval=$((refresh_interval + 1))
                fi
                ;;
        esac
    done
    
    # Restore cursor
    tput cnorm 2>/dev/null || true
    trap - INT TERM
}

# =============================================================================
# Status & Info Functions
# =============================================================================

show_status() {
    print_banner
    echo "${BOLD}  ${ICON_STATUS}  ScreenerBot Status${RESET}"
    echo ""
    print_separator
    
    # Installation status
    echo ""
    echo "${BOLD}Installation:${RESET}"
    echo ""
    
    local installed_version
    installed_version=$(get_installed_version)
    
    if [ -n "$installed_version" ]; then
        echo "  Version:     ${GREEN}${BOLD}v${installed_version}${RESET}"
        echo "  Binary:      ${INSTALL_DIR}/screenerbot"
        echo "  Symlink:     ${SYMLINK_PATH}"
    else
        echo "  ${DIM}ScreenerBot is not installed${RESET}"
    fi
    
    # Data directory
    local data_dir
    data_dir=$(get_data_dir)
    echo ""
    echo "${BOLD}Data Directory:${RESET}"
    echo ""
    if [ -d "$data_dir" ]; then
        local data_size
        data_size=$(du -sh "$data_dir" 2>/dev/null | cut -f1)
        echo "  Path:        ${data_dir}"
        echo "  Size:        ${data_size:-unknown}"
        
        if [ -f "${data_dir}/data/config.toml" ]; then
            echo "  Config:      ${GREEN}${ICON_CHECK} Found${RESET}"
        else
            echo "  Config:      ${YELLOW}${ICON_WARN} Not configured${RESET}"
        fi
    else
        echo "  ${DIM}Data directory not created yet${RESET}"
    fi
    
    # Service status
    service_status
    
    # Latest version check
    echo "${BOLD}Latest Version:${RESET}"
    echo ""
    local latest_response
    latest_response=$(get_latest_release 2>/dev/null)
    if [ -n "$latest_response" ]; then
        local latest_version
        if command -v jq &>/dev/null; then
            latest_version=$(echo "$latest_response" | jq -r '.data.version' 2>/dev/null)
        else
            latest_version=$(echo "$latest_response" | sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\?\([^,\"]*\)"\?.*/\1/p' | head -1)
        fi
        
        if [ -n "$latest_version" ]; then
            echo "  Available:   v${latest_version}"
            
            if [ -n "$installed_version" ]; then
                compare_versions "$installed_version" "$latest_version"
                local cmp_result=$?
                case $cmp_result in
                    0) echo "  Status:      ${GREEN}Up to date${RESET}" ;;
                    2) echo "  Status:      ${YELLOW}Update available!${RESET}" ;;
                esac
            fi
        fi
    else
        echo "  ${DIM}Could not fetch latest version${RESET}"
    fi
    
    echo ""
    print_separator
    press_enter
}

# =============================================================================
# Help & Tips
# =============================================================================

show_help() {
    print_banner
    echo "${BOLD}  ${ICON_HELP}  Help & Tips${RESET}"
    echo ""
    print_separator
    echo ""
    
    echo "${BOLD}${CYAN}Quick Start:${RESET}"
    echo ""
    echo "  1. Install ScreenerBot using option [1]"
    echo "  2. Configure your wallet and RPC in the dashboard"
    echo "  3. Access dashboard at http://localhost:8080"
    echo "  4. Enable auto-start via option [6] Manage Service"
    echo ""
    
    echo "${BOLD}${CYAN}Remote Dashboard Access:${RESET}"
    echo ""
    echo "  The safest way to access your dashboard remotely is via SSH tunnel:"
    echo ""
    echo "  ${DIM}ssh -L 8080:localhost:8080 user@your-server-ip${RESET}"
    echo ""
    echo "  Then open http://localhost:8080 in your local browser."
    echo ""
    
    echo "${BOLD}${CYAN}Useful Commands:${RESET}"
    echo ""
    echo "  View logs:          ${DIM}journalctl -u screenerbot -f${RESET}"
    echo "  Restart service:    ${DIM}sudo systemctl restart screenerbot${RESET}"
    echo "  Check status:       ${DIM}sudo systemctl status screenerbot${RESET}"
    echo "  Edit config:        ${DIM}nano ~/.local/share/ScreenerBot/data/config.toml${RESET}"
    echo ""
    
    echo "${BOLD}${CYAN}Security Tips:${RESET}"
    echo ""
    echo "  • Never expose port 8080 to the public internet"
    echo "  • Use SSH tunnel or VPN for remote access"
    echo "  • Keep your system and ScreenerBot updated"
    echo "  • Enable Telegram notifications for monitoring"
    echo "  • Regularly backup your data directory"
    echo ""
    
    echo "${BOLD}${CYAN}Resources:${RESET}"
    echo ""
    echo "  Documentation:      ${CYAN}https://screenerbot.io/docs${RESET}"
    echo "  Telegram Channel:   ${CYAN}https://t.me/screenerbotio${RESET}"
    echo "  Telegram Group:     ${CYAN}https://t.me/screenerbotio_talk${RESET}"
    echo "  Telegram Support:   ${CYAN}https://t.me/screenerbotio_support${RESET}"
    echo "  Twitter/X:          ${CYAN}https://x.com/screenerbotio${RESET}"
    echo ""
    
    print_separator
    press_enter
}

# =============================================================================
# Main Menu
# =============================================================================

# Quick stats for main menu (non-blocking)
get_quick_cpu() {
    if [ -f /proc/stat ]; then
        local user nice sys idle iow irq sirq
        read -r _ user nice sys idle iow irq sirq _ < /proc/stat
        local total=$((user + nice + sys + idle + iow + irq + sirq))
        
        if [ -f /tmp/.screenerbot_cpu_stat ]; then
            local prev_total prev_idle
            read -r prev_total prev_idle < /tmp/.screenerbot_cpu_stat
            
            local diff_total=$((total - prev_total))
            local diff_idle=$((idle - prev_idle))
            
            if [ "$diff_total" -gt 0 ]; then
                local cpu=$((100 * (diff_total - diff_idle) / diff_total))
                echo "$cpu"
            else
                echo "0"
            fi
        else
            # First run, return 0 and establish baseline
            echo "0"
        fi
        
        echo "$total $idle" > /tmp/.screenerbot_cpu_stat
    else
        echo "N/A"
    fi
}

get_quick_memory() {
    if [ -f /proc/meminfo ]; then
        local total used pct
        total=$(awk '/MemTotal/ {print $2}' /proc/meminfo)
        local available
        available=$(awk '/MemAvailable/ {print $2}' /proc/meminfo)
        if [ -n "$total" ] && [ -n "$available" ] && [ "$total" -gt 0 ]; then
            used=$((total - available))
            pct=$((used * 100 / total))
            local used_h total_h
            used_h=$(awk "BEGIN {printf \"%.1f\", $used/1048576}")
            total_h=$(awk "BEGIN {printf \"%.1f\", $total/1048576}")
            echo "${pct}|${used_h}G/${total_h}G"
        else
            echo "N/A"
        fi
    else
        echo "N/A"
    fi
}

get_quick_disk() {
    df / 2>/dev/null | awk 'NR==2 {gsub(/%/,"",$5); print $5 "|" $3 "/" $2}'
}

get_quick_load() {
    cut -d' ' -f1-3 /proc/loadavg 2>/dev/null | tr ' ' ', '
}

get_quick_uptime() {
    local up
    up=$(cat /proc/uptime 2>/dev/null | cut -d. -f1)
    if [ -n "$up" ]; then
        local days=$((up / 86400))
        local hours=$(( (up % 86400) / 3600 ))
        local mins=$(( (up % 3600) / 60 ))
        if [ "$days" -gt 0 ]; then
            echo "${days}d ${hours}h ${mins}m"
        elif [ "$hours" -gt 0 ]; then
            echo "${hours}h ${mins}m"
        else
            echo "${mins}m"
        fi
    else
        echo "N/A"
    fi
}

get_bot_quick_stats() {
    # Quick API call with short timeout
    curl -fsSL --connect-timeout 1 --max-time 2 "http://127.0.0.1:8080/api/status" 2>/dev/null
}

# Draw modern horizontal bar for main menu (Option C style)
draw_modern_bar() {
    local percent=$1
    local width=${2:-12}
    
    if [ "$percent" = "N/A" ]; then
        printf "%-${width}s %s" "" "N/A"
        return
    fi
    
    local filled=$((percent * width / 100))
    local empty=$((width - filled))
    
    # Color based on percentage
    local color="$GREEN"
    if [ "$percent" -ge 80 ]; then
        color="$RED"
    elif [ "$percent" -ge 60 ]; then
        color="$YELLOW"
    fi
    
    printf "${color}"
    printf "%${filled}s" '' | tr ' ' '▰'
    printf "${RESET}${DIM}"
    printf "%${empty}s" '' | tr ' ' '▱'
    printf "${RESET} %3d%%" "$percent"
}

print_dashboard_status() {
    # Option C Status - Clean Horizontal Bars
    echo -e "  ${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
    echo -e "  ${BOLD}SYSTEM${RESET}                                 ${BOLD}SCREENERBOT${RESET}"
    echo ""
    
    # Get system stats (fast)
    local cpu_pct mem_info disk_info load_avg uptime_info
    cpu_pct=$(get_quick_cpu)
    mem_info=$(get_quick_memory)
    disk_info=$(get_quick_disk)
    load_avg=$(get_quick_load)
    uptime_info=$(get_quick_uptime)
    
    local mem_pct disk_pct
    mem_pct=$(echo "$mem_info" | cut -d'|' -f1)
    disk_pct=$(echo "$disk_info" | cut -d'|' -f1)
    
    # Get bot status (fast - cache systemctl result)
    local installed_version service_status_text service_pid bot_mem
    local service_running=false
    installed_version=$(get_installed_version)
    
    if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
        service_running=true
        service_status_text="${GREEN}● RUNNING${RESET}"
        service_pid=$(systemctl show -p MainPID "${SERVICE_NAME}" 2>/dev/null | cut -d= -f2)
        if [ -n "$service_pid" ] && [ "$service_pid" != "0" ]; then
            bot_mem=$(ps -o rss= -p "$service_pid" 2>/dev/null | awk '{printf "%.0f", $1/1024}')
        fi
    elif systemctl list-unit-files 2>/dev/null | grep -q "${SERVICE_NAME}"; then
        service_status_text="${YELLOW}○ STOPPED${RESET}"
    else
        service_status_text="${DIM}○ NOT INSTALLED${RESET}"
    fi
    
    # CPU | Status
    printf "  CPU    "
    draw_modern_bar "$cpu_pct" 12
    printf "             Status   %b\n" "$service_status_text"
    
    # Memory | Version
    printf "  Memory "
    draw_modern_bar "$mem_pct" 12
    if [ -n "$installed_version" ]; then
        printf "             Version  ${GREEN}v%s${RESET}\n" "$installed_version"
    else
        printf "             Version  ${DIM}---${RESET}\n"
    fi
    
    # Disk | PID
    printf "  Disk   "
    draw_modern_bar "$disk_pct" 12
    if [ -n "$service_pid" ] && [ "$service_pid" != "0" ]; then
        printf "             PID      %s (%sMB)\n" "$service_pid" "$bot_mem"
    else
        printf "             PID      ${DIM}---${RESET}\n"
    fi
    
    # Load | Wallet (get bot stats only if running)
    local wallet_sol positions_count trader_status
    if [ "$service_running" = true ]; then
        local bot_stats
        bot_stats=$(get_bot_quick_stats)
        if [ -n "$bot_stats" ]; then
            wallet_sol=$(json_get "$bot_stats" "sol_balance")
            positions_count=$(json_get "$bot_stats" "open_positions")
            trader_status=$(json_get "$bot_stats" "trading_enabled")
        fi
    fi
    
    printf "  Load   %-22s" "$load_avg"
    if [ -n "$wallet_sol" ]; then
        printf "             Wallet   ${GREEN}◆ %.4f SOL${RESET}\n" "$wallet_sol"
    else
        printf "             Wallet   ${DIM}---${RESET}\n"
    fi
    
    # Uptime | Trading
    printf "  Uptime %-22s" "$uptime_info"
    if [ -n "$positions_count" ]; then
        if [ "$trader_status" = "true" ]; then
            printf "             Trading  ${GREEN}▲ ON${RESET} (%s pos)\n" "$positions_count"
        else
            printf "             Trading  ${YELLOW}▼ OFF${RESET} (%s pos)\n" "$positions_count"
        fi
    else
        printf "             Trading  ${DIM}---${RESET}\n"
    fi
    
    echo ""
    echo -e "  ${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
}

main_menu() {
    while true; do
        local choice=-1
        local input_buffer=""
        local installed_version
        
        # Inner loop: Live Dashboard & Input
        while true; do
            clear
            print_banner
            print_dashboard_status
            
            # Build menu options based on installation state
            installed_version=$(get_installed_version)
            local options=()
            if [ -z "$installed_version" ]; then
                options+=("${ICON_PACKAGE} Install ScreenerBot")
            else
                options+=("${ICON_PACKAGE} Reinstall ScreenerBot")
            fi
            options+=(
                "${ICON_UPDATE} Update ScreenerBot"
                "${ICON_TRASH} Uninstall ScreenerBot"
                "${ICON_BACKUP} Backup Data"
                "${ICON_RESTORE} Restore Data"
                "${ICON_SERVICE} Manage Service"
                "${ICON_MONITOR} System Monitor"
                "${ICON_LOCK} Dashboard Security"
                "${ICON_STATUS} Status & Info"
                "${ICON_CHECK} System Check"
                "${ICON_TELEGRAM} Setup Update Notifications"
                "${ICON_UPDATE} Update Management Script"
                "${ICON_HELP} Help & Tips"
                "${ICON_EXIT} Exit"
            )
            
            # Print options
            for i in "${!options[@]}"; do
                printf "  [${CYAN}%d${RESET}] %s\n" "$((i + 1))" "${options[$i]}"
            done
            echo "  [${CYAN}Q${RESET}] Quit"
            echo ""
            
            # Prompt
            printf "  Select option [1-%d]: %s" "${#options[@]}" "$input_buffer"
            
            # Read input with timeout
            local key
            read -rsn1 -t 10 key < /dev/tty
            local exit_code=$?
            
            if [ $exit_code -gt 128 ]; then
                # Timeout - loop to refresh
                continue
            fi
            
            if [[ "$key" == "" ]]; then
                # Enter pressed
                if [[ -n "$input_buffer" ]]; then
                    if [[ "$input_buffer" =~ ^[0-9]+$ ]]; then
                        local val=$((input_buffer))
                        if [ "$val" -ge 1 ] && [ "$val" -le "${#options[@]}" ]; then
                            choice=$((val - 1))
                            break # Break inner loop to execute action
                        fi
                    fi
                    # Invalid input, clear buffer
                    input_buffer=""
                fi
            elif [[ "$key" =~ [qQ] ]]; then
                exit 0
            elif [[ "$key" =~ [0-9] ]]; then
                input_buffer="${input_buffer}${key}"
            elif [[ "$key" == $'\x7f' || "$key" == $'\08' ]]; then # Backspace
                input_buffer="${input_buffer%?}"
            fi
        done
        
        # Re-fetch version for case context
        installed_version=$(get_installed_version)
        
        case "$choice" in
            0)
                # Install/Reinstall
                if [ -n "$installed_version" ]; then
                    log_warn "ScreenerBot is already installed (v${installed_version})"
                    if ! confirm "Reinstall and replace current installation?"; then
                        continue
                    fi
                fi
                
                if check_requirements; then
                    echo ""
                    # Get latest version
                    local latest_response
                    latest_response=$(get_latest_release)
                    local latest_version
                    
                    if [ -n "$latest_response" ] && command -v jq &>/dev/null; then
                        latest_version=$(echo "$latest_response" | jq -r '.data.version' 2>/dev/null)
                    elif [ -n "$latest_response" ]; then
                        latest_version=$(echo "$latest_response" | sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\?\([^,\"]*\)"\?.*/\1/p' | head -1)
                    fi
                    
                    if [ -z "$latest_version" ]; then
                        log_error "Failed to get latest version"
                        press_enter
                        continue
                    fi
                    
                    echo ""
                    log_info "Latest version: ${BOLD}v${latest_version}${RESET}"
                    echo ""
                    echo -n "Install version [${latest_version}] or Q to cancel: "
                    read -r user_version < /dev/tty
                    
                    # Handle quit/cancel
                    if [[ "$user_version" =~ ^[qQcC]$ ]]; then
                        log_info "Installation cancelled"
                        press_enter
                        continue
                    fi
                    
                    if [ -z "$user_version" ]; then
                        user_version="$latest_version"
                    fi
                    
                    # Validate version format (x.y.z)
                    if ! [[ "$user_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
                        log_error "Invalid version format. Expected: x.y.z (e.g., 0.1.107)"
                        press_enter
                        continue
                    fi
                    
                    if download_and_install "$user_version"; then
                        echo ""
                        # Check if service already exists
                        if systemctl list-unit-files 2>/dev/null | grep -q "${SERVICE_NAME}"; then
                            if confirm "Update systemd service configuration?" "n"; then
                                create_service
                            else
                                # Just start/restart the service if it exists
                                if confirm "Start/restart service now?" "y"; then
                                    systemctl restart "${SERVICE_NAME}" 2>/dev/null || systemctl start "${SERVICE_NAME}"
                                    sleep 2
                                    if systemctl is-active --quiet "${SERVICE_NAME}"; then
                                        log_success "Service started successfully!"
                                    else
                                        log_warn "Service may have failed to start. Check: journalctl -u ${SERVICE_NAME}"
                                    fi
                                fi
                            fi
                        else
                            if confirm "Create systemd service for auto-start?" "y"; then
                                create_service
                            fi
                        fi
                    fi
                fi
                press_enter
                ;;
            1)
                # Update
                if [ -z "$installed_version" ]; then
                    log_error "ScreenerBot is not installed"
                    press_enter
                    continue
                fi
                
                local arch
                arch=$(detect_arch)
                local platform="linux-${arch}-headless"
                
                log_info "Checking for updates..."
                local check_response
                check_response=$(check_update_available "$installed_version" "$platform")
                
                local update_available="false"
                local latest_version=""
                
                if [ -n "$check_response" ]; then
                    if command -v jq &>/dev/null; then
                        update_available=$(echo "$check_response" | jq -r '.data.updateAvailable' 2>/dev/null)
                        latest_version=$(echo "$check_response" | jq -r '.data.latestVersion' 2>/dev/null)
                    else
                        if echo "$check_response" | grep -q '"updateAvailable"\s*:\s*true'; then
                            update_available="true"
                        fi
                        latest_version=$(echo "$check_response" | sed -n 's/.*"latestVersion"[[:space:]]*:[[:space:]]*"\?\([^,\"]*\)"\?.*/\1/p' | head -1)
                    fi
                fi
                
                if [ "$update_available" = "true" ] && [ -n "$latest_version" ]; then
                    echo ""
                    log_success "Update available!"
                    echo ""
                    echo "  Current: v${installed_version}"
                    echo "  Latest:  v${latest_version}"
                    echo ""
                    
                    if confirm "Download and install update?"; then
                        if download_and_install "$latest_version"; then
                            if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
                                log_info "Restarting service..."
                                systemctl restart "${SERVICE_NAME}"
                                log_success "Service restarted with new version!"
                            fi
                        fi
                    fi
                else
                    log_success "You're running the latest version (v${installed_version})"
                fi
                press_enter
                ;;
            2)
                # Uninstall
                if [ -z "$installed_version" ]; then
                    log_warn "ScreenerBot is not installed"
                    press_enter
                    continue
                fi
                
                echo ""
                log_warn "This will remove ScreenerBot from your system"
                if confirm "Are you sure you want to uninstall?"; then
                    uninstall
                fi
                press_enter
                ;;
            3)
                # Backup
                create_backup
                press_enter
                ;;
            4)
                # Restore
                restore_backup
                press_enter
                ;;
            5)
                # Service menu
                service_menu
                ;;
            6)
                # System Monitor
                system_monitor
                ;;
            7)
                # Dashboard Security
                manage_dashboard_security
                ;;
            8)
                # Status
                show_status
                ;;
            9)
                # System Check
                check_requirements
                press_enter
                ;;
            10)
                # Telegram notifications
                setup_update_notifications
                ;;
            11)
                # Update Script
                self_update
                ;;
            12)
                # Help
                show_help
                ;;
            13|-1)
                echo ""
                log_info "Thanks for using ScreenerBot! ${ICON_ROCKET}"
                echo ""
                exit 0
                ;;
        esac
    done
}

# =============================================================================
# Command Line Arguments
# =============================================================================

show_usage() {
    echo "ScreenerBot VPS Manager v${SCRIPT_VERSION}"
    echo ""
    echo "Usage: $0 [command] [options]"
    echo ""
    echo "Commands:"
    echo "  install [version]    Install ScreenerBot (latest if version not specified)"
    echo "  update               Check and install bot updates"
    echo "  uninstall            Remove ScreenerBot"
    echo "  status               Show installation status"
    echo "  monitor              Live system monitor (CPU, memory, disk, bot stats)"
    echo "  backup               Create backup of data directory"
    echo "  restore [file]       Restore from backup"
    echo "  start                Start the service"
    echo "  stop                 Stop the service"
    echo "  restart              Restart the service"
    echo "  logs                 View service logs"
    echo "  self-update          Update this management script"
    echo "  install-manager      Install manager to /usr/local/bin/screenerbot-manager"
    echo "  help                 Show this help message"
    echo ""
    echo "Without arguments, starts interactive menu mode (auto-checks for script updates)."
    echo ""
    echo "Examples:"
    echo "  $0                   # Interactive menu"
    echo "  $0 install           # Install latest version"
    echo "  $0 install 0.1.107   # Install specific version"
    echo "  $0 update            # Check and install updates"
    echo "  $0 status            # Show status"
    echo "  $0 self-update       # Update management script"
    echo ""
}

# =============================================================================
# Main Entry Point
# =============================================================================

main() {
    # Handle command line arguments
    case "${1:-}" in
        install)
            check_requirements || exit 1
            local version="${2:-}"
            if [ -z "$version" ]; then
                local response
                response=$(get_latest_release)
                if command -v jq &>/dev/null; then
                    version=$(echo "$response" | jq -r '.data.version' 2>/dev/null)
                else
                    version=$(echo "$response" | sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\?\([^,\"]*\)"\?.*/\1/p' | head -1)
                fi
            fi
            if [ -z "$version" ]; then
                log_error "Failed to determine version"
                exit 1
            fi
            download_and_install "$version"
            ;;
        update)
            local installed_version
            installed_version=$(get_installed_version)
            if [ -z "$installed_version" ]; then
                log_error "ScreenerBot is not installed"
                exit 1
            fi
            
            local arch
            arch=$(detect_arch)
            local check_response
            check_response=$(check_update_available "$installed_version" "linux-${arch}-headless")
            
            local update_available="false"
            local latest_version=""
            
            if command -v jq &>/dev/null; then
                update_available=$(echo "$check_response" | jq -r '.data.updateAvailable' 2>/dev/null)
                latest_version=$(echo "$check_response" | jq -r '.data.latestVersion' 2>/dev/null)
            else
                if echo "$check_response" | grep -q '"updateAvailable"\s*:\s*true'; then
                    update_available="true"
                fi
                latest_version=$(echo "$check_response" | sed -n 's/.*"latestVersion"[[:space:]]*:[[:space:]]*"\?\([^,\"]*\)"\?.*/\1/p' | head -1)
            fi
            
            if [ "$update_available" = "true" ] && [ -n "$latest_version" ]; then
                log_info "Update available: v${installed_version} → v${latest_version}"
                download_and_install "$latest_version"
                if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
                    systemctl restart "${SERVICE_NAME}"
                    log_success "Service restarted"
                fi
            else
                log_success "Already up to date (v${installed_version})"
            fi
            ;;
        uninstall)
            uninstall
            ;;
        status)
            local version
            version=$(get_installed_version)
            if [ -n "$version" ]; then
                echo "Installed: v${version}"
                echo "Binary: ${INSTALL_DIR}/screenerbot"
            else
                echo "Not installed"
            fi
            
            if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
                echo "Service: running"
            elif systemctl list-unit-files | grep -q "${SERVICE_NAME}" 2>/dev/null; then
                echo "Service: stopped"
            else
                echo "Service: not configured"
            fi
            ;;
        monitor)
            system_monitor
            ;;
        backup)
            create_backup
            ;;
        restore)
            restore_backup "$2"
            ;;
        start)
            systemctl start "${SERVICE_NAME}"
            log_success "Service started"
            ;;
        stop)
            systemctl stop "${SERVICE_NAME}"
            log_success "Service stopped"
            ;;
        restart)
            systemctl restart "${SERVICE_NAME}"
            log_success "Service restarted"
            ;;
        logs)
            echo "Showing last 50 lines of logs (Ctrl+C to exit)..."
            journalctl -u "${SERVICE_NAME}" -n 50 -f
            ;;
        help|--help|-h)
            show_usage
            ;;
        self-update|update-script)
            self_update
            ;;
        install-manager)
            install_manager_script
            ;;
        "")
            # No arguments - start interactive menu
            # Check for script updates on startup
            auto_check_script_update
            main_menu
            ;;
        *)
            log_error "Unknown command: $1"
            echo ""
            show_usage
            exit 1
            ;;
    esac
}

# Run main
main "$@"
