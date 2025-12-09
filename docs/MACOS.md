# Weave for macOS

Complete guide for installing, using, and troubleshooting Weave on macOS.

## Installation

### Prerequisites

- **Apple Silicon:** macOS 14.0 (Sonoma) or later
- **Intel:** macOS 15.0 (Sequoia) or later
- ~2GB free disk space
- Internet connection
- Administrator access (for Homebrew)

### Install from Installer Package

1. **Download** `Weave-X.X.X.pkg` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. **Double-click** the `.pkg` file
3. **Standard macOS installer** appears - click "Install"
4. **Terminal window opens automatically** - this is intentional
   - Shows real-time progress of framework download and dependency installation
   - May ask for your password (Homebrew needs admin privileges)
   - Takes 10-30 minutes depending on internet speed
5. **When complete**, Terminal shows success message
6. **Close Terminal** when done
7. **Weave.app may launch** - you can now create your first project

### What Gets Installed

- **`/Library/Weave/`** - Project creator CLI tool and templates
- **`/Applications/Weave.app`** - Universal GUI application (arm64 + x86_64)
- **Homebrew** - Manages MayaFlux framework installation
- **`~/.local/bin/weave`** - Symlink to CLI tool
- **`~/.zshenv`** - Sources environment from Homebrew MayaFlux package

### Post-Installation

**Reload environment variables:**

```bash
source ~/.zshenv
```

Or simply restart your terminal.

**Verify installation:**

```bash
echo $MAYAFLUX_ROOT
# Should output the MayaFlux installation path (set by Homebrew)

weave --version
# Should show version number
```

---

## Creating Projects

### Using Weave.app (GUI)

1. Open `/Applications/Weave.app` (or find via Spotlight)
2. Enter **Project Name** (e.g., "MyFirstProject")
3. Click **"Browse..."** to select location
4. Optional: Enable "Live Coding (Lila)" or "Configure for VS Code"
5. Click **"Create Project"**
6. Success dialog shows your project location

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

### Manual Build

```bash
cd MyProject/build
cmake --build . --config Release --parallel 4
```

---

## Environment Variables

After installation, these are set in `~/.zshenv`:

| Variable            | Value                     | Purpose            |
| ------------------- | ------------------------- | ------------------ |
| `MAYAFLUX_ROOT`     | Set by Homebrew           | Framework location |
| `CMAKE_PREFIX_PATH` | Includes `$MAYAFLUX_ROOT` | CMake discovery    |
| `PATH`              | Includes MayaFlux bin     | CLI tools          |
| `VULKAN_SDK`        | Vulkan installation path  | GPU compute        |

**To apply immediately without restarting:**

```bash
source ~/.zshenv
```

---

## Troubleshooting

### Installer Hangs or Stalls

**Check progress in another terminal:**

```bash
tail -f ~/.weave_install.log
```

**Common causes:**

- Large dependency compilation (LLVM, FFmpeg, Vulkan SDK)
- Homebrew is fetching sources
- System is busy

**Solution:** Wait it out, or check the log file for actual errors.

### "weave: command not found"

**Cause:** Environment variables not loaded

**Fix:**

```bash
source ~/.zshenv
weave new MyProject
```

Or restart your terminal completely.

### "Weave.app won't open" or "damaged application"

**Fix:**

```bash
sudo xattr -rd com.apple.quarantine /Applications/Weave.app
```

Then try opening again.

### CMake can't find MayaFlux

**Verify environment is set:**

```bash
echo $MAYAFLUX_ROOT
# Should show: /Library/MayaFlux
```

**If empty, reload:**

```bash
source ~/.zshenv
```

**If still not found, set manually in CMake:**

```bash
cd build
cmake .. -DCMAKE_PREFIX_PATH=/Library/MayaFlux -DCMAKE_BUILD_TYPE=Release
```

### Build errors with C++23 features

**Ensure you have a modern compiler:**

```bash
clang++ --version  # Should be 15+
# or
g++ --version      # Should be 12+
```

**If compiler is too old, update via Homebrew:**

```bash
brew install llvm
# Then use: /usr/local/opt/llvm/bin/clang++
```

### "Missing architecture" errors

**Weave.app is universal (arm64 + x86_64).** If you get architecture errors building your project, ensure your project CMakeLists.txt doesn't force a specific architecture.

### Homebrew password prompt during install

**This is normal.** Homebrew needs admin privileges to install system libraries. Provide your password when prompted.

---

## Uninstalling

### Remove Everything

```bash
# Remove Weave app
rm -rf /Applications/Weave.app

# Remove installation files
rm -rf /Library/Weave

# Remove CLI symlink
rm ~/.local/bin/weave

# Remove environment setup (optional)
nano ~/.zshenv
# Find and delete lines sourcing MayaFlux environment file
```

### Remove Homebrew MayaFlux Package

```bash
# Remove MayaFlux framework
brew uninstall mayaflux-dev

# Remove all unused dependencies
brew autoremove
```

---

## FAQ

**Q: Why does installation take time?**

A: Homebrew acquires heavy dependencies (LLVM, FFmpeg, Vulkan SDK). Timeframe depends on your machine and whether these are already cached. If you already have these installed via Homebrew, installation is much faster.

**Q: Can I use Weave.app and the CLI tool together?**

A: Yes. Use whichever is more convenient for your workflow. Both create the same project structure.

---

## Links

- **[MayaFlux Framework](https://github.com/MayaFlux/MayaFlux)** - Learn the API
- **[Back to README](../README.md)** - Overview and quick start
