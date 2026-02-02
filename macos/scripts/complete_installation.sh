#!/usr/bin/env zsh
set -euo pipefail

WEAVE_LOCATION="/Library/Weave"
LOG_FILE="$HOME/.cache/weave_install.log"
STATE_FILE="$HOME/.cache/weave_install_state"

# ============================================================================
# STATE MANAGEMENT
# ============================================================================

save_state() {
    echo "$1" >"$STATE_FILE"
    log "[STATE] Saved: $1"
}

get_state() {
    if [ -f "$STATE_FILE" ]; then
        cat "$STATE_FILE"
    else
        echo "START"
    fi
}

clear_state() {
    rm -f "$STATE_FILE"
    log "[STATE] Cleared"
}

is_step_complete() {
    local step="$1"
    local current_state=$(get_state)

    case "$current_state" in
    "START")
        return 1
        ;;
    "WEAVE_CLI_INSTALLED")
        [ "$step" = "WEAVE_CLI_INSTALLED" ] && return 0 || return 1
        ;;
    "HOMEBREW_READY")
        [[ "$step" =~ ^(WEAVE_CLI_INSTALLED|HOMEBREW_READY)$ ]] && return 0 || return 1
        ;;
    "MAYAFLUX_INSTALLED")
        [[ "$step" =~ ^(WEAVE_CLI_INSTALLED|HOMEBREW_READY|MAYAFLUX_INSTALLED)$ ]] && return 0 || return 1
        ;;
    "ENVIRONMENT_CONFIGURED")
        [[ "$step" =~ ^(WEAVE_CLI_INSTALLED|HOMEBREW_READY|MAYAFLUX_INSTALLED|ENVIRONMENT_CONFIGURED)$ ]] && return 0 || return 1
        ;;
    esac
    return 1
}

# ============================================================================
# LOGGING & ERROR HANDLING
# ============================================================================

log() {
    echo "$*"
    echo "$*" >>"$LOG_FILE"
}

error() {
    echo "ERROR: $*"
    echo "ERROR: $*" >>"$LOG_FILE"

    osascript -e "Tell application \"System Events\" to display dialog \"Installation failed:\n\n$*\n\nCheck log: $LOG_FILE\n\n⚠️  Re-run the installer to retry from this point.\" buttons {\"OK\"} default button \"OK\" with title \"Weave Installer Error\" with icon stop" 2>/dev/null || echo "ERROR: $*"

    exit 1
}

retry_command() {
    local max_attempts=3
    local attempt=1
    local cmd="$@"

    while [ $attempt -le $max_attempts ]; do
        log "  Attempt $attempt/$max_attempts..."

        if eval "$cmd"; then
            return 0
        else
            log "  ⚠️  Attempt $attempt failed"

            if [ $attempt -eq $max_attempts ]; then
                return 1
            fi

            attempt=$((attempt + 1))
            sleep 2
        fi
    done

    return 1
}

trap 'error "Unexpected error occurred. See log for details."' ERR

# ============================================================================
# INSTALLATION STEPS
# ============================================================================

step_welcome() {
    clear
    cat <<'BANNER'
╔════════════════════════════════════════════════════════════╗
║                                                            ║
║ 🎛️  Weave - MayaFlux Installer                             ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝

BANNER

    local current_state=$(get_state)
    if [ "$current_state" != "START" ]; then
        log "📍 Resuming from: $current_state"
        log ""
        osascript -e "Tell application \"System Events\" to display dialog \"Previous installation was interrupted.\n\nResuming from: $current_state\n\nClick Continue to resume.\" buttons {\"Cancel\", \"Continue\"} default button \"Continue\" with title \"Weave Installer - Resume\" with icon note" 2>/dev/null || true
    else
        log "Starting Installation..."
        log ""
        osascript -e 'Tell application "System Events" to display dialog "Welcome to Weave Installer!\n\nThis will install MayaFlux via Homebrew and set up your environment.\n\nYou will be asked for your password once." buttons {"Cancel", "Continue"} default button "Continue" with title "Weave Installer" with icon note' 2>/dev/null || echo "Continuing..."
    fi
}

step_install_weave_cli() {
    if is_step_complete "WEAVE_CLI_INSTALLED"; then
        log "✓ Weave CLI already installed (skipping)"
        return 0
    fi

    log "➤ Installing Weave CLI..."
    WEAVE_BIN="$HOME/.local/bin"

    mkdir -p "$WEAVE_BIN" || error "Failed to create $WEAVE_BIN"

    if ! retry_command "rm -f \"$WEAVE_BIN/weave\""; then
        error "Failed to remove old weave executable after 3 attempts"
    fi

    if ! retry_command "cp \"$WEAVE_LOCATION/project_creator.sh\" \"$WEAVE_BIN/weave\""; then
        error "Failed to copy project_creator.sh after 3 attempts"
    fi

    if ! retry_command "chmod +x \"$WEAVE_BIN/weave\""; then
        error "Failed to make weave executable after 3 attempts"
    fi

    log "✅ Weave CLI installed"
    save_state "WEAVE_CLI_INSTALLED"
}

step_check_homebrew() {
    if is_step_complete "HOMEBREW_READY"; then
        log "✓ Homebrew already ready (skipping)"

        # Re-detect BREW_CMD for later steps
        for brew_path in /opt/homebrew/bin/brew /usr/local/bin/brew; do
            if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
                BREW_CMD="$brew_path"
                break
            fi
        done
        return 0
    fi

    log "➤ Checking for Homebrew..."

    BREW_CMD=""
    for brew_path in /opt/homebrew/bin/brew /usr/local/bin/brew; do
        if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
            BREW_CMD="$brew_path"
            break
        fi
    done

    if [ -z "$BREW_CMD" ]; then
        log "Installing Homebrew..."
        log "  ⚠️  You will be prompted for your password"

        if ! retry_command "/bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""; then
            error "Failed to install Homebrew after 3 attempts.\n\nPlease check your internet connection."
        fi

        # Re-detect after installation
        for brew_path in /opt/homebrew/bin/brew /usr/local/bin/brew; do
            if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
                BREW_CMD="$brew_path"
                break
            fi
        done

        if [ -z "$BREW_CMD" ]; then
            error "Homebrew installed but executable not found.\n\nExpected at /opt/homebrew/bin/brew or /usr/local/bin/brew"
        fi
    fi

    log "✅ Homebrew ready at $BREW_CMD"
    save_state "HOMEBREW_READY"
}

step_check_conflicts() {
    log "➤ Checking for existing MayaFlux installations..."

    MAYAFLUX_INSTALLED=$("$BREW_CMD" list --formula | grep -E '^mayaflux$' || true)
    MAYAFLUX_DEV_INSTALLED=$("$BREW_CMD" list --formula | grep -E '^mayaflux-dev$' || true)

    if [ -n "$MAYAFLUX_INSTALLED" ]; then
        log "  Found: mayaflux (stable)"
    fi
    if [ -n "$MAYAFLUX_DEV_INSTALLED" ]; then
        log "  Found: mayaflux-dev"
    fi
    if [ -z "$MAYAFLUX_INSTALLED" ] && [ -z "$MAYAFLUX_DEV_INSTALLED" ]; then
        log "  No existing installations found"
    fi
}

step_select_release() {
    log "➤ Selecting release channel..."

    RELEASE_TYPE=$(osascript -e 'choose from list {"Stable", "Development (latest)"} with prompt "Select MayaFlux release channel:" default items {"Stable"}' 2>/dev/null)

    if [[ "$RELEASE_TYPE" == "false" ]]; then
        log "Installation cancelled by user"
        clear_state
        exit 0
    fi

    if [[ "$RELEASE_TYPE" == "Development (latest)" ]]; then
        FORMULA="mayaflux-dev"
        CONFLICTING_FORMULA="mayaflux"
        log "  Selected: Development (mayaflux-dev)"
    else
        FORMULA="mayaflux"
        CONFLICTING_FORMULA="mayaflux-dev"
        log "  Selected: Stable (mayaflux)"
    fi
}

step_handle_conflicts() {
    if [[ "$FORMULA" == "mayaflux" && -n "$MAYAFLUX_DEV_INSTALLED" ]]; then
        log "⚠️  Warning: mayaflux-dev is currently installed"

        CHOICE=$(osascript -e 'display dialog "You have mayaflux-dev installed, but selected the Stable release.\n\nBoth versions cannot coexist. What would you like to do?" buttons {"Exit", "Remove Dev & Install Stable"} default button "Exit" with title "Conflicting Installation" with icon caution' 2>/dev/null | sed -n 's/.*button returned:\([^,]*\).*/\1/p' || echo "Exit")

        if [[ "$CHOICE" == "Remove Dev & Install Stable" ]]; then
            log "  Removing mayaflux-dev..."

            if ! retry_command "\"$BREW_CMD\" uninstall mayaflux-dev"; then
                error "Failed to uninstall mayaflux-dev after 3 attempts"
            fi

            log "✅ mayaflux-dev removed"
        else
            log "Installation cancelled by user"
            clear_state
            exit 0
        fi
    elif [[ "$FORMULA" == "mayaflux-dev" && -n "$MAYAFLUX_INSTALLED" ]]; then
        log "⚠️  Warning: mayaflux (stable) is currently installed"

        CHOICE=$(osascript -e 'display dialog "You have mayaflux (stable) installed, but selected Development.\n\nBoth versions cannot coexist. What would you like to do?" buttons {"Exit", "Remove Stable & Install Dev"} default button "Exit" with title "Conflicting Installation" with icon caution' 2>/dev/null | sed -n 's/.*button returned:\([^,]*\).*/\1/p' || echo "Exit")

        if [[ "$CHOICE" == "Remove Stable & Install Dev" ]]; then
            log "  Removing mayaflux..."

            if ! retry_command "\"$BREW_CMD\" uninstall mayaflux"; then
                error "Failed to uninstall mayaflux after 3 attempts"
            fi

            log "✅ mayaflux removed"
        else
            log "Installation cancelled by user"
            clear_state
            exit 0
        fi
    fi
}

step_install_mayaflux() {
    if is_step_complete "MAYAFLUX_INSTALLED"; then
        log "✓ MayaFlux already installed (skipping)"

        # Re-detect FORMULA for later steps
        MAYAFLUX_INSTALLED=$("$BREW_CMD" list --formula | grep -E '^mayaflux$' || true)
        MAYAFLUX_DEV_INSTALLED=$("$BREW_CMD" list --formula | grep -E '^mayaflux-dev$' || true)

        if [ -n "$MAYAFLUX_DEV_INSTALLED" ]; then
            FORMULA="mayaflux-dev"
        elif [ -n "$MAYAFLUX_INSTALLED" ]; then
            FORMULA="mayaflux"
        else
            error "State says MayaFlux installed but not found in brew list"
        fi

        return 0
    fi

    log "➤ Installing MayaFlux via Homebrew..."
    log "  Formula: $FORMULA"

    log "  Adding tap: mayaflux/mayaflux"
    if ! retry_command "\"$BREW_CMD\" tap mayaflux/mayaflux"; then
        error "Failed to add mayaflux tap after 3 attempts.\n\nCheck your internet connection."
    fi
    log "  ✅ Tap added"

    log "  Installing: $FORMULA"
    log "  ⏳ This may take several minutes..."

    if ! retry_command "\"$BREW_CMD\" install \"$FORMULA\""; then
        error "Failed to install $FORMULA after 3 attempts.\n\nCheck your internet connection and Homebrew status."
    fi

    log "✅ MayaFlux installation step completed"

    # Verify installation
    log "  Verifying installation..."
    if ! "$BREW_CMD" list --formula | grep "^$FORMULA$"; then
        error "MayaFlux ($FORMULA) installation verification failed.\n\nFormula not found in brew list."
    fi

    log "✅ MayaFlux verified"
    save_state "MAYAFLUX_INSTALLED"
}

step_configure_environment() {
    if is_step_complete "ENVIRONMENT_CONFIGURED"; then
        log "✓ Environment already configured (skipping)"
        return 0
    fi

    log "➤ Configuring environment..."

    MAYAFLUX_PREFIX=$("$BREW_CMD" --prefix "$FORMULA" 2>/dev/null)

    if [ -z "$MAYAFLUX_PREFIX" ]; then
        error "Failed to get Homebrew prefix for $FORMULA.\n\nThis should not happen."
    fi

    log "  MayaFlux prefix: $MAYAFLUX_PREFIX"

    ZSHENV="${ZDOTDIR:-$HOME}/.zshenv"

    log "  Cleaning up old MayaFlux configuration..."

    if [ -f "$ZSHENV" ]; then
        # Backup
        cp "$ZSHENV" "${ZSHENV}.weave-backup-$(date +%s)" || error "Failed to backup .zshenv"
        log "  Created backup: ${ZSHENV}.weave-backup-$(date +%s)"

        # Clean old entries
        sed -i '' '/# MayaFlux/d' "$ZSHENV" || error "Failed to clean .zshenv"
        sed -i '' '/MAYAFLUX_ROOT/d' "$ZSHENV" || error "Failed to clean .zshenv"
        sed -i '' '/mayaflux_env\.sh/d' "$ZSHENV" || error "Failed to clean .zshenv"
        sed -i '' '/source.*mayaflux.*\/env\.sh/d' "$ZSHENV" || error "Failed to clean .zshenv"
        sed -i '' '/\.local\/bin.*\$PATH/d' "$ZSHENV" || error "Failed to clean .zshenv"
        sed -i '' '/./,$!d' "$ZSHENV" || error "Failed to clean .zshenv"
    fi

    log "  ✅ Cleaned old configuration"

    log "  Writing new configuration..."
    cat >>"$ZSHENV" <<EOF

# MayaFlux (installed via Homebrew)
source "$MAYAFLUX_PREFIX/env.sh"
export PATH="\$HOME/.local/bin:\$PATH"
EOF

    if [ $? -ne 0 ]; then
        error "Failed to write to .zshenv"
    fi

    log "✅ Environment configured"
    save_state "ENVIRONMENT_CONFIGURED"
}

step_complete() {
    log ""
    log "=========================================="
    log "✅ Installation Complete!"
    log "=========================================="
    log ""
    log "Next steps:"
    log "  1. Restart your terminal or run: source ~/.zshenv"
    log "  2. Create a project: weave new MyProject ~/Projects/"
    log ""

    clear_state

    osascript -e 'Tell application "System Events" to display dialog "Installation complete! 🎉\n\n1. Restart your terminal\n2. weave new MyProject ~/Projects/" buttons {"OK"} default button "OK"' 2>/dev/null || true
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

step_welcome
step_install_weave_cli
step_check_homebrew
step_check_conflicts
step_select_release
step_handle_conflicts
step_install_mayaflux
step_configure_environment
step_complete
