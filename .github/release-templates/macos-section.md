**Platform:** macOS 15.0+ (Sequoia or later)
**Architecture:** Universal Binary (arm64 + x86_64)

<details>
<summary>Click to expand</summary>

### Installation

**Open the `.dmg` file** and double-click **Weave.app** to launch the installer.

Weave will:
1. Install Homebrew if not already present (requires your password once)
2. Download and install the MayaFlux framework via Homebrew (~100 MB)
3. Configure your shell environment in `$ZDOTDIR/.zshenv`
4. Install the `weave` CLI tool to `~/.local/bin/weave`

### What Gets Installed

- **MayaFlux Framework** — installed via Homebrew
- **CLI Tool** — `weave` at `~/.local/bin/weave`
- **Shell Environment** — `source` line added to `$ZDOTDIR/.zshenv`

### Environment Setup

The following is added to `~/.zshenv` automatically:

```zsh
source "<homebrew-prefix>/env.sh"
export PATH="$HOME/.local/bin:$PATH"
```

**Restart your terminal** or run `source $ZDOTDIR/.zshenv` to activate immediately.

### Creating Your First Project

**Using Weave.app (GUI):**
1. Open **Weave.app** from the DMG (or move it to `/Applications` first)
2. Choose **Create New Project**
3. Enter a project name and location, then click **Create Project**

**Using CLI (after installation):**
```bash
weave new MyProject ~/Projects/
weave new MyProject ~/Projects/ --with-lila     # With live coding
weave new MyProject ~/Projects/ --no-vscode     # Without VS Code config
```

---

</details>
