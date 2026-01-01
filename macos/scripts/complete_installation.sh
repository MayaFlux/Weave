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
    exit 1
}

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
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    BREW_CMD=$(/opt/homebrew/bin/brew --prefix)/bin/brew
fi

log "✅ Homebrew ready"

#----- Install MayaFlux -----
log "➤ Installing MayaFlux via Homebrew..."

RELEASE_TYPE=$(osascript -e 'choose from list {"Stable", "Development (latest)"} with prompt "Select MayaFlux release channel:" default items {"Stable"}' 2>/dev/null)

if [[ "$RELEASE_TYPE" == "false" ]]; then
    exit 0
fi

if [[ "$RELEASE_TYPE" == "Development (latest)" ]]; then
    FORMULA="mayaflux-dev"
else
    FORMULA="mayaflux"
fi

"$BREW_CMD" tap mayaflux/mayaflux
"$BREW_CMD" install "$FORMULA"
log "✅ MayaFlux installed"

#----- Setup Environment -----
log "➤ Configuring environment..."
MAYAFLUX_PREFIX=$("$BREW_CMD" --prefix mayaflux-dev)
ZSHENV="${ZDOTDIR:-$HOME}/.zshenv"

if ! grep -q "MAYAFLUX_ROOT" "$ZSHENV" 2>/dev/null; then
    cat >>"$ZSHENV" <<EOF

# MayaFlux (installed via Homebrew)
source "$MAYAFLUX_PREFIX/env.sh"
export PATH="\$HOME/.local/bin:\$PATH"
EOF
fi

log "✅ Environment configured"

#----- Setup Weave CLI -----
log "➤ Installing Weave CLI..."
WEAVE_BIN="$HOME/.local/bin"
mkdir -p "$WEAVE_BIN"
cp "$WEAVE_LOCATION/project_creator.sh" "$WEAVE_BIN/weave"
chmod +x "$WEAVE_BIN/weave"
log "✅ Weave CLI installed"

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
