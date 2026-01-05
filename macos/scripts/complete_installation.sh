#!/usr/bin/env zsh
set -euo pipefail

WEAVE_LOCATION="/Library/Weave"
LOG_FILE="$HOME/.weave_install.log"

log() {
    echo "$*"
    echo "$*" >>"$LOG_FILE"
}

error() {
    echo "ERROR: $*"
    echo "ERROR: $*" >>"$LOG_FILE"

    osascript -e "Tell application \"System Events\" to display dialog \"Installation failed:\n\n$*\n\nCheck log: $LOG_FILE\" buttons {\"OK\"} default button \"OK\" with title \"Weave Installer Error\" with icon stop" 2>/dev/null || echo "ERROR: $*"

    exit 1
}

trap 'error "Unexpected error occurred. See log for details."' ERR

clear
cat <<'BANNER'
╔════════════════════════════════════════════════════════════╗
║                                                            ║
║ 🎛️  Weave - MayaFlux Installer                            ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝

BANNER

log "Starting Installation..."
log ""

osascript -e 'Tell application "System Events" to display dialog "Welcome to Weave Installer!\n\nThis will install MayaFlux via Homebrew and set up your environment.\n\nYou will be asked for your password once." buttons {"Cancel", "Continue"} default button "Continue" with title "Weave Installer" with icon note' 2>/dev/null || echo "Continuing..."

#----- Setup Weave CLI -----
log "➤ Installing Weave CLI..."
WEAVE_BIN="$HOME/.local/bin"
mkdir -p "$WEAVE_BIN" || error "Failed to create $WEAVE_BIN"
rm -f "$WEAVE_BIN/weave" || error "Failed to remove old weave executable"
cp "$WEAVE_LOCATION/project_creator.sh" "$WEAVE_BIN/weave" || error "Failed to copy project_creator.sh"
chmod +x "$WEAVE_BIN/weave" || error "Failed to make weave executable"
log "✅ Weave CLI installed"

#----- Homebrew -----
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
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" || error "Failed to install Homebrew"
    BREW_CMD=$(/opt/homebrew/bin/brew --prefix)/bin/brew
fi

log "✅ Homebrew ready"

#----- Check for existing installations -----
log "➤ Checking for existing MayaFlux installations..."

MAYAFLUX_INSTALLED=$("$BREW_CMD" list --formula | grep -E '^mayaflux$' || true)
MAYAFLUX_DEV_INSTALLED=$("$BREW_CMD" list --formula | grep -E '^mayaflux-dev$' || true)

#----- Release Type Selection -----
RELEASE_TYPE=$(osascript -e 'choose from list {"Stable", "Development (latest)"} with prompt "Select MayaFlux release channel:" default items {"Stable"}' 2>/dev/null)

if [[ "$RELEASE_TYPE" == "false" ]]; then
    log "Installation cancelled by user"
    exit 0
fi

if [[ "$RELEASE_TYPE" == "Development (latest)" ]]; then
    FORMULA="mayaflux-dev"
    CONFLICTING_FORMULA="mayaflux"
else
    FORMULA="mayaflux"
    CONFLICTING_FORMULA="mayaflux-dev"
fi

#----- Handle conflicts -----
if [[ "$FORMULA" == "mayaflux" && -n "$MAYAFLUX_DEV_INSTALLED" ]]; then
    log "⚠️  Warning: mayaflux-dev is currently installed"

    CHOICE=$(osascript -e 'display dialog "You have mayaflux-dev installed, but selected the Stable release.\n\nBoth versions cannot coexist. What would you like to do?" buttons {"Exit", "Remove Dev & Install Stable"} default button "Exit" with title "Conflicting Installation" with icon caution' 2>/dev/null | grep -oP 'button returned:\K.*' || echo "Exit")

    if [[ "$CHOICE" == "Remove Dev & Install Stable" ]]; then
        log "Removing mayaflux-dev..."
        "$BREW_CMD" uninstall mayaflux-dev || error "Failed to uninstall mayaflux-dev"
        log "✅ mayaflux-dev removed"
    else
        log "Installation cancelled by user"
        exit 0
    fi
elif [[ "$FORMULA" == "mayaflux-dev" && -n "$MAYAFLUX_INSTALLED" ]]; then
    log "⚠️  Warning: mayaflux (stable) is currently installed"

    CHOICE=$(osascript -e 'display dialog "You have mayaflux (stable) installed, but selected Development.\n\nBoth versions cannot coexist. What would you like to do?" buttons {"Exit", "Remove Stable & Install Dev"} default button "Exit" with title "Conflicting Installation" with icon caution' 2>/dev/null | grep -oP 'button returned:\K.*' || echo "Exit")

    if [[ "$CHOICE" == "Remove Stable & Install Dev" ]]; then
        log "Removing mayaflux..."
        "$BREW_CMD" uninstall mayaflux || error "Failed to uninstall mayaflux"
        log "✅ mayaflux removed"
    else
        log "Installation cancelled by user"
        exit 0
    fi
fi

#----- Install MayaFlux -----
log "➤ Installing MayaFlux via Homebrew..."

"$BREW_CMD" tap mayaflux/mayaflux || error "Failed to tap mayaflux/mayaflux"
"$BREW_CMD" install "$FORMULA" || error "Failed to install $FORMULA"
log "✅ MayaFlux installed"

#----- Setup Environment -----
log "➤ Configuring environment..."
MAYAFLUX_PREFIX=$("$BREW_CMD" --prefix "$FORMULA")
ZSHENV="${ZDOTDIR:-$HOME}/.zshenv"

# Remove any existing MayaFlux configuration
if [ -f "$ZSHENV" ]; then
    log "Cleaning up old MayaFlux configuration..."

    TEMP_ZSHENV=$(mktemp)

    IN_MAYAFLUX_BLOCK=false

    while IFS= read -r line; do
        if [[ "$line" =~ ^#.*MayaFlux ]]; then
            IN_MAYAFLUX_BLOCK=true
            continue
        fi

        if [[ "$line" =~ source.*/(mayaflux|mayaflux-dev)/env\.sh ]]; then
            continue
        fi

        if [[ "$line" =~ MAYAFLUX_ROOT|HOME/\.local/bin.*PATH ]] && [[ "$IN_MAYAFLUX_BLOCK" == true ]]; then
            continue
        fi

        if [[ -z "$line" ]] && [[ "$IN_MAYAFLUX_BLOCK" == true ]]; then
            IN_MAYAFLUX_BLOCK=false
            continue
        fi

        if [[ "$IN_MAYAFLUX_BLOCK" == false ]]; then
            echo "$line" >>"$TEMP_ZSHENV"
        fi
    done <"$ZSHENV"

    mv "$TEMP_ZSHENV" "$ZSHENV" || error "Failed to update $ZSHENV"
    log "✅ Cleaned old configuration"
fi

cat >>"$ZSHENV" <<EOF

# MayaFlux (installed via Homebrew)
source "$MAYAFLUX_PREFIX/env.sh"
export PATH="\$HOME/.local/bin:\$PATH"
EOF

log "✅ Environment configured"

#----- Done -----
log ""
log "=========================================="
log "✅ Installation Complete!"
log "=========================================="
log ""
log "Next steps:"
log "  1. Restart your terminal or run: source ~/.zshenv"
log "  2. Create a project: weave new MyProject ~/Projects/"
log ""

osascript -e 'Tell application "System Events" to display dialog "Installation complete! 🎉\n\n1. Restart your terminal\n2. weave new MyProject ~/Projects/" buttons {"OK"} default button "OK"' 2>/dev/null || true
