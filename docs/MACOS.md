# Weave for macOS

Complete guide for installing, using, and troubleshooting Weave on macOS.

## Installation

### Prerequisites

- macOS 15.0 (Sequoia) or later
- ~2GB free disk space
- Internet connection
- Administrator access (for Homebrew)

### Install from DMG

1. **Download** `Weave-macos.dmg` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. **Open** the DMG and double-click **Weave.app**
3. **If "unverified developer" warning appears:**
   - Close the warning
   - Go to **System Settings → Privacy & Security**
   - Scroll down to find **Weave**
   - Click **"Open Anyway"**
   - Double-click Weave.app again
4. Choose **"Install MayaFlux"**
5. Select a release channel (Stable recommended)
6. If Homebrew is not installed, enter your password when prompted
7. Wait for installation to complete — progress is shown in the app
8. Restart your terminal when done

### What Gets Installed

- **Homebrew** - manages the MayaFlux framework installation
- **MayaFlux framework** - installed via `brew install mayaflux/mayaflux/mayaflux`
- **`~/.local/bin/weave`** - CLI project creator tool
- **`$ZDOTDIR/.zshenv`** - updated to source MayaFlux environment and extend `PATH`

### Post-Installation

**Reload environment variables:**

```bash
source $ZDOTDIR/.zshenv
```

Or simply restart your terminal.

**Verify installation:**

```bash
weave --version
```

---

## Creating Projects

### Using Weave.app (GUI)

1. Open **Weave.app** (from the DMG, or move it to `/Applications` first)
2. Choose **"Create New Project"**
3. Enter a **Project Name** (e.g., "MyFirstProject")
4. Click **"Browse..."** to select a location
5. Optional: Enable **"Live Coding (Lila)"**
6. Click **"Create Project"**

### Using CLI Tool

```bash
# Basic project
weave new MyProject ~/Projects/

# With live coding enabled
weave new MyProject ~/Projects/ --with-lila

# Without VS Code setup
weave new MyProject ~/Projects/ --no-vscode
```

---

## Building & Running

### Quick Start

```bash
cd MyProject
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
./MyProject
```

### With VS Code

1. Open project folder: `code .`
2. VS Code should auto-detect the build configuration
3. Terminal → Run Task → "Build Project"
4. Press F5 to run with debugger (lldb)

---

## Environment Variables

After installation, these are added to `$ZDOTDIR/.zshenv`:

```zsh
source "<homebrew-prefix>/env.sh"
export PATH="$HOME/.local/bin:$PATH"
```

The `env.sh` sourced from the Homebrew prefix sets any variables the framework requires (e.g. `MAYAFLUX_ROOT`, `CMAKE_PREFIX_PATH`, `VULKAN_SDK`).

**To apply immediately without restarting:**

```bash
source $ZDOTDIR/.zshenv
```

---

## Troubleshooting

### "weave: command not found"

**Cause:** Environment variables not loaded yet.

**Fix:**

```bash
source $ZDOTDIR/.zshenv
```

Or restart your terminal.

### "Weave.app won't open" or "damaged application"

**Fix:**

```bash
xattr -rd com.apple.quarantine /path/to/Weave.app
```

Then try opening again.

### CMake can't find MayaFlux

**Verify environment is set:**

```bash
echo $MAYAFLUX_ROOT
```

**If empty, reload:**

```bash
source $ZDOTDIR/.zshenv
```

### Build errors with C++23 features

**Ensure you have a modern compiler:**

```bash
clang++ --version  # Should be 15+
```

**If too old, update via Homebrew:**

```bash
brew install llvm
```

### Homebrew password prompt during install

**This is normal.** Homebrew needs admin privileges to set up its directory structure on a fresh install. Your password is not stored.

---

## Uninstalling

```bash
# Remove MayaFlux framework
brew uninstall mayaflux
brew autoremove

# Remove CLI tool
rm ~/.local/bin/weave

# Remove environment config (optional)
# Edit $ZDOTDIR/.zshenv and delete the MayaFlux lines
```

---

## FAQ

**Q: Why does installation take a while?**

A: Homebrew is downloading and building the MayaFlux framework and its dependencies (LLVM, FFmpeg, Vulkan SDK, etc.). If these are already cached on your machine, it's much faster.

**Q: Can I use Weave.app and the CLI together?**

A: Yes. Both create the same project structure - use whichever suits your workflow.

**Q: Do I need to keep Weave.app after installing?**

A: Only if you want to create new projects via the GUI. The CLI (`weave`) works independently once installed.

---

## Links

- **[MayaFlux Framework](https://github.com/MayaFlux/MayaFlux)** - Learn the API
- **[Back to README](../README.md)** - Overview and quick start
