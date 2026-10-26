#!/bin/bash
# Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
# SPDX-License-Identifier: BUSL-1.1
#
# Installs, updates and manages the headless ScreenerBot service on a Linux server.
#
#   Install or repair:   curl -fsSL https://screenerbot.io/install.sh | sudo bash
#   Manage afterwards:   sudo screenerbot-manager          (menu on a terminal)
#                        sudo screenerbot-manager help     (every command)
#
# Everything it creates, and nothing else:
#   /opt/screenerbot/                          the ScreenerBot binary and the previous one
#   /usr/local/bin/screenerbot                 link to the binary
#   /usr/local/bin/screenerbot-manager         this script
#   /etc/systemd/system/screenerbot.service    the service, never run as a login shell
#   /etc/systemd/system/screenerbot-auto-update.{service,timer}   only after "auto-update on"
#   /var/lib/screenerbot-manager/              update state and the packages it installed
#   the "screenerbot" system user              only when installed from a root login
#   the jq package                             only when it is missing
# Wallets, settings and databases stay in the service user's ~/.local/share/ScreenerBot.
# "uninstall" keeps them; only "uninstall --purge" deletes them.
#
# It reports nothing to any server. Every release is checked against the checksum on
# screenerbot.io and the one published with the GitHub release, and both must match.
# It never replaces, stops or removes a service, directory or file it did not create.

set -u
umask 022
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export PATH

readonly SCRIPT_VERSION="1.3.0"

readonly SITE_URL="https://screenerbot.io"
readonly API_BASE="${SITE_URL}/api"
readonly DEFAULT_SCRIPT_URL="${SITE_URL}/install.sh"
readonly GITHUB_RELEASE_BASE="https://github.com/farfary/ScreenerBot/releases/download"
readonly USER_AGENT="ScreenerBot-Manager/${SCRIPT_VERSION}"

readonly INSTALL_DIR="/opt/screenerbot"
readonly BINARY_PATH="${INSTALL_DIR}/screenerbot"
readonly PREVIOUS_BINARY_PATH="${INSTALL_DIR}/screenerbot.previous"
readonly INSTALL_MARKER="${INSTALL_DIR}/.installed-by-screenerbot-manager"
readonly SYMLINK_PATH="/usr/local/bin/screenerbot"
readonly MANAGER_PATH="/usr/local/bin/screenerbot-manager"
# Created only where sudo's secure_path omits /usr/local/bin (RHEL, AlmaLinux, Rocky).
readonly SUDO_LINK_PATH="/usr/bin/screenerbot-manager"
readonly SERVICE_NAME="screenerbot"
readonly SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
readonly AUTO_UPDATE_NAME="screenerbot-auto-update"
readonly AUTO_UPDATE_SERVICE_FILE="/etc/systemd/system/${AUTO_UPDATE_NAME}.service"
readonly AUTO_UPDATE_TIMER_FILE="/etc/systemd/system/${AUTO_UPDATE_NAME}.timer"
readonly TIMER_STAMP_DIR="/var/lib/systemd/timers"
readonly STATE_DIR="/var/lib/screenerbot-manager"
readonly UPDATE_STATE_FILE="${STATE_DIR}/update-state.json"
readonly PACKAGES_FILE="${STATE_DIR}/installed-packages"
readonly CREATED_DIRS_FILE="${STATE_DIR}/created-dirs"
readonly PACKAGE_LOG="${STATE_DIR}/packages.log"
readonly LOCK_FILE="/run/screenerbot-manager.lock"
readonly SERVICE_USER_NAME="screenerbot"
readonly SERVICE_USER_COMMENT="ScreenerBot trading service"
readonly DEFAULT_PORT=8080
readonly MIN_GLIBC="2.34"
readonly HEALTH_TIMEOUT_SECONDS=120

# The first line of every unit this script writes. Its presence is what makes a
# unit "ours"; a unit without it is never changed, stopped or removed.
readonly UNIT_MARKER="# Managed by screenerbot-manager; it rewrites this file on install and update."

# The 1.x installers wrote units and state without a marker. They are recognised
# by their exact content so an upgrade can adopt or retire them.
readonly LEGACY_UNIT_DESCRIPTION="Description=ScreenerBot - Automated Solana Trading Bot"
readonly LEGACY_UNIT_EXEC="ExecStart=/usr/local/bin/screenerbot"
readonly LEGACY_NOTIFIER_SERVICE_FILE="/etc/systemd/system/screenerbot-update.service"
readonly LEGACY_NOTIFIER_TIMER_FILE="/etc/systemd/system/screenerbot-update.timer"
readonly LEGACY_NOTIFIER_SERVICE_DESCRIPTION="Description=ScreenerBot Update Checker"
readonly LEGACY_NOTIFIER_TIMER_DESCRIPTION="Description=ScreenerBot Update Checker Timer"
readonly LEGACY_NOTIFIER_STATE_DIR="/var/lib/screenerbot-update-notifier"
readonly LEGACY_STATE_DIR="/var/lib/screenerbot"
readonly LEGACY_STATE_FILE="${LEGACY_STATE_DIR}/update-state.json"

# Files this script may leave in INSTALL_DIR; uninstall removes exactly these.
readonly INSTALL_DIR_FILES=(screenerbot screenerbot.previous screenerbot.new screenerbot.rollback
    screenerbot.previous.new .installed-by-screenerbot-manager)

# =============================================================================
# Output
# =============================================================================

if [ -t 1 ]; then
    C_BOLD=$'\033[1m'
    C_DIM=$'\033[2m'
    C_RED=$'\033[31m'
    C_GREEN=$'\033[32m'
    C_YELLOW=$'\033[33m'
    C_CYAN=$'\033[36m'
    C_RESET=$'\033[0m'
else
    C_BOLD=""
    C_DIM=""
    C_RED=""
    C_GREEN=""
    C_YELLOW=""
    C_CYAN=""
    C_RESET=""
fi

say() { printf '%s\n' "$*"; }
info() { printf '  %s\n' "$*"; }
step() { printf '\n%s==> %s%s\n' "$C_BOLD" "$*" "$C_RESET"; }
ok() { printf '  %s[ok]%s %s\n' "$C_GREEN" "$C_RESET" "$*"; }
warn() { printf '  %s[!]%s %s\n' "$C_YELLOW" "$C_RESET" "$*" >&2; }
err() { printf '  %s[x]%s %s\n' "$C_RED" "$C_RESET" "$*" >&2; }
die() {
    err "$@"
    exit 1
}
show_command() { printf '      %s%s%s\n' "$C_CYAN" "$*" "$C_RESET"; }

# =============================================================================
# Session state
# =============================================================================

HAS_TTY=0
ASSUME_YES=0
OPT_PORT=""
OPT_PURGE=0
OPT_FOLLOW=0
SCRIPT_URL="$DEFAULT_SCRIPT_URL"
SCRIPT_URL_IS_LOCAL=0
HTTP_STATUS=""
HTTP_BODY=""
LATEST_VERSION=""
UPDATE_VERSION=""
API_STATUS=""
API_BODY=""
API_COOKIE_JAR=""
PKG_MANAGER=""
CLEANUP_PATHS=()

SELF_PATH=""
if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
    SELF_PATH=$(readlink -f "${BASH_SOURCE[0]}")
fi

# Temporary paths are created only by mktemp under INSTALL_DIR or /tmp, and only
# those are removed here.
register_cleanup() { CLEANUP_PATHS+=("$1"); }

run_cleanup() {
    local path
    for path in ${CLEANUP_PATHS[@]+"${CLEANUP_PATHS[@]}"}; do
        case "$path" in
            "${INSTALL_DIR}"/.staging.* | /tmp/screenerbot-manager.*) rm -rf -- "$path" ;;
        esac
    done
    CLEANUP_PATHS=()
}

detect_tty() {
    if { : </dev/tty; } 2>/dev/null; then
        HAS_TTY=1
    fi
}

# A question answered from the terminal. Without a terminal the default is
# taken, and --yes answers yes to everything.
confirm() {
    local question="$1" default="${2:-n}" hint="[y/N]" reply=""
    if [ "$ASSUME_YES" = 1 ]; then
        return 0
    fi
    if [ "$HAS_TTY" != 1 ]; then
        [ "$default" = y ]
        return
    fi
    [ "$default" = y ] && hint="[Y/n]"
    printf '  %s %s ' "$question" "$hint" >/dev/tty
    IFS= read -r reply </dev/tty || reply=""
    [ -n "$reply" ] || reply="$default"
    case "$reply" in
        [yY] | [yY][eE][sS]) return 0 ;;
        *) return 1 ;;
    esac
}

# Reads one line from the terminal; prints the default without one.
ask() {
    local question="$1" default="${2:-}" reply=""
    if [ "$HAS_TTY" != 1 ]; then
        printf '%s' "$default"
        return 0
    fi
    if [ -n "$default" ]; then
        printf '  %s [%s]: ' "$question" "$default" >/dev/tty
    else
        printf '  %s: ' "$question" >/dev/tty
    fi
    IFS= read -r reply </dev/tty || reply=""
    printf '%s' "${reply:-$default}"
}

# Reads a secret without echoing it. Requires a terminal.
ask_secret() {
    local question="$1" reply=""
    [ "$HAS_TTY" = 1 ] || return 1
    printf '  %s: ' "$question" >/dev/tty
    IFS= read -rs reply </dev/tty || reply=""
    printf '\n' >/dev/tty
    printf '%s' "$reply"
}

pause() {
    [ "$HAS_TTY" = 1 ] || return 0
    printf '\n  %sPress Enter to return to the menu%s' "$C_DIM" "$C_RESET" >/dev/tty
    IFS= read -r _ </dev/tty || true
}

# =============================================================================
# Privileges, locking, test hook
# =============================================================================

require_root() {
    [ "$(id -u)" -eq 0 ] && return 0
    if [ -n "$SELF_PATH" ] && command -v sudo >/dev/null 2>&1; then
        info "Administrator rights are needed; asking sudo."
        exec sudo -- bash "$SELF_PATH" "$@"
    fi
    err "Run this as root, with sudo:"
    if [ -n "$SELF_PATH" ]; then
        show_command "sudo bash ${SELF_PATH} $*"
    else
        show_command "curl -fsSL ${DEFAULT_SCRIPT_URL} | sudo bash"
    fi
    exit 1
}

# One manager at a time: the automatic update timer and an administrator must
# never replace the binary concurrently. The lock lives in /run (memory only).
acquire_lock() {
    command -v flock >/dev/null 2>&1 || return 0
    exec 9>"$LOCK_FILE" || return 0
    if ! flock -n 9; then
        die "Another screenerbot-manager is running (an update or the automatic update check). Try again in a minute."
    fi
}

is_local_script_url() {
    [[ "$1" =~ ^http://(127\.0\.0\.1|localhost|host\.docker\.internal)(:[0-9]{1,5})?/[A-Za-z0-9._/-]*$ ]]
}

# SCREENERBOT_INSTALL_SCRIPT_URL exists for the installer's own test harness. It
# changes only where the manager copy is fetched from, accepts only a loopback
# HTTP server, and never affects where ScreenerBot releases come from.
configure_script_url() {
    local override="${SCREENERBOT_INSTALL_SCRIPT_URL:-}"
    [ -n "$override" ] || return 0
    if ! is_local_script_url "$override"; then
        die "SCREENERBOT_INSTALL_SCRIPT_URL accepts only http://127.0.0.1, http://localhost or http://host.docker.internal; refusing ${override}"
    fi
    SCRIPT_URL="$override"
    SCRIPT_URL_IS_LOCAL=1
    warn "TEST MODE: the manager script comes from ${override}. ScreenerBot releases still come from ${SITE_URL}."
}

# =============================================================================
# System facts
# =============================================================================

detect_arch() {
    local machine bits
    machine=$(uname -m)
    bits=$(getconf LONG_BIT 2>/dev/null || echo 64)
    case "$machine" in
        x86_64 | amd64)
            if [ "$bits" = 64 ]; then
                echo x64
                return 0
            fi
            ;;
        aarch64 | arm64)
            if [ "$bits" = 64 ]; then
                echo arm64
                return 0
            fi
            ;;
    esac
    return 1
}

glibc_version() {
    getconf GNU_LIBC_VERSION 2>/dev/null | awk '{ print $2 }'
}

os_name() {
    local name
    # shellcheck disable=SC1091
    name=$(. /etc/os-release 2>/dev/null && printf '%s' "${PRETTY_NAME:-${NAME:-}}")
    printf '%s' "${name:-Linux}"
}

systemd_running() {
    [ -d /run/systemd/system ]
}

is_version() {
    [[ "$1" =~ ^[0-9]{1,4}\.[0-9]{1,4}\.[0-9]{1,4}$ ]]
}

# True when version $1 is at least $2 (dotted numbers of any length).
version_at_least() {
    [ "$(printf '%s\n%s\n' "$2" "$1" | LC_ALL=C sort -V | head -n 1)" = "$2" ]
}

# Prints -1, 0 or 1 for two x.y.z versions.
version_compare() {
    local -a left right
    local i a b
    IFS=. read -r -a left <<<"$1"
    IFS=. read -r -a right <<<"$2"
    for i in 0 1 2; do
        a=$((10#${left[i]:-0}))
        b=$((10#${right[i]:-0}))
        if ((a < b)); then
            echo -1
            return 0
        elif ((a > b)); then
            echo 1
            return 0
        fi
    done
    echo 0
}

free_kb() {
    df -Pk "$1" 2>/dev/null | awk 'NR == 2 { print $4 }'
}

human_size() {
    local kb="${1:-0}"
    if [ "$kb" -ge 1048576 ]; then
        awk -v k="$kb" 'BEGIN { printf "%.1f GB", k / 1048576 }'
    else
        awk -v k="$kb" 'BEGIN { printf "%.0f MB", k / 1024 }'
    fi
}

binary_version() {
    timeout 15 "$1" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n 1
}

installed_version() {
    [ -x "$BINARY_PATH" ] || return 1
    local version
    version=$(binary_version "$BINARY_PATH")
    [ -n "$version" ] || return 1
    printf '%s' "$version"
}

# =============================================================================
# Ownership: what this script created, what belongs to someone else
# =============================================================================

# Prints absent, ours, legacy or foreign for the main service unit.
classify_service_unit() {
    local file="${1:-$SERVICE_FILE}"
    if [ ! -e "$file" ] && [ ! -L "$file" ]; then
        echo absent
    elif [ -L "$file" ] || [ ! -f "$file" ]; then
        echo foreign
    elif [ "$(head -n 1 "$file")" = "$UNIT_MARKER" ]; then
        echo ours
    elif grep -qxF "$LEGACY_UNIT_DESCRIPTION" "$file" && grep -qxF "$LEGACY_UNIT_EXEC" "$file"; then
        echo legacy
    else
        echo foreign
    fi
}

# A unit of the same name can also live in /usr/lib, /lib or /run. Prints the
# path of such a unit when it is not the file this script manages.
other_service_unit_path() {
    local fragment
    systemd_running || return 1
    fragment=$(systemctl show -p FragmentPath --value "${SERVICE_NAME}.service" 2>/dev/null)
    if [ -n "$fragment" ] && [ "$fragment" != "$SERVICE_FILE" ]; then
        printf '%s' "$fragment"
        return 0
    fi
    return 1
}

# Prints absent, ours, legacy or foreign for a helper unit.
classify_helper_unit() {
    local file="$1" legacy_description="${2:-}"
    if [ ! -e "$file" ] && [ ! -L "$file" ]; then
        echo absent
    elif [ -L "$file" ] || [ ! -f "$file" ]; then
        echo foreign
    elif [ "$(head -n 1 "$file")" = "$UNIT_MARKER" ]; then
        echo ours
    elif [ -n "$legacy_description" ] && grep -qxF "$legacy_description" "$file"; then
        echo legacy
    else
        echo foreign
    fi
}

# Prints absent, ours, legacy or foreign for the install directory.
classify_install_dir() {
    local dir="${1:-$INSTALL_DIR}" entry name known
    if [ ! -e "$dir" ] && [ ! -L "$dir" ]; then
        echo absent
        return 0
    fi
    if [ -L "$dir" ] || [ ! -d "$dir" ]; then
        echo foreign
        return 0
    fi
    if [ -f "${dir}/.installed-by-screenerbot-manager" ]; then
        echo ours
        return 0
    fi
    for entry in "$dir"/* "$dir"/.[!.]*; do
        [ -e "$entry" ] || [ -L "$entry" ] || continue
        name=${entry##*/}
        known=0
        case "$name" in
            screenerbot | screenerbot.previous | screenerbot.new | screenerbot.rollback) [ -f "$entry" ] && known=1 ;;
            .staging.*) [ -d "$entry" ] && known=1 ;;
        esac
        if [ "$known" != 1 ] || [ -L "$entry" ]; then
            echo foreign
            return 0
        fi
    done
    echo legacy
}

# Prints absent, ours or foreign for the command link.
classify_symlink() {
    local path="${1:-$SYMLINK_PATH}" target="${2:-$BINARY_PATH}"
    if [ ! -e "$path" ] && [ ! -L "$path" ]; then
        echo absent
    elif [ -L "$path" ] && [ "$(readlink "$path")" = "$target" ]; then
        echo ours
    else
        echo foreign
    fi
}

# Prints absent, ours or foreign for the installed manager script.
classify_manager() {
    local path="${1:-$MANAGER_PATH}"
    if [ ! -e "$path" ] && [ ! -L "$path" ]; then
        echo absent
    elif [ -f "$path" ] && [ ! -L "$path" ] &&
        grep -q '^readonly SCRIPT_VERSION="' "$path" &&
        grep -qF 'screenerbot.io/install.sh' "$path"; then
        echo ours
    else
        echo foreign
    fi
}

# Stops before any change when a name this script uses belongs to something else.
guard_ownership() {
    local problems=() state other
    state=$(classify_service_unit)
    [ "$state" = foreign ] && problems+=("${SERVICE_FILE} is a different service")
    if [ "$state" = absent ] && other=$(other_service_unit_path); then
        problems+=("a service named ${SERVICE_NAME}.service already exists at ${other}")
    fi
    [ "$(classify_install_dir)" = foreign ] && problems+=("${INSTALL_DIR} holds files this installer did not create")
    [ "$(classify_symlink)" = foreign ] && problems+=("${SYMLINK_PATH} is not the link this installer creates")
    [ "$(classify_manager)" = foreign ] && problems+=("${MANAGER_PATH} is a different program")
    [ "$(classify_helper_unit "$AUTO_UPDATE_SERVICE_FILE")" = foreign ] && problems+=("${AUTO_UPDATE_SERVICE_FILE} is a different service")
    [ "$(classify_helper_unit "$AUTO_UPDATE_TIMER_FILE")" = foreign ] && problems+=("${AUTO_UPDATE_TIMER_FILE} is a different timer")
    [ ${#problems[@]} -eq 0 ] && return 0

    err "This server already has something named like ScreenerBot that this installer did not create:"
    for state in "${problems[@]}"; do
        info "  - ${state}"
    done
    info "The installer never changes, stops or removes them, so nothing was changed."
    info "If they are no longer needed, rename or remove them yourself and run the installer again."
    exit 1
}

# =============================================================================
# The service account and its data
# =============================================================================

unit_value() {
    [ -f "$2" ] || return 1
    sed -n "s/^$1=//p" "$2" | tail -n 1
}

service_user() {
    local user
    user=$(unit_value User "$SERVICE_FILE" 2>/dev/null)
    printf '%s' "${user:-root}"
}

user_home() {
    getent passwd "$1" | cut -d: -f6
}

user_comment() {
    getent passwd "$1" | cut -d: -f5
}

service_home() {
    local home
    home=$(unit_value WorkingDirectory "$SERVICE_FILE" 2>/dev/null)
    [ -n "$home" ] || home=$(user_home "$(service_user)")
    printf '%s' "$home"
}

# The data directory the running service resolves: SCREENERBOT_DATA_DIR, then
# XDG_DATA_HOME, then ~/.local/share, read from the unit and its drop-ins.
service_data_dir() {
    local environment pair
    if systemd_running; then
        environment=$(systemctl show -p Environment --value "$SERVICE_NAME" 2>/dev/null)
        for pair in $environment; do
            case "$pair" in
                SCREENERBOT_DATA_DIR=?*)
                    printf '%s' "${pair#*=}"
                    return 0
                    ;;
            esac
        done
        for pair in $environment; do
            case "$pair" in
                XDG_DATA_HOME=?*)
                    printf '%s/ScreenerBot' "${pair#*=}"
                    return 0
                    ;;
            esac
        done
    fi
    printf '%s/.local/share/ScreenerBot' "$(service_home)"
}

# Who runs the service on a fresh install: the existing unit's user, the
# administrator who ran sudo, or a dedicated no-login system user.
choose_service_user() {
    local unit_state user
    unit_state=$(classify_service_unit)
    if [ "$unit_state" = ours ] || [ "$unit_state" = legacy ]; then
        service_user
        return 0
    fi
    user="${SUDO_USER:-}"
    if [ -n "$user" ] && [ "$user" != root ] && getent passwd "$user" >/dev/null; then
        printf '%s' "$user"
        return 0
    fi
    printf '%s' "$SERVICE_USER_NAME"
}

nologin_shell() {
    local shell
    for shell in /usr/sbin/nologin /sbin/nologin /usr/bin/nologin; do
        if [ -x "$shell" ]; then
            printf '%s' "$shell"
            return 0
        fi
    done
    printf '/bin/false'
}

# Refuses to adopt an existing "screenerbot" account this installer did not create.
check_service_user() {
    local name="$1"
    if [ "$name" = "$SERVICE_USER_NAME" ] && getent passwd "$name" >/dev/null &&
        [ "$(user_comment "$name")" != "$SERVICE_USER_COMMENT" ] && [ "$(classify_service_unit)" = absent ]; then
        err "A user named ${name} already exists and was not created by this installer."
        info "Rename that user, or install from your own account with sudo so ScreenerBot runs as you."
        info "Nothing was changed."
        return 1
    fi
}

ensure_service_user() {
    local name="$1"
    check_service_user "$name" || return 1
    getent passwd "$name" >/dev/null && return 0
    useradd --system --create-home --home-dir "/home/${name}" --shell "$(nologin_shell)" \
        --comment "$SERVICE_USER_COMMENT" --user-group "$name" </dev/null || {
        err "Could not create the system user ${name}"
        return 1
    }
    chmod 0700 "/home/${name}"
    ok "Created the system user ${name} to run ScreenerBot (it cannot log in)"
}

# Before the service first creates ~/.local/share/ScreenerBot, records which of its
# parents do not exist yet, so uninstall --purge removes exactly those it caused.
record_data_parents() {
    local home dir
    [ -f "$CREATED_DIRS_FILE" ] && return 0
    home=$(user_home "$1")
    [ -n "$home" ] && [ -d "$home" ] && [ ! -e "${home}/.local/share/ScreenerBot" ] || return 0
    for dir in "${home}/.local" "${home}/.local/share"; do
        [ -e "$dir" ] || printf '%s\n' "$dir"
    done >"$CREATED_DIRS_FILE"
    chmod 0600 "$CREATED_DIRS_FILE"
}

# Removes the recorded parents, deepest first, while they are still empty.
remove_recorded_data_parents() {
    local dir
    [ -f "$CREATED_DIRS_FILE" ] || return 0
    while IFS= read -r dir; do
        is_safe_path "$dir" && [ -d "$dir" ] && [ ! -L "$dir" ] && rmdir "$dir" 2>/dev/null
    done < <(LC_ALL=C sort -r "$CREATED_DIRS_FILE")
    return 0
}

is_safe_unit_word() {
    [[ "$1" =~ ^[A-Za-z_][A-Za-z0-9_.-]*$ ]]
}

is_safe_path() {
    [[ "$1" =~ ^/[A-Za-z0-9/_.-]+$ ]] && [[ "$1" != *..* ]]
}

# =============================================================================
# Command output from the local dashboard
# =============================================================================

# The --port the unit passes to ScreenerBot, if any.
unit_port() {
    sed -n 's/^ExecStart=.* --port \([0-9]\{1,5\}\)$/\1/p' "${1:-$SERVICE_FILE}" 2>/dev/null | tail -n 1
}

# Reads one value from a section of the service's config.toml.
config_value() {
    local section="$1" key="$2" file
    file="$(service_data_dir)/data/config.toml"
    [ -r "$file" ] || return 1
    awk -v section="[${section}]" -v key="$key" '
        { line = $0; sub(/^[[:space:]]+/, "", line) }
        line ~ /^\[/ { gsub(/[[:space:]]/, "", line); in_section = (line == section); next }
        in_section && line ~ /=/ {
            name = line; sub(/[[:space:]]*=.*/, "", name)
            if (name == key) {
                value = line; sub(/^[^=]*=[[:space:]]*/, "", value); sub(/[[:space:]]*(#.*)?$/, "", value)
                gsub(/^"|"$/, "", value); print value; exit
            }
        }' "$file"
}

runtime_port() {
    local file pid port main_pid
    file="$(service_data_dir)/data/agent-runtime.json"
    [ -r "$file" ] && command -v jq >/dev/null 2>&1 || return 1
    port=$(jq -r '.port // empty' "$file" 2>/dev/null)
    pid=$(jq -r '.pid // empty' "$file" 2>/dev/null)
    main_pid=$(systemctl show -p MainPID --value "$SERVICE_NAME" 2>/dev/null)
    [[ "$port" =~ ^[0-9]{1,5}$ ]] || return 1
    [ -n "$pid" ] && [ "$pid" = "$main_pid" ] || return 1
    printf '%s' "$port"
}

# The port the dashboard listens on. A --port in the unit wins; otherwise the
# running process reports its own port. The configured port applies only after
# setup, so it is the last guess before the default.
dashboard_port() {
    local port
    port=$(unit_port)
    [ -n "$port" ] || port=$(runtime_port)
    if [ -z "$port" ]; then
        port=$(config_value webserver port)
        [[ "$port" =~ ^[0-9]{1,5}$ ]] || port=""
    fi
    printf '%s' "${port:-$DEFAULT_PORT}"
}

# Calls the local dashboard. Sets API_STATUS and API_BODY. A body, when given,
# travels on stdin so secrets never appear in a process list.
local_api() {
    local method="$1" path="$2" body="${3:-}" port out
    local args=(-sS --max-time 20 -X "$method" -H 'Accept: application/json' -w '\n%{http_code}')
    port=$(dashboard_port)
    [ -n "$body" ] || body='{}'
    [ -n "$API_COOKIE_JAR" ] && args+=(-b "$API_COOKIE_JAR" -c "$API_COOKIE_JAR")
    if [ "$method" = POST ]; then
        args+=(-H 'Content-Type: application/json' --data-binary @-)
        out=$(printf '%s' "$body" | curl "${args[@]}" "http://127.0.0.1:${port}${path}" 2>/dev/null)
    else
        out=$(curl "${args[@]}" "http://127.0.0.1:${port}${path}" 2>/dev/null </dev/null)
    fi
    if [ -z "$out" ]; then
        API_STATUS="000"
        API_BODY=""
        return 1
    fi
    API_STATUS=${out##*$'\n'}
    API_BODY=${out%$'\n'*}
    [[ "$API_STATUS" =~ ^2 ]]
}

api_error_text() {
    local message details
    message=$(jq -r '.error.message // .message // empty' <<<"$API_BODY" 2>/dev/null)
    details=$(jq -r '.error.details // empty' <<<"$API_BODY" 2>/dev/null)
    if [ -n "$message" ] && [ -n "$details" ]; then
        printf '%s (%s)' "$message" "$details"
    elif [ -n "$message" ]; then
        printf '%s' "$message"
    else
        printf 'HTTP %s' "$API_STATUS"
    fi
}

# Encodes a string as a JSON literal. Control characters are refused, not escaped.
json_string() {
    local value="$1"
    [[ "$value" != *[[:cntrl:]]* ]] || return 1
    value=${value//\\/\\\\}
    value=${value//\"/\\\"}
    printf '"%s"' "$value"
}

service_active() {
    systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null
}

# Logs in to a password-protected dashboard for this run only. The session
# cookie lives in a private temporary file that is removed on exit.
dashboard_login_if_needed() {
    local auth_enabled authenticated totp_enabled password password_json totp="" body
    local_api GET /api/auth/status || return 0
    auth_enabled=$(jq -r '.auth_enabled // false' <<<"$API_BODY")
    authenticated=$(jq -r '.authenticated // false' <<<"$API_BODY")
    [ "$auth_enabled" = true ] && [ "$authenticated" != true ] || return 0
    totp_enabled=$(jq -r '.totp_enabled // false' <<<"$API_BODY")
    if [ "$HAS_TTY" != 1 ]; then
        err "The dashboard is protected by a password; run this from a terminal to enter it."
        return 1
    fi
    API_COOKIE_JAR=$(mktemp /tmp/screenerbot-manager.XXXXXX) || return 1
    register_cleanup "$API_COOKIE_JAR"
    password=$(ask_secret "Dashboard password")
    if [ "$totp_enabled" = true ]; then
        totp=$(ask "Two-factor code from your authenticator app")
    fi
    password_json=$(json_string "$password") || {
        err "Passwords cannot contain control characters."
        return 1
    }
    body="{\"password\":${password_json}}"
    if [ -n "$totp" ]; then
        [[ "$totp" =~ ^[0-9]{6,8}$ ]] || {
            err "The two-factor code is 6 to 8 digits."
            return 1
        }
        body="{\"password\":${password_json},\"totp_code\":\"${totp}\"}"
    fi
    if ! local_api POST /api/auth/login "$body" || [ "$(jq -r '.success // false' <<<"$API_BODY")" != true ]; then
        err "The dashboard did not accept that password: $(api_error_text)"
        return 1
    fi
    ok "Logged in to the dashboard for this command"
}

# Open positions as the trader reports them. Before the first setup the status API
# answers INITIALIZATION_REQUIRED; with no config file yet no wallet was ever set
# up, so nothing can be open. Any other unreadable state is an error, never zero.
open_positions_count() {
    local count
    if local_api GET /api/status; then
        count=$(jq -r '.open_positions // empty' <<<"$API_BODY" 2>/dev/null)
        [[ "$count" =~ ^[0-9]+$ ]] || return 1
        printf '%s' "$count"
        return 0
    fi
    [ "$(jq -r '.error.code // empty' <<<"$API_BODY" 2>/dev/null)" = INITIALIZATION_REQUIRED ] || return 1
    local_api GET /api/initialization/status || return 1
    [ "$(jq -r '.config_exists' <<<"$API_BODY" 2>/dev/null)" = false ] || return 1
    printf '0'
}

# The listening addresses of the service process, one per line.
service_listen_addresses() {
    local pid
    pid=$(systemctl show -p MainPID --value "$SERVICE_NAME" 2>/dev/null)
    [ -n "$pid" ] && [ "$pid" != 0 ] && command -v ss >/dev/null 2>&1 || return 1
    ss -Hltnp 2>/dev/null | awk -v pid="pid=${pid}," 'index($0, pid) { print $4 }'
}

is_loopback_address() {
    case "$1" in
        127.* | "[::1]":* | "::1":* | localhost:*) return 0 ;;
    esac
    return 1
}

# =============================================================================
# Release metadata (screenerbot.io) and the independent checksum (GitHub)
# =============================================================================

# GET from the site API. Sets HTTP_STATUS and HTTP_BODY.
site_get() {
    local out
    if ! out=$(curl -sS -L --proto '=https' --proto-redir '=https' --tlsv1.2 --connect-timeout 15 \
        --max-time 90 -A "$USER_AGENT" -H 'Accept: application/json' -w '\n%{http_code}' \
        "${API_BASE}$1" 2>/dev/null </dev/null); then
        HTTP_STATUS="000"
        HTTP_BODY=""
        return 1
    fi
    HTTP_STATUS=${out##*$'\n'}
    HTTP_BODY=${out%$'\n'*}
    [ "$HTTP_STATUS" = 200 ]
}

# Sets LATEST_VERSION to the newest published version.
fetch_latest_version() {
    local version
    LATEST_VERSION=""
    site_get "/releases/latest" || return 1
    version=$(jq -r 'if .success == true then .data.version // empty else empty end' <<<"$HTTP_BODY" 2>/dev/null)
    is_version "$version" || return 1
    LATEST_VERSION="$version"
}

# Sets UPDATE_VERSION and returns 0 when a newer version exists, returns 1 when
# the installed one is current and 2 when the check failed.
fetch_update_check() {
    local current="$1" platform="$2" available latest
    UPDATE_VERSION=""
    site_get "/releases/check?version=${current}&platform=${platform}" || return 2
    [ "$(jq -r '.success // false' <<<"$HTTP_BODY" 2>/dev/null)" = true ] || return 2
    available=$(jq -r '.data.updateAvailable // false' <<<"$HTTP_BODY")
    latest=$(jq -r '.data.latestVersion // empty' <<<"$HTTP_BODY")
    if [ "$available" = true ] && is_version "$latest"; then
        UPDATE_VERSION="$latest"
        return 0
    fi
    return 1
}

# Versions screenerbot.io still hosts for this platform, newest first.
hosted_versions() {
    local platform="$1"
    site_get "/releases/history?limit=25" || return 1
    jq -r --arg platform "$platform" '
        .data.releases[]? | select(.hasFiles == true)
        | select(any(.builds[]?; .platform == $platform)) | .version' <<<"$HTTP_BODY" 2>/dev/null |
        grep -E '^[0-9]{1,4}\.[0-9]{1,4}\.[0-9]{1,4}$'
}

verify_release_manifest() {
    local manifest="$1" version="$2" platform="$3"
    local filename checksum size download_url

    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || return 1
    [ "$(jq -r '.success // false' <<<"$manifest" 2>/dev/null)" = true ] || return 1
    [ "$(jq -r '.data.version // empty' <<<"$manifest")" = "$version" ] || return 1
    [ "$(jq -r '.data.platform // empty' <<<"$manifest")" = "$platform" ] || return 1

    filename=$(jq -r '.data.filename // empty' <<<"$manifest")
    checksum=$(jq -r '.data.checksum // empty' <<<"$manifest")
    size=$(jq -r '.data.fileSize // empty' <<<"$manifest")
    download_url=$(jq -r '.data.downloadUrl // empty' <<<"$manifest")

    [[ "$filename" =~ ^ScreenerBot-v${version//./\\.}-Linux-(x64|arm64)-headless\.tar\.gz$ ]] || return 1
    [[ "$checksum" =~ ^[0-9a-f]{64}$ ]] || return 1
    [[ "$size" =~ ^[1-9][0-9]*$ ]] || return 1
    [ "$size" -le 2147483648 ] || return 1
    [ "$download_url" = "/api/releases/download?version=${version}&platform=${platform}" ] || return 1
}

# Prints the one checksum SHA256SUMS lists for a file name.
checksum_from_sums() {
    local sums="$1" filename="$2" matches
    matches=$(awk -v name="$filename" '$2 == name || $2 == "*" name { print $1 }' <<<"$sums")
    [ -n "$matches" ] && [ "$(printf '%s\n' "$matches" | wc -l)" -eq 1 ] || return 1
    [[ "$matches" =~ ^[0-9a-f]{64}$ ]] || return 1
    printf '%s' "$matches"
}

github_checksum() {
    local version="$1" filename="$2" sums
    sums=$(curl -fsS -L --proto '=https' --proto-redir '=https' --tlsv1.2 --connect-timeout 15 \
        --max-time 90 -A "$USER_AGENT" "${GITHUB_RELEASE_BASE}/v${version}/SHA256SUMS" 2>/dev/null </dev/null) || return 1
    checksum_from_sums "$sums" "$filename"
}

# The download API answers with a redirect to the file. Resolving it once and
# retrying against the file itself keeps one download request per install.
resolve_download_url() {
    local api_url="$1" out code target
    out=$(curl -sS --proto '=https' --tlsv1.2 --connect-timeout 15 --max-time 60 -A "$USER_AGENT" \
        -o /dev/null -w '%{http_code} %{redirect_url}' "$api_url" 2>/dev/null </dev/null) || return 1
    code=${out%% *}
    target=${out#* }
    case "$code" in
        301 | 302 | 303 | 307 | 308)
            [[ "$target" =~ ^https://[A-Za-z0-9.-]+/ ]] || return 1
            printf '%s' "$target"
            ;;
        200) printf '%s' "$api_url" ;;
        429)
            err "screenerbot.io limits downloads per address; wait 10 minutes and try again."
            return 1
            ;;
        *) return 1 ;;
    esac
}

# Downloads with resume. A transfer slower than 1 KB/s for a minute is retried.
download_file() {
    local url="$1" dest="$2" expected_size="$3" attempt size
    local progress=(-sS)
    [ "$HAS_TTY" = 1 ] && [ -t 2 ] && progress=(--progress-bar)
    for attempt in 1 2 3 4 5; do
        if curl -fL "${progress[@]}" --proto '=https' --proto-redir '=https' --tlsv1.2 \
            --connect-timeout 20 --speed-limit 1024 --speed-time 60 -A "$USER_AGENT" \
            -C - -o "$dest" "$url" </dev/null; then
            return 0
        fi
        size=$(stat -c %s "$dest" 2>/dev/null || echo 0)
        [ "$size" = "$expected_size" ] && return 0
        [ "$attempt" -lt 5 ] || break
        warn "The download was interrupted; retrying in 5 seconds (${attempt}/5)"
        sleep 5
    done
    return 1
}

# Exactly one regular file named screenerbot: no links, paths or extra entries.
verify_headless_archive() {
    local tarball="$1"
    [ "$(tar -tzf "$tarball" 2>/dev/null)" = "screenerbot" ] || return 1
    tar -tvzf "$tarball" 2>/dev/null | grep -q '^-.* screenerbot$'
}

# =============================================================================
# Packages
# =============================================================================

ensure_state_dir() {
    install -d -m 0700 "$STATE_DIR"
}

detect_package_manager() {
    local manager
    [ -n "$PKG_MANAGER" ] && return 0
    for manager in apt-get dnf yum zypper pacman; do
        if command -v "$manager" >/dev/null 2>&1; then
            PKG_MANAGER="$manager"
            return 0
        fi
    done
    return 1
}

package_for() {
    case "$1" in
        sha256sum) echo coreutils ;;
        runuser | flock) echo util-linux ;;
        *) echo "$1" ;;
    esac
}

installed_package_list() {
    case "$PKG_MANAGER" in
        apt-get)
            # shellcheck disable=SC2016
            dpkg-query -W -f='${db:Status-Abbrev} ${Package}\n' 2>/dev/null | awk '$1 == "ii" { print $2 }'
            ;;
        dnf | yum | zypper) rpm -qa --qf '%{NAME}\n' 2>/dev/null ;;
        pacman) pacman -Qq 2>/dev/null ;;
    esac | LC_ALL=C sort -u
}

# Installs packages and records the ones that were not there before, so
# "uninstall --purge" can remove exactly those.
install_packages() {
    local before after added status
    detect_package_manager || {
        err "No supported package manager (apt, dnf, yum, zypper, pacman). Install these and run the installer again: $*"
        return 1
    }
    ensure_state_dir
    before=$(installed_package_list)
    info "Installing $* with ${PKG_MANAGER} (details in ${PACKAGE_LOG})"
    {
        case "$PKG_MANAGER" in
            apt-get)
                DEBIAN_FRONTEND=noninteractive apt-get update -q &&
                    DEBIAN_FRONTEND=noninteractive apt-get install -y -q --no-install-recommends "$@"
                ;;
            dnf) dnf install -y "$@" ;;
            yum) yum install -y "$@" ;;
            zypper) zypper --non-interactive install "$@" ;;
            pacman) pacman -S --noconfirm --needed "$@" ;;
        esac
    } >>"$PACKAGE_LOG" 2>&1 </dev/null
    status=$?
    after=$(installed_package_list)
    added=$(LC_ALL=C comm -13 <(printf '%s\n' "$before") <(printf '%s\n' "$after"))
    if [ -n "$added" ]; then
        { cat "$PACKAGES_FILE" 2>/dev/null; printf '%s\n' "$added"; } | LC_ALL=C sort -u >"${PACKAGES_FILE}.new"
        mv -f "${PACKAGES_FILE}.new" "$PACKAGES_FILE"
        chmod 0600 "$PACKAGES_FILE"
    fi
    if [ "$status" -ne 0 ]; then
        err "${PKG_MANAGER} could not install $*. Its last lines:"
        tail -n 12 "$PACKAGE_LOG" | sed 's/^/      /' >&2
        return 1
    fi
}

has_ca_bundle() {
    local bundle
    for bundle in /etc/ssl/certs/ca-certificates.crt /etc/pki/tls/certs/ca-bundle.crt \
        /etc/ssl/ca-bundle.pem /etc/ssl/cert.pem; do
        [ -s "$bundle" ] && return 0
    done
    return 1
}

ensure_tools() {
    local missing=() packages=() tool
    for tool in curl tar gzip sha256sum jq runuser flock; do
        command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
    done
    has_ca_bundle || packages+=(ca-certificates)
    for tool in ${missing[@]+"${missing[@]}"}; do
        packages+=("$(package_for "$tool")")
    done
    [ ${#packages[@]} -eq 0 ] && return 0

    step "Installing the tools the installer needs"
    install_packages "${packages[@]}" || return 1
    for tool in ${missing[@]+"${missing[@]}"}; do
        command -v "$tool" >/dev/null 2>&1 || {
            err "${tool} is still missing after installing ${packages[*]}"
            return 1
        }
    done
    ok "Installed ${packages[*]}"
}

# =============================================================================
# Checks before installing
# =============================================================================

preflight() {
    local arch glibc mem_kb cores free
    step "Checking this server"

    if ! arch=$(detect_arch); then
        err "This processor is not supported: $(uname -m) ($(getconf LONG_BIT 2>/dev/null || echo '?')-bit)."
        info "ScreenerBot servers need a 64-bit x86_64 (Intel/AMD) or aarch64 (ARM64) system. Nothing was changed."
        return 1
    fi

    glibc=$(glibc_version)
    if [ -z "$glibc" ]; then
        err "This system does not use the GNU C library (glibc), for example Alpine Linux."
        info "ScreenerBot needs glibc ${MIN_GLIBC} or newer: Ubuntu 22.04+, Debian 12+, RHEL/Alma/Rocky 9+. Nothing was changed."
        return 1
    fi
    if ! version_at_least "$glibc" "$MIN_GLIBC"; then
        err "glibc ${glibc} is too old; ScreenerBot needs ${MIN_GLIBC} or newer."
        info "Supported: Ubuntu 22.04+, Debian 12+, RHEL/Alma/Rocky 9+, Fedora 35+. Nothing was changed."
        return 1
    fi
    if ! systemd_running; then
        err "systemd is not running here, and ScreenerBot runs as a systemd service."
        info "This happens in containers without an init system, WSL without systemd, and"
        info "distributions that use another init. Nothing was changed."
        return 1
    fi
    ok "$(os_name), $(uname -m), glibc ${glibc}, systemd"

    mem_kb=$(awk '/^MemTotal:/ { print $2 }' /proc/meminfo 2>/dev/null)
    cores=$(nproc 2>/dev/null || echo 1)
    if [ "${mem_kb:-0}" -lt 3500000 ]; then
        warn "Memory $(human_size "${mem_kb:-0}"): 4 GB or more is recommended"
    else
        ok "Memory $(human_size "$mem_kb")"
    fi
    if [ "$cores" -lt 2 ]; then
        warn "${cores} CPU core: 2 or more are recommended"
    else
        ok "${cores} CPU cores"
    fi
    free=$(free_kb /home)
    if [ -n "$free" ] && [ "$free" -lt 5242880 ]; then
        warn "$(human_size "$free") free for data: 5 GB or more is recommended"
    elif [ -n "$free" ]; then
        ok "$(human_size "$free") free for data"
    fi
}

port_in_use() {
    command -v ss >/dev/null 2>&1 || return 1
    ss -Hltn 2>/dev/null | awk '{ print $4 }' | grep -qE "[:.]$1\$"
}

# Picks the port for a fresh install: --port, else 8080, else the next free one.
choose_port() {
    local port="${OPT_PORT}" candidate answer
    if [ -n "$port" ]; then
        printf '%s' "$port"
        return 0
    fi
    port_in_use "$DEFAULT_PORT" || return 0
    for candidate in $(seq 8081 8099); do
        if ! port_in_use "$candidate"; then
            warn "Port ${DEFAULT_PORT} is already used by another program on this server." >&2
            answer=$(ask "Port for the ScreenerBot dashboard" "$candidate")
            [[ "$answer" =~ ^[0-9]{2,5}$ ]] && [ "$answer" -ge 1024 ] && [ "$answer" -le 65535 ] || answer="$candidate"
            printf '%s' "$answer"
            return 0
        fi
    done
    return 0
}

# =============================================================================
# Update state
# =============================================================================

write_update_state() {
    local phase="$1" version="$2" message="$3"
    ensure_state_dir
    jq -n --arg phase "$phase" --arg version "$version" --arg message "$message" \
        --arg timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        '{phase: $phase, version: $version, message: $message, updatedAt: $timestamp}' \
        >"${UPDATE_STATE_FILE}.part" 2>/dev/null || return 0
    chmod 0600 "${UPDATE_STATE_FILE}.part"
    mv -f "${UPDATE_STATE_FILE}.part" "$UPDATE_STATE_FILE"
}

# Moves 1.x state into the manager's directory and retires the old Telegram
# update notifier, which the app's own update notice replaces.
migrate_legacy() {
    local state
    if [ -f "$LEGACY_STATE_FILE" ] && [ ! -L "$LEGACY_STATE_FILE" ]; then
        ensure_state_dir
        [ -f "$UPDATE_STATE_FILE" ] || mv -f "$LEGACY_STATE_FILE" "$UPDATE_STATE_FILE"
        rm -f "$LEGACY_STATE_FILE"
        rmdir "$LEGACY_STATE_DIR" 2>/dev/null || true
    fi

    state=$(classify_helper_unit "$LEGACY_NOTIFIER_TIMER_FILE" "$LEGACY_NOTIFIER_TIMER_DESCRIPTION")
    if [ "$state" = legacy ]; then
        systemctl disable --now screenerbot-update.timer >/dev/null 2>&1 </dev/null || true
        rm -f "$LEGACY_NOTIFIER_TIMER_FILE" "${TIMER_STAMP_DIR}/stamp-screenerbot-update.timer"
        info "Removed the old update-notifier timer. ScreenerBot announces new versions on Telegram itself."
    fi
    state=$(classify_helper_unit "$LEGACY_NOTIFIER_SERVICE_FILE" "$LEGACY_NOTIFIER_SERVICE_DESCRIPTION")
    if [ "$state" = legacy ]; then
        systemctl stop screenerbot-update.service >/dev/null 2>&1 </dev/null || true
        rm -f "$LEGACY_NOTIFIER_SERVICE_FILE"
        rm -rf "$LEGACY_NOTIFIER_STATE_DIR"
        systemctl daemon-reload </dev/null
    fi
}

# =============================================================================
# Installing a release
# =============================================================================

render_service_unit() {
    local user="$1" group="$2" home="$3" port="$4" exec_start="$BINARY_PATH"
    [ -n "$port" ] && exec_start="${BINARY_PATH} --port ${port}"
    cat <<EOF
${UNIT_MARKER}
[Unit]
Description=ScreenerBot - Automated Solana Trading Bot
Documentation=https://screenerbot.io/docs
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${user}
Group=${group}
WorkingDirectory=${home}
Environment=HOME=${home}
ExecStart=${exec_start}
Restart=always
RestartSec=10
LimitNOFILE=65536
UMask=0077
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
RestrictSUIDSGID=true
RestrictRealtime=true
LockPersonality=true

[Install]
WantedBy=multi-user.target
EOF
}

# Writes the unit when its content changed. Returns 0 when it changed.
write_service_unit() {
    local user="$1" port="$2" group home rendered
    group=$(id -gn "$user" 2>/dev/null) || {
        err "The user ${user} does not exist"
        return 2
    }
    home=$(user_home "$user")
    if ! is_safe_unit_word "$user" || ! is_safe_unit_word "$group" || ! is_safe_path "$home"; then
        err "The user ${user} or its home ${home} has characters a service file cannot carry safely"
        return 2
    fi
    rendered=$(render_service_unit "$user" "$group" "$home" "$port")
    if [ -f "$SERVICE_FILE" ] && [ "$(cat "$SERVICE_FILE")" = "$rendered" ]; then
        return 1
    fi
    printf '%s\n' "$rendered" >"${SERVICE_FILE}.new" && chmod 0644 "${SERVICE_FILE}.new" &&
        mv -f "${SERVICE_FILE}.new" "$SERVICE_FILE" || return 2
    systemctl daemon-reload </dev/null
    return 0
}

# Waits until the service's main process runs the binary at BINARY_PATH and its
# dashboard answers. The process is matched by the inode behind /proc/PID/exe;
# reading that link takes CAP_SYS_PTRACE, which root lacks in some containers,
# and the health answer's instance id ("<pid>-<start ms>") then ties the answer
# to the main process instead. A dashboard with a password answers 401 to
# everyone, which still proves it is up; the version then comes from the binary.
wait_for_dashboard() {
    local expected="$1" deadline restarts_before restarts pid exe_inode port body code version instance
    deadline=$((SECONDS + HEALTH_TIMEOUT_SECONDS))
    restarts_before=$(systemctl show -p NRestarts --value "$SERVICE_NAME" 2>/dev/null || echo 0)
    while [ "$SECONDS" -lt "$deadline" ]; do
        restarts=$(systemctl show -p NRestarts --value "$SERVICE_NAME" 2>/dev/null || echo 0)
        if [ "$((restarts - restarts_before))" -ge 3 ]; then
            return 1
        fi
        pid=$(systemctl show -p MainPID --value "$SERVICE_NAME" 2>/dev/null)
        exe_inode=""
        [ -n "$pid" ] && [ "$pid" != 0 ] && exe_inode=$(stat -L -c %i "/proc/${pid}/exe" 2>/dev/null)
        if [ -n "$pid" ] && [ "$pid" != 0 ] &&
            { [ -z "$exe_inode" ] || [ "$exe_inode" = "$(stat -c %i "$BINARY_PATH" 2>/dev/null)" ]; }; then
            port=$(dashboard_port)
            body=$(curl -sS --max-time 3 -w '\n%{http_code}' "http://127.0.0.1:${port}/api/health" 2>/dev/null </dev/null)
            code=${body##*$'\n'}
            body=${body%$'\n'*}
            version=$(jq -r '.version // empty' <<<"$body" 2>/dev/null)
            instance=$(jq -r '.instance_id // empty' <<<"$body" 2>/dev/null)
            if [ "$code" = 200 ] && [ "$version" = "$expected" ] &&
                { [ -n "$exe_inode" ] || [ "${instance%%-*}" = "$pid" ]; }; then
                return 0
            fi
            if [ "$code" = 401 ] && [ "$(installed_version)" = "$expected" ]; then
                return 0
            fi
        fi
        sleep 2
    done
    return 1
}

show_recent_logs() {
    info "Last service log lines:"
    journalctl -u "$SERVICE_NAME" -n 15 --no-pager -o cat 2>/dev/null </dev/null | sed 's/^/      /' >&2
}

restore_previous_binary() {
    [ -f "$PREVIOUS_BINARY_PATH" ] || return 1
    install -m 0755 "$PREVIOUS_BINARY_PATH" "${BINARY_PATH}.rollback" &&
        mv -f "${BINARY_PATH}.rollback" "$BINARY_PATH"
}

# Downloads, verifies and stages one release, then switches the service to it.
# Every failure before the switch leaves the installation untouched; a failure
# after it puts the previous binary back.
install_release() {
    local version="$1" service_user_name="$2" port="$3"
    write_update_state checking "$version" "Resolving release metadata"
    if install_release_steps "$version" "$service_user_name" "$port"; then
        write_update_state applied "$version" "Installed and verified"
        return 0
    fi
    write_update_state failed "$version" "Failed; the previous installation was kept or restored"
    return 1
}

install_release_steps() {
    local version="$1" run_as="$2" port="$3"
    local arch platform manifest filename checksum size github download_url stage tarball actual
    local staged_output current was_active=0 had_binary=0 rotated=0 unit_status created_dir=0

    arch=$(detect_arch) || return 1
    platform="linux-${arch}-headless"

    step "Downloading ScreenerBot v${version} for Linux ${arch}"
    if ! site_get "/releases/manifest?version=${version}&platform=${platform}"; then
        if [ "$HTTP_STATUS" = 404 ]; then
            err "screenerbot.io does not host a Linux ${arch} build of v${version}."
            info "See the versions you can install with: sudo screenerbot-manager versions"
        else
            err "Could not reach screenerbot.io (HTTP ${HTTP_STATUS}). Check this server's internet connection; nothing was changed."
        fi
        return 1
    fi
    manifest="$HTTP_BODY"
    verify_release_manifest "$manifest" "$version" "$platform" || {
        err "screenerbot.io returned incomplete release information for v${version}; nothing was changed."
        return 1
    }
    filename=$(jq -r '.data.filename' <<<"$manifest")
    checksum=$(jq -r '.data.checksum' <<<"$manifest")
    size=$(jq -r '.data.fileSize' <<<"$manifest")

    github=$(github_checksum "$version" "$filename") || {
        err "Could not read the release checksums published on GitHub (github.com)."
        info "Every download is checked against two independent sources, so the install stops"
        info "when one is unreachable. Nothing was changed; try again later."
        return 1
    }
    if [ "$github" != "$checksum" ]; then
        err "The checksum published on GitHub does not match screenerbot.io for ${filename}."
        info "Nothing was installed. Please report this to ScreenerBot support."
        return 1
    fi

    if [ ! -d "$INSTALL_DIR" ]; then
        install -d -m 0755 "$INSTALL_DIR" || return 1
        created_dir=1
    fi
    actual=$(free_kb "$INSTALL_DIR")
    if [ -n "$actual" ] && [ "$actual" -lt $((size * 8 / 1024)) ]; then
        err "Not enough free space in ${INSTALL_DIR}: $(human_size "$actual") free, about $(human_size $((size * 8 / 1024))) needed."
        [ "$created_dir" = 1 ] && rmdir "$INSTALL_DIR" 2>/dev/null
        return 1
    fi
    stage=$(mktemp -d "${INSTALL_DIR}/.staging.XXXXXX") || return 1
    register_cleanup "$stage"
    tarball="${stage}/${filename}"

    write_update_state downloading "$version" "Downloading"
    download_url=$(resolve_download_url "${API_BASE}/releases/download?version=${version}&platform=${platform}") || {
        err "screenerbot.io did not hand out the download for v${version}; nothing was changed."
        run_cleanup
        [ "$created_dir" = 1 ] && rmdir "$INSTALL_DIR" 2>/dev/null
        return 1
    }
    if ! download_file "$download_url" "$tarball" "$size"; then
        err "The download failed (network or disk). Nothing was changed; run the same command again to retry."
        run_cleanup
        [ "$created_dir" = 1 ] && rmdir "$INSTALL_DIR" 2>/dev/null
        return 1
    fi

    write_update_state verifying "$version" "Verifying"
    actual=$(stat -c %s "$tarball")
    if [ "$actual" != "$size" ]; then
        err "The download has the wrong size (${actual} bytes, expected ${size}); nothing was changed."
        run_cleanup
        [ "$created_dir" = 1 ] && rmdir "$INSTALL_DIR" 2>/dev/null
        return 1
    fi
    actual=$(sha256sum "$tarball" | awk '{ print $1 }')
    if [ "$actual" != "$checksum" ]; then
        err "The download does not match its published checksum; nothing was changed."
        run_cleanup
        [ "$created_dir" = 1 ] && rmdir "$INSTALL_DIR" 2>/dev/null
        return 1
    fi
    ok "Checksum matches screenerbot.io and GitHub"
    if ! verify_headless_archive "$tarball" ||
        ! tar -xzf "$tarball" -C "$stage" --no-same-owner --no-same-permissions 2>/dev/null; then
        err "The release archive has an unexpected layout; nothing was changed."
        run_cleanup
        [ "$created_dir" = 1 ] && rmdir "$INSTALL_DIR" 2>/dev/null
        return 1
    fi
    chmod 0755 "${stage}/screenerbot"
    staged_output=$(timeout 15 "${stage}/screenerbot" --version 2>&1 </dev/null)
    if [ "$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' <<<"$staged_output" | head -n 1)" != "$version" ]; then
        err "The downloaded program does not run on this server:"
        printf '%s\n' "$staged_output" | head -n 5 | sed 's/^/      /' >&2
        run_cleanup
        [ "$created_dir" = 1 ] && rmdir "$INSTALL_DIR" 2>/dev/null
        return 1
    fi
    ok "It runs on this server (ScreenerBot v${version})"

    write_update_state applying "$version" "Switching the service to the new version"
    step "Setting up the service"
    ensure_service_user "$run_as" || {
        run_cleanup
        [ "$created_dir" = 1 ] && rmdir "$INSTALL_DIR" 2>/dev/null
        return 1
    }
    record_data_parents "$run_as"

    current=$(installed_version 2>/dev/null || true)
    [ -f "$BINARY_PATH" ] && had_binary=1
    service_active && was_active=1

    install -m 0755 "${stage}/screenerbot" "${BINARY_PATH}.new" || {
        err "Could not write ${BINARY_PATH}.new"
        run_cleanup
        return 1
    }
    run_cleanup
    if [ "$had_binary" = 1 ] && [ -n "$current" ] && [ "$current" != "$version" ]; then
        install -m 0755 "$BINARY_PATH" "${PREVIOUS_BINARY_PATH}.new" &&
            mv -f "${PREVIOUS_BINARY_PATH}.new" "$PREVIOUS_BINARY_PATH" && rotated=1
    fi
    mv -f "${BINARY_PATH}.new" "$BINARY_PATH" || return 1
    printf 'Created by screenerbot-manager %s. Remove with: sudo screenerbot-manager uninstall\n' \
        "$SCRIPT_VERSION" >"$INSTALL_MARKER"
    ln -snf "$BINARY_PATH" "$SYMLINK_PATH"

    write_service_unit "$run_as" "$port"
    unit_status=$?
    if [ "$unit_status" -eq 2 ]; then
        return 1
    fi
    systemctl enable "$SERVICE_NAME" >/dev/null 2>&1 </dev/null
    if [ "$was_active" = 1 ] || [ "$unit_status" -eq 0 ] || [ "$current" != "$version" ]; then
        systemctl restart "$SERVICE_NAME" </dev/null
    else
        systemctl start "$SERVICE_NAME" </dev/null
    fi

    if wait_for_dashboard "$version"; then
        ok "Service running and dashboard answering on 127.0.0.1:$(dashboard_port)"
        return 0
    fi

    err "ScreenerBot v${version} did not start correctly."
    show_recent_logs
    if [ "$rotated" = 1 ] && restore_previous_binary; then
        warn "Putting back v${current}"
        systemctl restart "$SERVICE_NAME" </dev/null
        if wait_for_dashboard "$current"; then
            ok "v${current} is running again"
        else
            err "v${current} did not start either; see: sudo screenerbot-manager logs"
        fi
    elif [ "$had_binary" = 0 ]; then
        info "The installation is in place. Check the logs with: sudo screenerbot-manager logs"
    fi
    return 1
}

# =============================================================================
# The manager copy at /usr/local/bin/screenerbot-manager
# =============================================================================

script_version_of() {
    sed -n 's/^readonly SCRIPT_VERSION="\([0-9]\{1,4\}\.[0-9]\{1,4\}\.[0-9]\{1,4\}\)"$/\1/p' "$1" | head -n 1
}

# Accepts only a complete copy of this script. Prints its version.
validate_manager_script() {
    local file="$1" version
    [ "$(head -n 1 "$file" 2>/dev/null)" = "#!/bin/bash" ] || return 1
    bash -n "$file" 2>/dev/null || return 1
    grep -qF 'SCREENERBOT_INSTALLER_LIBRARY_ONLY' "$file" || return 1
    version=$(script_version_of "$file")
    is_version "$version" || return 1
    printf '%s' "$version"
}

fetch_manager_script() {
    local dest="$1" protocol="=https"
    [ "$SCRIPT_URL_IS_LOCAL" = 1 ] && protocol="=http"
    curl -fsS -L --proto "$protocol" --proto-redir '=https' --connect-timeout 15 --max-time 60 \
        -A "$USER_AGENT" -o "$dest" "$SCRIPT_URL" 2>/dev/null </dev/null
}

install_manager_file() {
    install -m 0755 "$1" "${MANAGER_PATH}.new" && mv -f "${MANAGER_PATH}.new" "$MANAGER_PATH"
}

# True when sudo runs commands with a PATH that leaves out /usr/local/bin, so
# "sudo screenerbot-manager" would answer "command not found".
sudo_path_misses_manager() {
    local path
    command -v sudo >/dev/null 2>&1 || return 1
    path=$(sudo -n env 2>/dev/null </dev/null | sed -n 's/^PATH=//p')
    [ -n "$path" ] && [[ ":${path}:" != *":${MANAGER_PATH%/*}:"* ]]
}

# Makes "sudo screenerbot-manager" work where sudo's secure_path omits
# /usr/local/bin, with a link in /usr/bin that only this installer creates.
link_manager_for_sudo() {
    [ "$(classify_symlink "$SUDO_LINK_PATH" "$MANAGER_PATH")" = absent ] || return 0
    sudo_path_misses_manager || return 0
    if ln -s "$MANAGER_PATH" "$SUDO_LINK_PATH" 2>/dev/null; then
        ok "Linked ${SUDO_LINK_PATH}, because sudo on this system does not search ${MANAGER_PATH%/*}"
    else
        warn "sudo on this system does not search ${MANAGER_PATH%/*}; run the manager as: sudo ${MANAGER_PATH}"
    fi
}

# Installs this exact script as the manager. A streamed run has no file of its
# own, so it installs the copy screenerbot.io serves.
install_manager() {
    local temp version
    if [ "$SELF_PATH" = "$MANAGER_PATH" ]; then
        link_manager_for_sudo
        return 0
    fi
    if [ -n "$SELF_PATH" ]; then
        install_manager_file "$SELF_PATH" || return 1
        ok "Manager installed: ${MANAGER_PATH} (v${SCRIPT_VERSION})"
        link_manager_for_sudo
        return 0
    fi
    temp=$(mktemp /tmp/screenerbot-manager.XXXXXX) || return 1
    register_cleanup "$temp"
    if fetch_manager_script "$temp" && version=$(validate_manager_script "$temp") &&
        [ "$(version_compare "$version" "$SCRIPT_VERSION")" != -1 ]; then
        install_manager_file "$temp" || return 1
        ok "Manager installed: ${MANAGER_PATH} (v${version})"
        link_manager_for_sudo
        return 0
    fi
    warn "Could not install ${MANAGER_PATH}; run the install command again to add it."
    return 1
}

# =============================================================================
# Commands
# =============================================================================

require_systemd() {
    systemd_running || die "systemd is not running on this system, so there is no ScreenerBot service to manage."
}

require_installed() {
    local state
    state=$(classify_service_unit)
    if [ "$state" = foreign ]; then
        die "${SERVICE_FILE} belongs to a different program; this manager does not touch it."
    fi
    if [ ! -x "$BINARY_PATH" ] || [ "$state" = absent ]; then
        die "ScreenerBot is not installed. Install it with: curl -fsSL ${DEFAULT_SCRIPT_URL} | sudo bash"
    fi
}

# The SSH_CONNECTION of the session that started this run. sudo drops it from
# the environment, so it is read from the nearest ancestor process that has it.
ssh_connection() {
    local pid=$$ depth value
    if [ -n "${SSH_CONNECTION:-}" ]; then
        printf '%s' "$SSH_CONNECTION"
        return 0
    fi
    for depth in 1 2 3 4 5 6 7 8; do
        pid=$(awk '/^PPid:/ { print $2 }' "/proc/${pid}/status" 2>/dev/null)
        [ -n "$pid" ] && [ "$pid" -gt 1 ] || return 1
        value=$(tr '\0' '\n' <"/proc/${pid}/environ" 2>/dev/null | sed -n 's/^SSH_CONNECTION=//p' | head -n 1)
        if [ -n "$value" ]; then
            printf '%s' "$value"
            return 0
        fi
        : "$depth"
    done
    return 1
}

is_private_address() {
    case "$1" in
        10.* | 127.* | 192.168.* | 169.254.* | 172.1[6-9].* | 172.2[0-9].* | 172.3[01].* | \
            100.6[4-9].* | 100.[7-9][0-9].* | 100.1[01][0-9].* | 100.12[0-7].* | ::1 | fe80:* | fc* | fd*) return 0 ;;
    esac
    return 1
}

# Prints the SSH tunnel command for the dashboard, with this server's public
# address and SSH port when the current session reveals them.
ssh_tunnel_hint() {
    local port="${1:-$DEFAULT_PORT}" login="${SUDO_USER:-root}" server="YOUR_SERVER_ADDRESS" ssh_port=""
    local client_ip client_port server_ip server_port
    if read -r client_ip client_port server_ip server_port <<<"$(ssh_connection)" && [ -n "${server_ip:-}" ]; then
        is_private_address "$server_ip" || server="$server_ip"
        [ -n "${server_port:-}" ] && [ "$server_port" != 22 ] && ssh_port=" -p ${server_port}"
    fi
    : "${client_ip:-}" "${client_port:-}"
    show_command "ssh -N -L ${port}:127.0.0.1:${port}${ssh_port} ${login}@${server}"
    [ "$server" = YOUR_SERVER_ADDRESS ] && info "(YOUR_SERVER_ADDRESS is the address you use to connect to this server with ssh)"
    return 0
}

print_next_steps() {
    local version="$1" port
    port=$(dashboard_port)
    step "ScreenerBot v${version} is running"
    info "It starts by itself whenever the server boots. The dashboard listens only on this"
    info "server's 127.0.0.1, so nobody on the internet can reach it."
    say ""
    info "1. On your own computer, open a terminal (Terminal on macOS and Linux, PowerShell on"
    info "   Windows) and keep this running:"
    ssh_tunnel_hint "$port"
    info "   Then open http://localhost:${port} in your browser."
    info "2. In the dashboard, add your wallet and RPC to start trading, or choose Explore Mode"
    info "   to look around first."
    info "3. Optional, to sign in to your ScreenerBot account from this server:"
    show_command "sudo screenerbot-manager signin"
    say ""
    info "Status, updates, backups and everything else:"
    show_command "sudo screenerbot-manager"
}

cmd_install() {
    local requested="${1:-}" force="${2:-0}" latest current run_as port relation
    preflight || return 1
    guard_ownership
    run_as=$(choose_service_user)
    check_service_user "$run_as" || return 1
    ensure_tools || return 1
    ensure_state_dir
    migrate_legacy

    fetch_latest_version || {
        err "Could not reach screenerbot.io to find the latest version (HTTP ${HTTP_STATUS}). Check the server's internet connection."
        return 1
    }
    latest="$LATEST_VERSION"
    if [ -z "$requested" ]; then
        requested="$latest"
    elif ! is_version "$requested"; then
        err "\"${requested}\" is not a version. Versions look like ${latest}."
        return 1
    fi

    current=$(installed_version 2>/dev/null || true)
    if [ "$(classify_service_unit)" = absent ]; then
        port=$(choose_port)
    else
        port=$(unit_port)
        [ -n "$OPT_PORT" ] && port="$OPT_PORT"
    fi

    if [ -n "$current" ] && [ "$current" = "$requested" ] && [ "$force" != 1 ]; then
        step "ScreenerBot v${current} is already installed"
        ensure_service_user "$run_as" || return 1
        printf 'Created by screenerbot-manager %s. Remove with: sudo screenerbot-manager uninstall\n' \
            "$SCRIPT_VERSION" >"$INSTALL_MARKER"
        ln -snf "$BINARY_PATH" "$SYMLINK_PATH"
        write_service_unit "$run_as" "$port"
        case $? in
            0)
                systemctl enable "$SERVICE_NAME" >/dev/null 2>&1 </dev/null
                systemctl restart "$SERVICE_NAME" </dev/null
                ok "Service file brought up to date"
                ;;
            1)
                systemctl enable "$SERVICE_NAME" >/dev/null 2>&1 </dev/null
                service_active || systemctl start "$SERVICE_NAME" </dev/null
                ;;
            *) return 1 ;;
        esac
        if ! wait_for_dashboard "$current"; then
            err "The service is not answering."
            show_recent_logs
            install_manager
            info "Check the logs with: sudo screenerbot-manager logs"
            return 1
        fi
        ok "Service running and dashboard answering on 127.0.0.1:$(dashboard_port)"
        install_manager
        if [ "$current" != "$latest" ]; then
            info "v${latest} is available: sudo screenerbot-manager update"
        fi
        print_next_steps "$current"
        return 0
    fi

    if [ -n "$current" ] && [ "$current" != "$requested" ]; then
        relation=$(version_compare "$requested" "$current")
        if [ "$relation" = -1 ]; then
            warn "v${requested} is older than the installed v${current}. An older version may not read data written by a newer one."
            confirm "Install the older v${requested} anyway?" n || {
                info "Nothing was changed. Use --yes to confirm without a terminal."
                return 1
            }
        fi
    fi

    if ! install_release "$requested" "$run_as" "$port"; then
        # A release that is in place but not answering still gets the manager,
        # so the logs and repair commands named in the failure message exist.
        [ -f "$INSTALL_MARKER" ] && install_manager
        return 1
    fi
    install_manager
    print_next_steps "$requested"
}

cmd_update() {
    local current arch latest status
    require_installed
    current=$(installed_version) || die "The installed binary does not run; reinstall with: sudo screenerbot-manager reinstall"
    arch=$(detect_arch) || die "Unsupported architecture: $(uname -m)"
    ensure_tools || return 1
    migrate_legacy
    step "Checking for a new version"
    fetch_update_check "$current" "linux-${arch}-headless"
    status=$?
    latest="$UPDATE_VERSION"
    case "$status" in
        1)
            ok "ScreenerBot v${current} is the latest version"
            notice_manager_update
            return 0
            ;;
        2)
            err "Could not check for updates (HTTP ${HTTP_STATUS}). Nothing was changed."
            return 1
            ;;
    esac
    info "v${current} -> v${latest}"
    install_release "$latest" "$(service_user)" "$(unit_port)" || return 1
    ok "Updated to v${latest}"
    notice_manager_update
}

cmd_rollback() {
    local current previous
    require_installed
    [ -f "$PREVIOUS_BINARY_PATH" ] || die "There is no previous version on this server to go back to."
    current=$(installed_version 2>/dev/null || echo unknown)
    previous=$(binary_version "$PREVIOUS_BINARY_PATH")
    [ -n "$previous" ] || die "The previous binary does not run on this server."
    confirm "Go back from v${current} to v${previous}?" y || return 1
    step "Rolling back to v${previous}"
    if ! install -m 0755 "$PREVIOUS_BINARY_PATH" "${BINARY_PATH}.new" ||
        ! install -m 0755 "$BINARY_PATH" "${PREVIOUS_BINARY_PATH}.new" ||
        ! mv -f "${BINARY_PATH}.new" "$BINARY_PATH" ||
        ! mv -f "${PREVIOUS_BINARY_PATH}.new" "$PREVIOUS_BINARY_PATH"; then
        die "Could not swap the binaries."
    fi
    systemctl restart "$SERVICE_NAME" </dev/null
    if wait_for_dashboard "$previous"; then
        write_update_state applied "$previous" "Rolled back from v${current}"
        ok "v${previous} is running. v${current} is kept; run rollback again to return to it."
        return 0
    fi
    err "v${previous} did not start."
    show_recent_logs
    return 1
}

cmd_versions() {
    local arch platform versions current latest version label
    arch=$(detect_arch) || die "Unsupported architecture: $(uname -m)"
    command -v jq >/dev/null 2>&1 || die "jq is required; install ScreenerBot first."
    platform="linux-${arch}-headless"
    versions=$(hosted_versions "$platform") || die "Could not read the version list from screenerbot.io."
    current=$(installed_version 2>/dev/null || true)
    latest=$(head -n 1 <<<"$versions")
    step "Versions you can install on this server (Linux ${arch})"
    while IFS= read -r version; do
        [ -n "$version" ] || continue
        label=""
        [ "$version" = "$latest" ] && label=" (latest)"
        [ "$version" = "$current" ] && label="${label} (installed)"
        info "v${version}${label}"
    done <<<"$versions"
    say ""
    info "Install one with: sudo screenerbot-manager install <version>"
}

remove_if_ours() {
    local file="$1" state="$2"
    if [ "$state" = ours ] || [ "$state" = legacy ]; then
        rm -f "$file"
        return 0
    fi
    [ "$state" = foreign ] && warn "Left ${file} alone: it was not created by this installer."
    return 1
}

purge_recorded_packages() {
    local packages=() name installed
    [ -f "$PACKAGES_FILE" ] || return 0
    detect_package_manager || return 0
    installed=$(installed_package_list)
    while IFS= read -r name; do
        [[ "$name" =~ ^[A-Za-z0-9][A-Za-z0-9.+_:-]*$ ]] || continue
        grep -qxF "$name" <<<"$installed" && packages+=("$name")
    done <"$PACKAGES_FILE"
    [ ${#packages[@]} -eq 0 ] && return 0
    case "$PKG_MANAGER" in
        apt-get)
            if dpkg --dry-run -P "${packages[@]}" >/dev/null 2>&1; then
                dpkg -P "${packages[@]}" >>"$PACKAGE_LOG" 2>&1 </dev/null && ok "Removed packages: ${packages[*]}" && return 0
            fi
            ;;
        dnf | yum | zypper)
            if rpm -e --test "${packages[@]}" >/dev/null 2>&1; then
                rpm -e "${packages[@]}" >>"$PACKAGE_LOG" 2>&1 </dev/null && ok "Removed packages: ${packages[*]}" && return 0
            fi
            ;;
        pacman)
            pacman -R --noconfirm "${packages[@]}" >>"$PACKAGE_LOG" 2>&1 </dev/null && ok "Removed packages: ${packages[*]}" && return 0
            ;;
    esac
    warn "Kept ${packages[*]}: other software on this server now depends on them."
    return 1
}

dedicated_user_exists() {
    getent passwd "$SERVICE_USER_NAME" >/dev/null && [ "$(user_comment "$SERVICE_USER_NAME")" = "$SERVICE_USER_COMMENT" ]
}

# True when any file, unit, state or account this manager creates is present.
manager_owns_anything() {
    local state
    state=$(classify_service_unit)
    [ "$state" = ours ] || [ "$state" = legacy ] && return 0
    state=$(classify_install_dir)
    [ "$state" = ours ] || [ "$state" = legacy ] && return 0
    [ "$(classify_symlink)" = ours ] && return 0
    [ "$(classify_manager)" = ours ] && return 0
    [ "$(classify_symlink "$SUDO_LINK_PATH" "$MANAGER_PATH")" = ours ] && return 0
    [ "$(classify_helper_unit "$AUTO_UPDATE_TIMER_FILE")" = ours ] && return 0
    [ "$(classify_helper_unit "$AUTO_UPDATE_SERVICE_FILE")" = ours ] && return 0
    [ -d "$STATE_DIR" ] && return 0
    [ "$OPT_PURGE" = 1 ] && dedicated_user_exists && return 0
    return 1
}

report_foreign_names() {
    [ "$(classify_service_unit)" = foreign ] && warn "Left ${SERVICE_FILE} alone: it belongs to a different program."
    [ "$(classify_install_dir)" = foreign ] && warn "Left ${INSTALL_DIR} alone: it holds files this installer did not create."
    [ "$(classify_symlink)" = foreign ] && warn "Left ${SYMLINK_PATH} alone: it was not created by this installer."
    [ "$(classify_manager)" = foreign ] && warn "Left ${MANAGER_PATH} alone: it was not created by this installer."
    return 0
}

cmd_uninstall() {
    local unit_state data_dir="" user="" data_kb added_packages="" entry name keep_state=0
    require_systemd
    unit_state=$(classify_service_unit)
    if [ "$unit_state" = ours ] || [ "$unit_state" = legacy ]; then
        user=$(service_user)
        data_dir=$(service_data_dir)
    elif dedicated_user_exists; then
        user="$SERVICE_USER_NAME"
        data_dir="$(user_home "$user")/.local/share/ScreenerBot"
    fi
    [ -f "$PACKAGES_FILE" ] && added_packages=$(tr '\n' ' ' <"$PACKAGES_FILE")
    if ! manager_owns_anything; then
        report_foreign_names
        ok "Nothing on this server was installed by this manager; nothing to remove."
        return 0
    fi

    step "Uninstalling ScreenerBot"
    if [ "$OPT_PURGE" = 1 ]; then
        warn "--purge also deletes ${data_dir:-the service data}: wallets, keys, settings and history."
        warn "Funds in a wallet whose key exists only there are lost. Make a backup first:"
        show_command "sudo screenerbot-manager backup"
        if [ "$ASSUME_YES" != 1 ]; then
            [ "$HAS_TTY" = 1 ] || die "Without a terminal, confirm with: sudo screenerbot-manager uninstall --purge --yes"
            [ "$(ask "Type DELETE to remove ScreenerBot and all its data")" = DELETE ] || {
                info "Nothing was changed."
                return 1
            }
        fi
    else
        info "Removes the service, the program and this manager. Your wallets and settings${data_dir:+ in}"
        info "${data_dir:-} stay on the server."
        if [ "$HAS_TTY" != 1 ] && [ "$ASSUME_YES" != 1 ]; then
            die "Without a terminal, confirm with: sudo screenerbot-manager uninstall --yes"
        fi
        confirm "Uninstall ScreenerBot?" n || {
            info "Nothing was changed."
            return 1
        }
    fi

    if [ "$unit_state" = ours ] || [ "$unit_state" = legacy ]; then
        systemctl disable --now "$SERVICE_NAME" >/dev/null 2>&1 </dev/null
        rm -f "$SERVICE_FILE"
        systemctl daemon-reload </dev/null
        systemctl reset-failed "$SERVICE_NAME" >/dev/null 2>&1 </dev/null
        ok "Stopped and removed the service"
    elif [ "$unit_state" = foreign ]; then
        warn "Left ${SERVICE_FILE} alone: it belongs to a different program."
    fi
    remove_auto_update_units
    migrate_legacy >/dev/null
    systemctl daemon-reload </dev/null

    remove_if_ours "$SYMLINK_PATH" "$(classify_symlink)" >/dev/null
    case "$(classify_install_dir)" in
        ours | legacy)
            for name in "${INSTALL_DIR_FILES[@]}"; do
                rm -f "${INSTALL_DIR}/${name}"
            done
            for entry in "${INSTALL_DIR}"/.staging.*; do
                [ -d "$entry" ] && rm -rf -- "$entry"
            done
            if rmdir "$INSTALL_DIR" 2>/dev/null; then
                ok "Removed ${INSTALL_DIR}"
            else
                warn "Kept ${INSTALL_DIR}: it contains files this installer did not create."
            fi
            ;;
        foreign) warn "Left ${INSTALL_DIR} alone: it holds files this installer did not create." ;;
    esac

    if [ "$OPT_PURGE" = 1 ]; then
        if [ -n "$data_dir" ] && [ -d "$data_dir" ] && [ ! -L "$data_dir" ]; then
            rm -rf -- "$data_dir"
            ok "Deleted ${data_dir}"
        fi
        remove_recorded_data_parents
        if [ "$user" = "$SERVICE_USER_NAME" ] && dedicated_user_exists; then
            userdel -r "$user" >/dev/null 2>&1 </dev/null
            getent group "$user" >/dev/null && groupdel "$user" >/dev/null 2>&1
            ok "Removed the system user ${user}"
        fi
        purge_recorded_packages || keep_state=1
    fi

    if [ "$keep_state" = 0 ]; then
        rm -f "$UPDATE_STATE_FILE" "${UPDATE_STATE_FILE}.part" "$PACKAGE_LOG"
        [ "$OPT_PURGE" = 1 ] && rm -f "$PACKAGES_FILE" "$CREATED_DIRS_FILE"
    fi
    rmdir "$STATE_DIR" 2>/dev/null || true
    remove_if_ours "$MANAGER_PATH" "$(classify_manager)" >/dev/null
    [ "$(classify_symlink "$SUDO_LINK_PATH" "$MANAGER_PATH")" = ours ] && rm -f "$SUDO_LINK_PATH"
    rm -f "$LOCK_FILE"
    ok "ScreenerBot is uninstalled"

    if [ "$OPT_PURGE" != 1 ]; then
        say ""
        info "Kept on purpose:"
        if [ -n "$data_dir" ] && [ -d "$data_dir" ]; then
            data_kb=$(du -sk "$data_dir" 2>/dev/null | awk '{ print $1 }')
            info "  ${data_dir} ($(human_size "${data_kb:-0}")): wallets, settings, history"
        fi
        if [ "$user" = "$SERVICE_USER_NAME" ] && dedicated_user_exists; then
            info "  the system user ${user}, which owns that data"
        fi
        [ -n "$added_packages" ] && info "  packages added during install: ${added_packages}"
        info "Reinstalling later picks all of it up again. To delete everything instead, run the"
        info "installer again and then: sudo screenerbot-manager uninstall --purge"
    fi
}

# Where backups go: the administrator's home, readable only by them.
backup_dir() {
    local home=""
    if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != root ]; then
        home=$(user_home "$SUDO_USER")
    fi
    printf '%s' "${home:-/root}"
}

backup_owner() {
    if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != root ] && getent passwd "$SUDO_USER" >/dev/null; then
        printf '%s' "$SUDO_USER"
    else
        printf 'root'
    fi
}

# Runs a command as the service user, so file access follows that user's rights
# and a link planted in the data directory cannot redirect a root write.
as_service_user() {
    local user="$1"
    shift
    if [ "$user" = root ]; then
        "$@"
    else
        runuser -u "$user" -- "$@"
    fi
}

# Writes data_dir into a new archive; the archive is created 0600 by root.
write_backup_archive() {
    local user="$1" data_dir="$2" dest="$3" parent name
    parent=$(dirname "$data_dir")
    name=$(basename "$data_dir")
    (
        umask 077
        as_service_user "$user" tar -czf - -C "$parent" --exclude="${name}/logs" "$name" >"$dest"
    )
}

cmd_backup() {
    local user data_dir dest owner stopped=0 positions size_kb
    require_installed
    user=$(service_user)
    data_dir=$(service_data_dir)
    [ -d "$data_dir" ] || die "There is no data to back up yet (${data_dir} does not exist)."
    dest="$(backup_dir)/screenerbot-backup-$(date +%Y%m%d-%H%M%S).tar.gz"
    owner=$(backup_owner)

    step "Backing up ${data_dir}"
    if service_active; then
        positions=$(open_positions_count 2>/dev/null || true)
        [ -n "$positions" ] && [ "$positions" != 0 ] && warn "${positions} position(s) are open; they are not managed while the service is stopped."
        if confirm "Stop ScreenerBot for the few seconds the backup takes, so the databases are consistent?" y; then
            systemctl stop "$SERVICE_NAME" </dev/null && stopped=1
        else
            warn "Backing up while the service runs: databases being written may be inconsistent."
        fi
    fi
    if write_backup_archive "$user" "$data_dir" "${dest}.part" && tar -tzf "${dest}.part" >/dev/null 2>&1; then
        chown "$owner:" "${dest}.part" 2>/dev/null
        mv -f "${dest}.part" "$dest"
    else
        rm -f "${dest}.part"
        [ "$stopped" = 1 ] && systemctl start "$SERVICE_NAME" </dev/null
        die "The backup failed; nothing was written."
    fi
    [ "$stopped" = 1 ] && systemctl start "$SERVICE_NAME" </dev/null
    size_kb=$(du -k "$dest" | awk '{ print $1 }')
    ok "Saved ${dest} ($(human_size "$size_kb"), readable only by ${owner})"
    info "It holds your wallet keys. Copy it somewhere safe, for example:"
    show_command "scp ${owner}@YOUR_SERVER_ADDRESS:${dest} ."
}

# A backup may contain only regular files and directories under ScreenerBot/.
validate_backup_archive() {
    local file="$1" names listing name
    listing=$(tar -tzvf "$file" 2>/dev/null) || return 1
    [ -n "$listing" ] || return 1
    awk '{ kind = substr($1, 1, 1); if (kind != "-" && kind != "d") bad = 1 } END { exit bad }' <<<"$listing" || return 1
    names=$(tar -tzf "$file" 2>/dev/null) || return 1
    while IFS= read -r name; do
        case "$name" in
            ScreenerBot/ | ScreenerBot/*) ;;
            *) return 1 ;;
        esac
        case "/${name%/}/" in
            */../* | */./* | *//*) return 1 ;;
        esac
    done <<<"$names"
}

choose_backup_file() {
    local dir files=() file index=1 choice
    dir=$(backup_dir)
    while IFS= read -r file; do
        files+=("$file")
    done < <(find "$dir" -maxdepth 1 -type f \( -name 'screenerbot-backup-*.tar.gz' -o -name 'screenerbot-pre-restore-*.tar.gz' \) 2>/dev/null | LC_ALL=C sort -r)
    if [ ${#files[@]} -eq 0 ]; then
        err "No backups found in ${dir}. Pass the file: sudo screenerbot-manager restore /path/to/backup.tar.gz"
        return 1
    fi
    [ "$HAS_TTY" = 1 ] || {
        err "Pass the backup to restore: sudo screenerbot-manager restore ${files[0]}"
        return 1
    }
    for file in "${files[@]}"; do
        info "${index}) $(basename "$file") ($(human_size "$(du -k "$file" | awk '{ print $1 }')"))"
        index=$((index + 1))
    done
    choice=$(ask "Backup to restore (number)" 1)
    [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 1 ] && [ "$choice" -le ${#files[@]} ] || {
        err "No backup with that number"
        return 1
    }
    printf '%s' "${files[$((choice - 1))]}"
}

cmd_restore() {
    local file="${1:-}" user data_dir parent was_active=0 aside safety owner
    require_installed
    user=$(service_user)
    data_dir=$(service_data_dir)
    parent=$(dirname "$data_dir")
    [ "$(basename "$data_dir")" = ScreenerBot ] || die "The data directory ${data_dir} is customised; restore it by hand."
    if [ -z "$file" ]; then
        file=$(choose_backup_file) || return 1
    fi
    [ -f "$file" ] || die "No such file: ${file}"
    validate_backup_archive "$file" || die "${file} is not a ScreenerBot backup (or contains links or paths outside ScreenerBot/)."

    step "Restoring $(basename "$file")"
    info "Your current data is saved first and moved aside, never deleted."
    if [ "$HAS_TTY" != 1 ] && [ "$ASSUME_YES" != 1 ]; then
        die "Without a terminal, confirm with: sudo screenerbot-manager restore ${file} --yes"
    fi
    confirm "Replace the data in ${data_dir} with this backup?" n || {
        info "Nothing was changed."
        return 1
    }

    service_active && was_active=1
    [ "$was_active" = 1 ] && systemctl stop "$SERVICE_NAME" </dev/null
    if [ -d "$data_dir" ]; then
        owner=$(backup_owner)
        safety="$(backup_dir)/screenerbot-pre-restore-$(date +%Y%m%d-%H%M%S).tar.gz"
        if write_backup_archive "$user" "$data_dir" "${safety}.part"; then
            chown "$owner:" "${safety}.part" 2>/dev/null
            mv -f "${safety}.part" "$safety"
            ok "Current data saved to ${safety}"
        else
            rm -f "${safety}.part"
            [ "$was_active" = 1 ] && systemctl start "$SERVICE_NAME" </dev/null
            die "Could not save the current data; nothing was changed."
        fi
        aside="${data_dir}.before-restore-$(date +%Y%m%d-%H%M%S)"
        mv "$data_dir" "$aside" || {
            [ "$was_active" = 1 ] && systemctl start "$SERVICE_NAME" </dev/null
            die "Could not move the current data aside; nothing was changed."
        }
        ok "Current data moved to ${aside}"
    fi
    as_service_user "$user" mkdir -p "$parent" || die "Could not create ${parent}"
    if ! (
        umask 077
        as_service_user "$user" tar -xzf - -C "$parent" --no-same-owner --no-same-permissions <"$file"
    ); then
        err "Extracting the backup failed."
        if [ -n "${aside:-}" ]; then
            as_service_user "$user" rm -rf -- "$data_dir"
            mv "$aside" "$data_dir" && info "Your previous data is back in place."
        fi
        [ "$was_active" = 1 ] && systemctl start "$SERVICE_NAME" </dev/null
        return 1
    fi
    ok "Restored into ${data_dir}"
    if [ "$was_active" = 1 ] || confirm "Start ScreenerBot now?" y; then
        systemctl start "$SERVICE_NAME" </dev/null
        if wait_for_dashboard "$(installed_version)"; then
            ok "ScreenerBot is running with the restored data"
        else
            err "ScreenerBot did not start with the restored data."
            show_recent_logs
            return 1
        fi
    fi
}

cmd_signin() {
    local signed_in name user_code verification complete device_code interval expires deadline state
    require_installed
    service_active || die "ScreenerBot is not running. Start it first: sudo screenerbot-manager start"
    command -v jq >/dev/null 2>&1 || die "jq is required."
    dashboard_login_if_needed || return 1

    if ! local_api GET /api/account/status; then
        die "The dashboard did not answer on 127.0.0.1:$(dashboard_port): $(api_error_text)"
    fi
    signed_in=$(jq -r '.data.signed_in // false' <<<"$API_BODY")
    if [ "$signed_in" = true ]; then
        name=$(jq -r '.data.name // .data.email // "your account"' <<<"$API_BODY")
        ok "Already signed in as ${name}"
        return 0
    fi

    if ! local_api POST /api/account/signin/device '{}'; then
        die "Could not start the sign-in: $(api_error_text)"
    fi
    user_code=$(jq -r '.data.user_code // empty' <<<"$API_BODY")
    verification=$(jq -r '.data.verification_uri // empty' <<<"$API_BODY")
    complete=$(jq -r '.data.verification_uri_complete // empty' <<<"$API_BODY")
    device_code=$(jq -r '.data.device_code // empty' <<<"$API_BODY")
    interval=$(jq -r '.data.interval // 5' <<<"$API_BODY")
    expires=$(jq -r '.data.expires_in // 900' <<<"$API_BODY")
    [[ "$user_code" =~ ^[A-Za-z0-9-]{4,16}$ ]] && [[ "$device_code" =~ ^[A-Za-z0-9._~-]{8,256}$ ]] &&
        [[ "$verification" =~ ^https:// ]] || die "The sign-in service sent an unexpected answer."
    [[ "$interval" =~ ^[0-9]{1,3}$ ]] && [ "$interval" -ge 1 ] || interval=5
    [[ "$expires" =~ ^[0-9]{1,5}$ ]] || expires=900

    step "Sign in to your ScreenerBot account"
    info "1. On your phone or computer, open:  ${C_BOLD}${verification}${C_RESET}"
    info "2. Sign in there and enter this code:"
    say ""
    say "         ${C_BOLD}${user_code}${C_RESET}"
    say ""
    [[ "$complete" =~ ^https:// ]] && info "Or open ${complete}, which fills the code in."
    info "The code works for $((expires / 60)) minutes. Waiting for you to approve it... (Ctrl+C stops)"

    deadline=$((SECONDS + expires))
    while [ "$SECONDS" -lt "$deadline" ]; do
        sleep "$interval"
        if ! local_api POST /api/account/signin/device/poll "{\"device_code\":\"${device_code}\"}"; then
            err "The sign-in failed: $(api_error_text)"
            return 1
        fi
        state=$(jq -r '.data.state // empty' <<<"$API_BODY")
        case "$state" in
            signed_in)
                local_api GET /api/account/status >/dev/null 2>&1 || true
                name=$(jq -r '.data.name // .data.email // "your account"' <<<"$API_BODY" 2>/dev/null)
                ok "Signed in as ${name}. This server now uses your ScreenerBot account."
                return 0
                ;;
            slow_down)
                interval=$((interval + $(jq -r '.data.backoff_seconds // 5' <<<"$API_BODY")))
                ;;
            expired)
                err "The code expired before it was approved. Run sudo screenerbot-manager signin again."
                return 1
                ;;
        esac
    done
    err "The code expired before it was approved. Run sudo screenerbot-manager signin again."
    return 1
}

cmd_password() {
    local has_password current="" new confirm_new body current_json new_json
    require_installed
    service_active || die "ScreenerBot is not running. Start it first: sudo screenerbot-manager start"
    [ "$HAS_TTY" = 1 ] || die "Setting the dashboard password needs a terminal."
    local_api GET /api/auth/status || die "The dashboard did not answer: $(api_error_text)"
    has_password=$(jq -r '.has_password // false' <<<"$API_BODY")

    step "Dashboard password"
    info "A password protects the dashboard even for someone who reaches it through a tunnel."
    if [ "$has_password" = true ]; then
        info "A password is set. Leave the new password empty to remove it."
        current=$(ask_secret "Current password")
    fi
    new=$(ask_secret "New password")
    if [ -n "$new" ]; then
        confirm_new=$(ask_secret "New password again")
        [ "$new" = "$confirm_new" ] || die "The two passwords differ; nothing was changed."
    elif [ "$has_password" != true ]; then
        info "Nothing was changed."
        return 0
    fi
    new_json=$(json_string "$new") || die "Passwords cannot contain control characters."
    body="{\"new_password\":${new_json}}"
    if [ -n "$current" ]; then
        current_json=$(json_string "$current") || die "Passwords cannot contain control characters."
        body="{\"current_password\":${current_json},\"new_password\":${new_json}}"
    fi
    if local_api POST /api/auth/set-password "$body" && [ "$(jq -r '.success // false' <<<"$API_BODY")" = true ]; then
        if [ -n "$new" ]; then
            ok "Password set. The dashboard now asks for it."
        else
            ok "Password removed."
        fi
        return 0
    fi
    die "The dashboard refused: $(api_error_text)"
}

telegram_token_valid() {
    local token="$1" answer
    [[ "$token" =~ ^[0-9]{5,15}:[A-Za-z0-9_-]{30,64}$ ]] || return 1
    # The token goes to curl through its config on stdin, never on the command line.
    answer=$(printf 'url = "https://api.telegram.org/bot%s/getMe"\n' "$token" |
        curl -sS --proto '=https' --tlsv1.2 --max-time 20 -K - 2>/dev/null)
    [ "$(jq -r '.ok // false' <<<"$answer" 2>/dev/null)" = true ]
}

cmd_telegram() {
    local action="${1:-}" configured enabled token chat body
    require_installed
    service_active || die "ScreenerBot is not running. Start it first: sudo screenerbot-manager start"
    dashboard_login_if_needed || return 1
    if ! local_api GET /api/telegram/status; then
        if [ "$(jq -r '.error.code // empty' <<<"$API_BODY" 2>/dev/null)" = INITIALIZATION_REQUIRED ]; then
            die "Finish setup or choose Explore Mode in the dashboard first; Telegram is available after that."
        fi
        die "The dashboard did not answer: $(api_error_text)"
    fi
    configured=$(jq -r '.bot_configured // false' <<<"$API_BODY")
    enabled=$(jq -r '.enabled // false' <<<"$API_BODY")

    step "Telegram"
    info "ScreenerBot sends trade alerts and new-version notices to a Telegram bot you create"
    info "with @BotFather. Bot configured: ${configured}. Notifications on: ${enabled}."
    if [ -z "$action" ] && [ "$HAS_TTY" = 1 ]; then
        info "1) Send a test message   2) Set the bot token and chat ID   q) Back"
        case "$(ask "Choose" q)" in
            1) action="test" ;;
            2) action=setup ;;
            *) return 0 ;;
        esac
    fi
    case "$action" in
        test)
            if local_api POST /api/telegram/test '{}'; then
                ok "Test message sent. Check your Telegram chat."
            else
                die "Telegram did not accept the test message: $(api_error_text)"
            fi
            ;;
        setup)
            [ "$HAS_TTY" = 1 ] || die "Setting up Telegram needs a terminal."
            token=$(ask_secret "Bot token from @BotFather (input hidden)")
            if ! telegram_token_valid "$token"; then
                die "Telegram does not recognise that bot token; nothing was saved."
            fi
            chat=$(ask "Your chat ID (a number, for example from @userinfobot)")
            [[ "$chat" =~ ^-?[0-9]{3,20}$ ]] || die "A chat ID is a number; nothing was saved."
            body="{\"enabled\":true,\"bot_token\":$(json_string "$token"),\"chat_id\":\"${chat}\"}"
            local_api POST /api/telegram/settings "$body" || die "The dashboard refused the settings: $(api_error_text)"
            ok "Saved. Sending a test message..."
            if local_api POST /api/telegram/test '{}'; then
                ok "Test message sent. Check your Telegram chat."
            else
                err "The bot is saved, but Telegram did not deliver to chat ${chat}: $(api_error_text)"
                info "Open your bot in Telegram and press Start, then: sudo screenerbot-manager telegram test"
                return 1
            fi
            ;;
        "") ;;
        *) die "Unknown telegram action: ${action} (use test or setup)" ;;
    esac
}

render_auto_update_service() {
    cat <<EOF
${UNIT_MARKER}
[Unit]
Description=ScreenerBot automatic update
Documentation=https://screenerbot.io/docs
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=${MANAGER_PATH} auto-update run
EOF
}

render_auto_update_timer() {
    cat <<EOF
${UNIT_MARKER}
[Unit]
Description=Check for ScreenerBot updates every 6 hours

[Timer]
OnCalendar=*-*-* 00/6:00:00
RandomizedDelaySec=45min
Persistent=true

[Install]
WantedBy=timers.target
EOF
}

# Removes the automatic update timer and service when this script wrote them,
# with the timer's persistent run stamp.
remove_auto_update_units() {
    if [ "$(classify_helper_unit "$AUTO_UPDATE_TIMER_FILE")" = ours ]; then
        systemctl disable --now "${AUTO_UPDATE_NAME}.timer" >/dev/null 2>&1 </dev/null
        rm -f "$AUTO_UPDATE_TIMER_FILE" "${TIMER_STAMP_DIR}/stamp-${AUTO_UPDATE_NAME}.timer"
    fi
    remove_if_ours "$AUTO_UPDATE_SERVICE_FILE" "$(classify_helper_unit "$AUTO_UPDATE_SERVICE_FILE")" >/dev/null
}

auto_update_enabled() {
    systemctl is-enabled --quiet "${AUTO_UPDATE_NAME}.timer" 2>/dev/null
}

cmd_auto_update() {
    local action="${1:-status}"
    case "$action" in
        on)
            require_installed
            [ -x "$MANAGER_PATH" ] || die "The manager is missing; run the install command again first."
            if [ "$(classify_helper_unit "$AUTO_UPDATE_SERVICE_FILE")" = foreign ] ||
                [ "$(classify_helper_unit "$AUTO_UPDATE_TIMER_FILE")" = foreign ]; then
                die "${AUTO_UPDATE_NAME} units already exist and belong to something else; nothing was changed."
            fi
            render_auto_update_service >"$AUTO_UPDATE_SERVICE_FILE"
            render_auto_update_timer >"$AUTO_UPDATE_TIMER_FILE"
            chmod 0644 "$AUTO_UPDATE_SERVICE_FILE" "$AUTO_UPDATE_TIMER_FILE"
            systemctl daemon-reload </dev/null
            systemctl enable --now "${AUTO_UPDATE_NAME}.timer" >/dev/null 2>&1 </dev/null
            ok "Automatic updates are on: checked every 6 hours, verified like a manual update,"
            info "rolled back if the new version does not start, and postponed while positions are"
            info "open. ScreenerBot's own settings decide too (Settings > Updates)."
            ;;
        off)
            remove_auto_update_units
            systemctl daemon-reload </dev/null
            ok "Automatic updates are off. Update with: sudo screenerbot-manager update"
            ;;
        status)
            if auto_update_enabled; then
                info "Automatic updates: on"
                systemctl list-timers "${AUTO_UPDATE_NAME}.timer" --no-pager 2>/dev/null </dev/null | sed -n '2p' | sed 's/^/  next check: /'
            else
                info "Automatic updates: off (turn on with: sudo screenerbot-manager auto-update on)"
            fi
            ;;
        run) auto_update_run ;;
        *) die "Unknown auto-update action: ${action} (use on, off or status)" ;;
    esac
}

# One automatic update attempt, as the timer runs it. It follows the app's own
# update settings: "install automatically" and "wait while trading".
auto_update_run() {
    local current arch latest status auto_install defer positions
    current=$(installed_version 2>/dev/null) || {
        say "ScreenerBot is not installed; nothing to update."
        return 0
    }
    arch=$(detect_arch) || return 1
    fetch_update_check "$current" "linux-${arch}-headless"
    status=$?
    latest="$UPDATE_VERSION"
    if [ "$status" = 1 ]; then
        say "ScreenerBot v${current} is the latest version."
        return 0
    elif [ "$status" = 2 ]; then
        say "The update check failed (HTTP ${HTTP_STATUS}); trying again at the next check."
        return 1
    fi
    auto_install=$(config_value updates auto_install 2>/dev/null)
    defer=$(config_value updates defer_while_trading 2>/dev/null)
    if [ "${auto_install:-true}" != true ]; then
        say "v${latest} is available, but automatic installation is off in ScreenerBot (Settings > Updates)."
        return 0
    fi
    if [ "${defer:-true}" = true ] && service_active; then
        positions=$(open_positions_count 2>/dev/null) || {
            if local_api GET /api/auth/status && [ "$(jq -r '.has_password // false' <<<"$API_BODY")" = true ]; then
                say "v${latest} is available; postponed because the dashboard password keeps this check from reading the open positions. Install it with: sudo screenerbot-manager update"
            else
                say "v${latest} is available; postponed because the open positions could not be read."
            fi
            return 0
        }
        if [ "$positions" != 0 ]; then
            say "v${latest} is available; postponed while ${positions} position(s) are open."
            return 0
        fi
    fi
    say "Installing v${latest} (installed: v${current})."
    install_release "$latest" "$(service_user)" "$(unit_port)"
}

notice_manager_update() {
    local temp version
    temp=$(mktemp /tmp/screenerbot-manager.XXXXXX) || return 0
    register_cleanup "$temp"
    fetch_manager_script "$temp" || return 0
    version=$(validate_manager_script "$temp") || return 0
    if [ "$(version_compare "$SCRIPT_VERSION" "$version")" = -1 ]; then
        info "A newer manager is available (v${version}): sudo screenerbot-manager self-update"
    fi
}

cmd_self_update() {
    local temp version
    temp=$(mktemp /tmp/screenerbot-manager.XXXXXX) || die "Could not create a temporary file."
    register_cleanup "$temp"
    fetch_manager_script "$temp" || die "Could not download the manager from ${SCRIPT_URL}."
    version=$(validate_manager_script "$temp") || die "The downloaded manager is incomplete; nothing was changed."
    case "$(version_compare "$SCRIPT_VERSION" "$version")" in
        0)
            if [ -f "$MANAGER_PATH" ] && cmp -s "$temp" "$MANAGER_PATH"; then
                ok "The manager is up to date (v${SCRIPT_VERSION})"
                return 0
            fi
            ;;
        1) die "The published manager (v${version}) is older than this one (v${SCRIPT_VERSION}); nothing was changed." ;;
    esac
    [ "$(classify_manager)" = foreign ] && die "${MANAGER_PATH} is a different program; nothing was changed."
    install_manager_file "$temp" || die "Could not write ${MANAGER_PATH}."
    ok "Manager updated to v${version}"
}

cmd_logs() {
    [[ "${1:-100}" =~ ^[0-9]{1,6}$ ]] || die "logs takes a number of lines, for example: sudo screenerbot-manager logs 200"
    require_systemd
    if [ "$OPT_FOLLOW" = 1 ]; then
        exec journalctl -u "$SERVICE_NAME" -n 50 -f -o cat
    fi
    journalctl -u "$SERVICE_NAME" -n "${1:-100}" --no-pager -o cat </dev/null
    info "Follow new lines with: sudo screenerbot-manager logs -f"
}

cmd_service() {
    local action="$1" version
    require_installed
    case "$action" in
        start | restart)
            systemctl "$action" "$SERVICE_NAME" </dev/null || die "systemd could not ${action} ScreenerBot."
            version=$(installed_version)
            if wait_for_dashboard "$version"; then
                ok "ScreenerBot is running (dashboard on 127.0.0.1:$(dashboard_port))"
            else
                err "ScreenerBot did not come up."
                show_recent_logs
                return 1
            fi
            ;;
        stop)
            systemctl stop "$SERVICE_NAME" </dev/null && ok "ScreenerBot is stopped (it starts again at the next boot)"
            ;;
        enable) systemctl enable "$SERVICE_NAME" >/dev/null 2>&1 </dev/null && ok "ScreenerBot starts at boot" ;;
        disable) systemctl disable "$SERVICE_NAME" >/dev/null 2>&1 </dev/null && ok "ScreenerBot no longer starts at boot" ;;
    esac
}

cmd_status() {
    local current latest="" unit_state active enabled pid since memory user data_dir data_kb port
    local address exposed=0 setup signed account
    require_systemd
    unit_state=$(classify_service_unit)
    current=$(installed_version 2>/dev/null || true)

    step "ScreenerBot on this server"
    if [ "$unit_state" = foreign ]; then
        info "${SERVICE_FILE} belongs to a different program; ScreenerBot is not installed by this manager."
        return 0
    fi
    if [ -z "$current" ] || [ "$unit_state" = absent ]; then
        info "Not installed. Install with: curl -fsSL ${DEFAULT_SCRIPT_URL} | sudo bash"
        return 0
    fi
    if command -v jq >/dev/null 2>&1 && fetch_latest_version; then
        latest="$LATEST_VERSION"
    fi
    if [ -n "$latest" ] && [ "$(version_compare "$current" "$latest")" = -1 ]; then
        info "Version:      v${current} (v${latest} available: sudo screenerbot-manager update)"
    else
        info "Version:      v${current}${latest:+ (latest)}"
    fi
    active=$(systemctl is-active "$SERVICE_NAME" 2>/dev/null)
    enabled=$(systemctl is-enabled "$SERVICE_NAME" 2>/dev/null)
    if [ "$active" = active ]; then
        pid=$(systemctl show -p MainPID --value "$SERVICE_NAME")
        since=$(systemctl show -p ActiveEnterTimestamp --value "$SERVICE_NAME")
        memory=$(ps -o rss= -p "$pid" 2>/dev/null | awk '{ printf "%.0f MB", $1 / 1024 }')
        info "Service:      ${C_GREEN}running${C_RESET} since ${since}, ${memory:-unknown memory}"
    else
        info "Service:      ${C_YELLOW}${active:-stopped}${C_RESET} (start: sudo screenerbot-manager start)"
    fi
    info "At boot:      ${enabled:-unknown}"
    user=$(service_user)
    data_dir=$(service_data_dir)
    data_kb=$(du -sk "$data_dir" 2>/dev/null | awk '{ print $1 }')
    info "Runs as:      ${user}"
    info "Data:         ${data_dir} ($(human_size "${data_kb:-0}"))"
    [ "$user" = root ] && warn "The service runs as root. Reinstalling from a non-root sudo account gives it its own user."

    port=$(dashboard_port)
    if [ "$active" = active ]; then
        while IFS= read -r address; do
            [ -n "$address" ] || continue
            is_loopback_address "$address" || exposed=1
        done < <(service_listen_addresses)
        if [ "$exposed" = 1 ]; then
            warn "The dashboard listens on a public address ($(service_listen_addresses | tr '\n' ' '))."
            warn "Anyone who can reach this server can try its password. Unless that is intended, set"
            warn "Settings > Web server > Host back to 127.0.0.1 and use an SSH tunnel."
        else
            info "Dashboard:    http://127.0.0.1:${port} (this server only)"
        fi
        if local_api GET /api/initialization/status; then
            if [ "$(jq -r '.initialization_complete // false' <<<"$API_BODY")" = true ]; then
                setup="complete"
            elif [ "$(jq -r '.explore_mode // false' <<<"$API_BODY")" = true ]; then
                setup="Explore Mode (no wallet; finish setup in the dashboard to trade)"
            else
                setup="waiting for you in the dashboard"
            fi
            info "Setup:        ${setup}"
        elif [ "$API_STATUS" = 401 ]; then
            info "Setup:        dashboard is password-protected"
        fi
        if local_api GET /api/account/status; then
            signed=$(jq -r '.data.signed_in // false' <<<"$API_BODY")
            account=$(jq -r '.data.name // .data.email // empty' <<<"$API_BODY")
            if [ "$signed" = true ]; then
                info "Account:      signed in${account:+ as ${account}}"
            else
                info "Account:      not signed in (sudo screenerbot-manager signin)"
            fi
        fi
    fi
    if auto_update_enabled; then
        info "Auto-update:  on"
    else
        info "Auto-update:  off (sudo screenerbot-manager auto-update on)"
    fi
    if [ -f "$UPDATE_STATE_FILE" ] && command -v jq >/dev/null 2>&1; then
        info "Last update:  $(jq -r '"\(.phase) v\(.version) at \(.updatedAt)"' "$UPDATE_STATE_FILE" 2>/dev/null)"
    fi
    if [ -f "$PREVIOUS_BINARY_PATH" ]; then
        info "Previous:     v$(binary_version "$PREVIOUS_BINARY_PATH") (sudo screenerbot-manager rollback)"
    fi
    info "Manager:      v${SCRIPT_VERSION}"
    say ""
    info "Open the dashboard from your computer through an SSH tunnel:"
    ssh_tunnel_hint "$port"
    info "then browse to http://localhost:${port}"
}

cmd_check() {
    preflight || return 1
    guard_ownership
    ok "This server can run ScreenerBot"
}

usage() {
    cat <<EOF
ScreenerBot manager v${SCRIPT_VERSION}: installs and runs ScreenerBot as a service on a Linux server.

Usage: sudo screenerbot-manager [command] [options]

Without a command it opens a menu on a terminal. Streamed from the installer
without a terminal (cloud-init, automation), it installs the latest version.

Install and update
  install [VERSION]    Install the latest or the given version, or repair an install
        --port N         Dashboard port (default ${DEFAULT_PORT}, or the next free one)
  reinstall            Download the installed version again and replace it
  update               Install the newest version, rolled back if it does not start
  rollback             Go back to the version installed before the last update
  versions             List the versions you can install
  auto-update on|off|status   Install new versions by themselves every 6 hours
  self-update          Update this manager

Run
  status               Version, service, dashboard, setup and account at a glance
  start | stop | restart
  enable | disable     Start at boot, or not
  logs [-f]            Recent log lines; -f follows new ones

Account and dashboard
  signin               Sign in to your ScreenerBot account with a code
  password             Set, change or remove the dashboard password
  telegram [test|setup]  Test or set up Telegram notifications

Data
  backup               Save wallets, settings and databases to an archive
  restore [FILE]       Restore a backup (the current data is saved first)

Remove
  uninstall            Remove ScreenerBot; keeps wallets and settings
        --purge          Also delete all data, the service user and added packages

Other
  check                Check whether this server can run ScreenerBot
  help | version

Options: -y, --yes answers yes to every question.

Open the dashboard through an SSH tunnel from your own computer:
  ssh -N -L ${DEFAULT_PORT}:127.0.0.1:${DEFAULT_PORT} USER@YOUR_SERVER_ADDRESS   then http://localhost:${DEFAULT_PORT}
EOF
}

# =============================================================================
# Menu
# =============================================================================

menu_summary() {
    local current active
    current=$(installed_version 2>/dev/null || true)
    active=$(systemctl is-active "$SERVICE_NAME" 2>/dev/null)
    say ""
    say "${C_BOLD}ScreenerBot manager v${SCRIPT_VERSION}${C_RESET}   ${C_DIM}screenerbot.io${C_RESET}"
    say "  ScreenerBot v${current:-?}, service ${active:-unknown}, dashboard on 127.0.0.1:$(dashboard_port)"
}

# Runs one menu action in a subshell, so an action that stops with an error
# returns to the menu. The subshell removes its own temporary files.
run_menu_action() {
    (
        trap run_cleanup EXIT
        "$@"
    )
}

menu() {
    local choice
    while true; do
        menu_summary
        say ""
        info " 1  Status"
        info " 2  Update ScreenerBot"
        info " 3  Sign in to your ScreenerBot account"
        info " 4  Start, stop or restart"
        info " 5  Recent logs"
        info " 6  Back up wallets and settings"
        info " 7  Restore a backup"
        info " 8  Dashboard password"
        info " 9  Telegram"
        info "10  Automatic updates"
        info "11  Install another version or roll back"
        info "12  Update this manager"
        info "13  Uninstall"
        info " q  Quit"
        choice=$(ask "Choose" q)
        case "$choice" in
            1) run_menu_action cmd_status ;;
            2) run_menu_action cmd_update ;;
            3) run_menu_action cmd_signin ;;
            4)
                case "$(ask "start, stop or restart" restart)" in
                    start) run_menu_action cmd_service start ;;
                    stop) run_menu_action cmd_service stop ;;
                    restart) run_menu_action cmd_service restart ;;
                esac
                ;;
            5) run_menu_action cmd_logs 60 ;;
            6) run_menu_action cmd_backup ;;
            7) run_menu_action cmd_restore ;;
            8) run_menu_action cmd_password ;;
            9) run_menu_action cmd_telegram ;;
            10)
                run_menu_action cmd_auto_update status
                if auto_update_enabled; then
                    confirm "Turn automatic updates off?" n && run_menu_action cmd_auto_update off
                else
                    confirm "Turn automatic updates on?" n && run_menu_action cmd_auto_update on
                fi
                ;;
            11)
                run_menu_action cmd_versions
                if [ -f "$PREVIOUS_BINARY_PATH" ]; then
                    info "Or type rollback to go back to v$(binary_version "$PREVIOUS_BINARY_PATH")."
                fi
                choice=$(ask "Version to install, or rollback (empty to cancel)")
                if [ "$choice" = rollback ]; then
                    run_menu_action cmd_rollback
                elif [ -n "$choice" ]; then
                    run_menu_action cmd_install "$choice"
                fi
                ;;
            12) run_menu_action cmd_self_update ;;
            13)
                run_menu_action cmd_uninstall
                [ -x "$MANAGER_PATH" ] || return 0
                ;;
            q | Q | "") return 0 ;;
            *) err "Choose a number from the list" ;;
        esac
        pause
    done
}

guided_install() {
    say ""
    say "${C_BOLD}ScreenerBot installer v${SCRIPT_VERSION}${C_RESET}"
    info "Installs ScreenerBot as a background service that starts with the server. Your"
    info "wallet keys never leave this server, and the dashboard is reachable only through"
    info "an SSH tunnel from your own computer."
    if ! confirm "Install ScreenerBot now?" y; then
        info "Nothing was changed."
        return 1
    fi
    cmd_install "" || return 1
    if confirm "Install new versions automatically (checked every 6 hours, never while positions are open)?" n; then
        cmd_auto_update on
    fi
}

# =============================================================================
# Entry point
# =============================================================================

main() {
    local original=("$@") args=() arg command
    for arg in "$@"; do
        case "$arg" in
            -y | --yes) ASSUME_YES=1 ;;
            --purge) OPT_PURGE=1 ;;
            -f | --follow) OPT_FOLLOW=1 ;;
            --port=*) OPT_PORT="${arg#--port=}" ;;
            *) args+=("$arg") ;;
        esac
    done
    set -- ${args[@]+"${args[@]}"}
    args=()
    while [ $# -gt 0 ]; do
        if [ "$1" = --port ]; then
            OPT_PORT="${2:-}"
            shift 2 || shift
        else
            args+=("$1")
            shift
        fi
    done
    set -- ${args[@]+"${args[@]}"}
    if [ -n "$OPT_PORT" ] && { ! [[ "$OPT_PORT" =~ ^[0-9]{2,5}$ ]] || [ "$OPT_PORT" -lt 1024 ] || [ "$OPT_PORT" -gt 65535 ]; }; then
        die "--port takes a number from 1024 to 65535"
    fi

    command="${1:-}"
    [ $# -gt 0 ] && shift
    case "$command" in
        help | -h | --help)
            usage
            return 0
            ;;
        version | --version | -v)
            say "screenerbot-manager ${SCRIPT_VERSION}"
            return 0
            ;;
    esac

    require_root ${original[@]+"${original[@]}"}
    detect_tty
    configure_script_url
    trap run_cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM

    case "$command" in
        "")
            acquire_lock
            if [ "$HAS_TTY" != 1 ]; then
                cmd_install ""
            elif [ -x "$BINARY_PATH" ] && [ "$(classify_service_unit)" != absent ]; then
                menu
            else
                guided_install
            fi
            ;;
        install)
            acquire_lock
            cmd_install "${1:-}"
            ;;
        reinstall)
            acquire_lock
            require_installed
            cmd_install "$(installed_version)" 1
            ;;
        update)
            acquire_lock
            cmd_update
            ;;
        rollback)
            acquire_lock
            cmd_rollback
            ;;
        versions) cmd_versions ;;
        uninstall | remove)
            acquire_lock
            cmd_uninstall
            ;;
        status) cmd_status ;;
        start | stop | restart | enable | disable) cmd_service "$command" ;;
        logs) cmd_logs "${1:-100}" ;;
        signin | sign-in | login) cmd_signin ;;
        password) cmd_password ;;
        telegram) cmd_telegram "${1:-}" ;;
        auto-update)
            [ "${1:-status}" = status ] || acquire_lock
            cmd_auto_update "${1:-status}"
            ;;
        backup) cmd_backup ;;
        restore)
            acquire_lock
            cmd_restore "${1:-}"
            ;;
        self-update | update-manager)
            acquire_lock
            cmd_self_update
            ;;
        check) cmd_check ;;
        *)
            err "Unknown command: ${command}"
            usage >&2
            return 1
            ;;
    esac
}

# Library-only mode lets the test harness exercise the parsing and safety
# checks without root and without running a command.
if [ "${SCREENERBOT_INSTALLER_LIBRARY_ONLY:-0}" != "1" ]; then
    main "$@"
fi
