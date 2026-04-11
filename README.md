# Weave

**One-click installer and project creator for MayaFlux.**

Downloads the framework, installs dependencies, and gets you building in minutes.

---

## macOS

### Install

1. Download `Weave-macos.dmg` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. Open the DMG and double-click **Weave.app**
3. **If "unverified developer" warning appears:**
   - Close the warning
   - Go to **System Settings → Privacy & Security**
   - Scroll down to find **Weave**
   - Click **"Open Anyway"**
   - Double-click Weave.app again
4. Choose **"Install MayaFlux"**
5. Select a release channel and follow the prompts
6. When done, restart your terminal

**What gets installed:**

- MayaFlux framework (via Homebrew)
- `weave` CLI tool (`~/.local/bin/weave`)
- Environment variables in `~/.zshenv`

### Create & Build

```bash
# Using CLI
weave new MyProject ~/Projects/

# Or open Weave.app and choose "Create New Project"

# Build
cd MyProject && mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
./MyProject
```

**[Full macOS Guide](docs/MACOS.md)** - Troubleshooting, uninstall, etc.

---

## Linux

### Install

Download `Weave-X.X.X-linux.tar.gz` from [Releases](https://github.com/MayaFlux/Weave/releases) and extract:

```bash
tar -xzf Weave-X.X.X-linux.tar.gz -C ~/.local/
```

Run the installer:

```bash
~/.local/Weave-X.X.X/Weave
```

**Mode selection dialog:**

- Choose **"Install MayaFlux"** for fresh installation
- Choose **"Create Project"** if already installed

**Installation details:**

- **Arch Linux** - Automatically installs `mayaflux-dev-bin` from AUR
- **Fedora 43+** - Automatically installs `mayaflux-dev` from COPR
- **Ubuntu 25+, openSUSE Tumbleweed, other distros** - Automatically downloads and extracts MayaFlux to `~/MayaFlux`

**Note:** Requires GCC 15+, LLVM 21+, GLFW 3.4+. Check [Full Linux Guide](docs/LINUX.md) for distro compatibility.

### Create & Build

**Using GUI:**

```bash
~/.local/Weave-X.X.X/Weave
# Select "Create Project" mode
```

**Using CLI:**

```bash
weave new MyProject ~/Projects/
```

**Build:**

```bash
cd MyProject && mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
./MyProject
```

**[Full Linux Guide](docs/LINUX.md)** - Troubleshooting, uninstall, etc.

---

## Windows

### Install

1. Download `Weave-X.X.X.exe` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. Right-click → "Run as administrator"
3. **UAC prompt appears** - click "Yes"
4. **Mode selection dialog:**
   - Choose **"Install MayaFlux"** for fresh installation
   - Choose **"Create Project"** if already installed
5. **Follow step-by-step installer:**
   - System checks (verifies 64-bit Windows)
   - Downloads MayaFlux (~1.5 MB)
   - **Vulkan SDK installer launches** - click through installation
     - Select all components **except** ARM (optional: skip SDL2 if not needed)
     - All other components are required
   - Installs other dependencies (~90 MB total, includes bundled DLLs)
   - Sets environment variables
6. **Restart your terminal** when installation completes
7. Run `Weave.exe` again and select "Create Project"

**Installation takes longer on Windows** due to:

- Bundled DLLs (90 MB vs 1-2 MB on Mac/Linux)
- LLVM tarball extraction
- Manual Vulkan SDK installer intervention

### Create & Build

```powershell
# Using GUI
Weave.exe → "Create Project"

# Build
cd MyProject
mkdir build
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
.\Release\MyProject.exe
```

**[Full Windows Guide](docs/WINDOWS.md)** - Troubleshooting, uninstall, etc.

---

## Documentation

- **[macOS Guide](docs/MACOS.md)** - Detailed installation, troubleshooting, uninstall
- **[Linux Guide](docs/LINUX.md)** - Distribution compatibility, troubleshooting, uninstall
- **[Windows Guide](docs/WINDOWS.md)** - Step-by-step walkthrough, troubleshooting, uninstall
- **[Comprehensive overview](docs/PACKAGE.md)** - Common questions
- **[Development Guide](docs/DEVELOP.md)** - Building Weave from source, contributing

---

## Support

- **Issues:** [GitHub Issues](https://github.com/MayaFlux/Weave/issues)
- **Discussions:** [GitHub Discussions](https://github.com/MayaFlux/Weave/discussions)

---

## License

GNU General Public License v3.0 - See [LICENSE](LICENSE)
