## macos-section.md

**Platform:** macOS 14.0+ (Sonoma or later)  
**Architecture:** Universal Binary (arm64 + x86_64)

<details>
<summary>Click to expand</summary>

### Installation

**Double-click the `.pkg` file** to start installation.

The installer will:

1. Install Weave.app to `/Applications`
2. Copy templates and CLI tool to `/Library/Weave`
3. Open a Terminal window showing real-time installation progress
4. Download and install MayaFlux framework (~100 MB)
5. Install dependencies via Homebrew (takes 10-30 minutes)

**Command-line alternative:**

```bash
sudo installer -pkg Weave-{{VERSION}}.pkg -target /
```

### What Gets Installed

- **Weave.app** - Universal GUI application in `/Applications`
- **CLI Tool** - Command-line tool at `~/.local/bin/weave`
- **Project Templates** - Located at `/Library/Weave/templates`
- **MayaFlux Framework** - Downloaded to `/Library/MayaFlux`
- **Dependencies** - Installed via Homebrew (ffmpeg, rtaudio, glfw, eigen, vulkan-sdk, etc.)

### Environment Setup

Environment variables are added to `~/.zshenv` automatically:

- `MAYAFLUX_ROOT` - Points to MayaFlux installation
- `CMAKE_PREFIX_PATH` - For CMake package discovery
- `PATH` - Updated with MayaFlux tools

**Restart your terminal** or run `source ~/.zshenv` to activate immediately.

### Creating Your First Project

**Using Weave.app (GUI):**

1. Open `/Applications/Weave.app` or find it in Spotlight
2. Enter project name and select location
3. Click "Create Project"

**Using CLI:**

```bash
weave new MyProject ~/Projects/
weave new MyProject ~/Projects/ --with-lila     # With live coding
weave new MyProject ~/Projects/ --no-vscode    # Without VS Code config
```

**Note:** The standalone app requires the Weave files and templates to be already installed via the full installer.

**Installation:**

1. Download and open \`Weave-${VERSION}.pkg\`
2. Follow the installer prompts (installs Weave.app and files)
3. Terminal will open automatically with installation progress
4. Watch real-time progress for MayaFlux download and dependency installation

**What's installed:**

- Weave.app in \`/Applications\` (GUI project creator)
- Project templates and CLI tool in \`/Library/Weave\`
- MayaFlux framework to \`/Library/MayaFlux\` (via post-install script)
- Dependencies via Homebrew (via post-install script)

---

</details>
