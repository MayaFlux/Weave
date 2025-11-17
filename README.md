# Weave

**Project initialization, dependency management, and installation framework for MayaFlux.**

Weave handles everything needed to get MayaFlux running on your system: downloading the latest framework, installing dependencies, managing environment setup, and providing tools to scaffold new projects.

---

## Overview

MayaFlux is a unified multimedia processing architecture. Weave ensures you can **install it, configure it, and start building with it**—across macOS, Windows, and Linux.

Instead of juggling separate downloads, manual dependency installation, and environment configuration, Weave automates the entire setup process while providing both GUI and CLI tools for project creation.

### What Weave Does

- **Downloads and installs MayaFlux** from the latest GitHub release
- **Manages all dependencies** (build tools, graphics libraries, audio backends, development SDKs)
- **Configures environment variables** for seamless development
- **Provides project creation tools** (GUI and CLI) to scaffold new MayaFlux applications
- **Handles platform-specific setup** with intelligent fallbacks and validation

---

## Current Platform Support

| Platform | Status | Installer | Project Creator |
|----------|--------|-----------|-----------------|
| **macOS 14+** | ✓ Ready | `.pkg` package | `Weave.app` (GUI) + `weave` CLI |
| **Windows 10/11** | ✓ Ready | `.exe` installer | `Weave.exe` (GUI) + `weave` CLI |
| **Linux** | 🔄 In Progress | Coming soon | Coming soon |

---

## Quick Start

### macOS

1. **Download** the latest `Weave-X.X.X.pkg` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. **Double-click** to install (administrator password required)
3. **Weave.app** launches automatically—create your first project
4. **Build and run**:
   ```bash
   cd MyProject
   mkdir build && cd build
   cmake .. && make
   ./MyProject
   ```

### Windows

1. **Download** the latest `Weave-X.X.X.exe` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. **Double-click** to install (administrator privileges required)
3. **Weave Project Creator** launches automatically
4. **Build and run**:
   ```powershell
   cd MyProject
   mkdir build
   cd build
   cmake .. -DCMAKE_BUILD_TYPE=Release
   cmake --build . --config Release
   .\Release\MyProject.exe
   ```

### From Command Line

After installation, use the `weave` command to create projects:

```bash
# macOS / Linux
weave new MyProject ~/Projects/

# Windows
weave new MyProject C:\Projects\

# With live coding enabled
weave new MyProject ~/Projects/ --with-lila

# Skip VS Code setup
weave new MyProject ~/Projects/ --no-vscode
```

---

## What Gets Installed

### MayaFlux Framework
- **Core Library** (`libMayaFluxLib` / `MayaFluxLib.dll`)
- **Lila JIT Compiler** for live code modification
- **Headers and CMake configuration** for project integration

### Build Tools & Languages
- CMake 3.25+
- Git
- C++20 compiler (GCC/Clang/MSVC)
- Ninja build system

### Graphics & Rendering
- Vulkan SDK (GPU compute support)
- GLFW (windowing and input)
- GLM (math library)

### Audio Processing
- FFmpeg (media handling)
- RtAudio (real-time audio backend)
- Libsndfile (audio I/O)

### Development Libraries
- LLVM 21+ (for Lila JIT)
- Eigen (linear algebra)
- STB (image processing headers)
- MagicEnum (reflection utilities)
- oneDPL (parallel algorithms)

### Optional
- **libxml2** (data serialization)
- **7-Zip** (archive handling)

---

## Installation Details

### macOS

The `.pkg` installer includes two components:

1. **Weave-core.pkg** — MayaFlux framework, dependencies via Homebrew, environment setup
2. **Weave-gui.pkg** — `Weave.app` for the GUI project creator

**Installation directories:**
- System-wide: `/Library/MayaFlux/`
- User-level (recommended): `~/MayaFlux/`

**Environment setup:**
Variables are added to `~/.zshenv`:
```bash
export MAYAFLUX_ROOT="$HOME/MayaFlux"
export CMAKE_PREFIX_PATH="$MAYAFLUX_ROOT:$CMAKE_PREFIX_PATH"
```

**Manual installation:**
```bash
sudo installer -pkg Weave-X.X.X.pkg -target CurrentUserHomeDirectory
```

### Windows

The `.exe` installer handles:

1. **MayaFlux framework download** (from GitHub releases)
2. **Dependency installation** via WinGet and PowerShell automation
3. **Environment variable setup** (system-wide `MAYAFLUX_ROOT`, `PATH`, `CMAKE_PREFIX_PATH`)

**Installation directory:**
- `C:\MayaFlux\`

**Dependency handling:**
- Build tools installed via WinGet (CMake, Git, Ninja, 7-Zip)
- Graphics/audio libs auto-downloaded and extracted
- Visual Studio Build Tools detection and integration

**Automatic fallback:**
If WinGet isn't available, an embedded PowerShell script manages installation with detailed logging at `%LOCALAPPDATA%\weave_install.log`.

---

## Project Creation

### Using the GUI

**macOS:** Open `/Applications/Weave.app`  
**Windows:** Run `Weave.exe` (or find it in Start menu)

1. Enter project name (e.g., `AudioVisualizer`)
2. Select target directory
3. (Optional) Enable live coding (Lila)
4. (Optional) Configure VS Code integration
5. Click "Create Project"

### Using the CLI

```bash
weave new <project-name> [destination-directory] [options]

Options:
  --with-lila    Enable Lila JIT for live code modification
  --no-vscode    Skip VS Code configuration
  --help         Show usage information
```

**Examples:**
```bash
# Basic project
weave new BasicAudio ~/Projects/

# With live coding
weave new LiveCoding ~/Projects/ --with-lila

# Minimal setup (no IDE config)
weave new Minimal . --no-vscode
```

### Generated Project Structure

```
MyProject/
├── CMakeLists.txt              # Build configuration
├── README.md                   # Project documentation
├── .vscode/                    # VS Code settings (optional)
│   ├── settings.json           # Language server, formatting
│   ├── tasks.json              # Build tasks
│   └── launch.json             # Debug configuration
└── src/
    ├── main.cpp                # Application entry point
    └── user_project.hpp        # Your MayaFlux code
```

**Key files to edit:**
- `src/user_project.hpp` — Define `settings()` and `compose()` functions
- `CMakeLists.txt` — Add custom sources, dependencies, or compilation flags
- `.vscode/settings.json` — Customize editor behavior

---

## Building Projects

### From Source

```bash
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
```

### With VS Code (if configured)

1. Open project folder in VS Code
2. Tasks → Run Task → "Build Project"
3. Or press F5 to debug with gdb/lldb/MSVC

### Platform-Specific Commands

**macOS/Linux:**
```bash
./build/MyProject
```

**Windows:**
```powershell
.\build\Release\MyProject.exe
```

---

## Environment Variables

Weave sets up the following for you:

| Variable | Value | Purpose |
|----------|-------|---------|
| `MAYAFLUX_ROOT` | `~/MayaFlux` | MayaFlux installation root |
| `CMAKE_PREFIX_PATH` | Includes `$MAYAFLUX_ROOT` | CMake package discovery |
| `PATH` | Includes MayaFlux `bin/` | CLI tool access |
| `VULKAN_SDK` | Vulkan installation path | GPU compute support |
| `VK_LAYER_PATH` | Vulkan layer config | Validation layers |

**To reload environment (if installing manually):**
- **macOS/Linux:** `source ~/.zshenv`
- **Windows:** Restart terminal or `refreshenv` in PowerShell

---

## Troubleshooting

### Installation Issues

**"Weave.app won't open" (macOS)**
```bash
# Remove quarantine attribute
sudo xattr -rd com.apple.quarantine ~/Downloads/Weave-*.pkg

# Or use installer from command line
sudo installer -pkg Weave-*.pkg -target CurrentUserHomeDirectory
```

**"Administrator privileges required" (Windows)**
- Right-click `Weave-*.exe` → "Run as administrator"
- Or enable UAC if disabled

**Installation hangs or is slow**
- Check internet connection (dependencies are downloaded)
- For macOS: Homebrew may prompt for password—provide it when requested
- For Windows: PowerShell may need time to compile/extract dependencies
- Installation log: `~/.weave_install.log` (macOS) or `%LOCALAPPDATA%\weave_install.log` (Windows)

### Build Issues

**"MayaFlux not found" during CMake**
- Verify installation: `echo $MAYAFLUX_ROOT` (macOS/Linux) or `echo %MAYAFLUX_ROOT%` (Windows)
- If empty, reload environment (see Environment Variables section)
- Or set manually: `export MAYAFLUX_ROOT=/path/to/MayaFlux`

**"Command 'weave' not found"**
- Ensure `~/.local/bin` (macOS/Linux) or `%USERPROFILE%\.local\bin` (Windows) is in `PATH`
- Restart terminal after installation
- Check installation log for errors

**Compiler errors with C++23 features**
- Ensure modern compiler: GCC 12+, Clang 16+, or MSVC 2022+
- Verify `CMAKE_CXX_STANDARD=23` in CMakeLists.txt

---

## Building Weave from Source

For developers contributing to Weave itself:

### Prerequisites

- Git
- CMake 3.25+
- C++20 compiler
- Platform-specific tools:
  - **macOS:** Xcode Command Line Tools, Homebrew
  - **Windows:** Visual Studio Build Tools, WinGet
  - **Linux:** GCC 12+, standard build utils

### Clone and Build

```bash
git clone https://github.com/MayaFlux/Weave.git
cd Weave

# macOS
./macos/scripts/create_pkg.sh 0.1.0
# Output: build/macos/Weave-0.1.0.pkg

# Windows
.\windows\scripts\create_installer.ps1 -Version 0.1.0
# Output: build\windows\Weave-0.1.0.exe

# Linux (coming soon)
./linux/scripts/create_installer.sh 0.1.0
```

### Project Structure

```
Weave/
├── macos/
│   ├── scripts/
│   │   ├── build_weave_app.sh      # Builds Weave.app GUI
│   │   └── create_pkg.sh            # Packages .pkg installer
│   ├── Weave.sh                     # CLI tool (symlinked as `weave`)
│   ├── WeaveGUI.swift               # macOS app source
│   └── resources/
│       ├── Distribution.xml         # Installer metadata
│       ├── welcome.html
│       └── conclusion.html
├── windows/
│   ├── scripts/
│   │   ├── create_installer.ps1     # Main build script
│   │   ├── install_package.ps1      # Generic dependency installer
│   │   └── packages.psd1            # Dependency definitions
│   ├── gui/
│   │   └── WeaveGUI.cs              # Windows app source
│   ├── resources/
│   │   ├── welcome.html
│   │   └── conclusion.html
│   └── Weave.nsi                    # NSIS installer definition
├── linux/ (coming soon)
├── templates/
│   ├── CMakeLists.txt               # Project template
│   ├── main.cpp
│   ├── user_project.hpp
│   └── vscode/
│       ├── settings.json
│       ├── tasks.json
│       └── launch.json
├── .github/workflows/
│   ├── build-macos.yml              # macOS CI/CD
│   └── build-windows.yml            # Windows CI/CD
└── README.md (this file)
```

---

## Configuration

### Custom CMake Options

Edit generated `CMakeLists.txt` to customize:

```cmake
# Enable additional logging
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -DMAYAFLUX_DEBUG=1")

# Link against optional libraries
find_package(Eigen3 REQUIRED)
target_link_libraries(${PROJECT_NAME} PRIVATE Eigen3::Eigen)

# Add custom sources
target_sources(${PROJECT_NAME} PRIVATE
    src/custom_module.cpp
    src/audio_nodes.cpp
)
```

### Environment Customization

Before running your application, set MayaFlux-specific variables:

```bash
# Enable detailed logging
export MAYAFLUX_DEBUG=1

# Set custom sample rate (48000 is default)
export MAYAFLUX_SAMPLE_RATE=96000

# Enable graphics debugging
export MAYAFLUX_GRAPHICS_DEBUG=1
```

---

## CI/CD Integration

Weave includes GitHub Actions workflows for automated builds:

### macOS

```yaml
# .github/workflows/build-macos.yml
- Builds and signs Weave.app
- Creates notarized .pkg installer
- Uploads to GitHub Releases
```

### Windows

```yaml
# .github/workflows/build-windows.yml
- Compiles with NSIS
- Manages dependency packages
- Generates SHA256 hashes
```

Trigger releases with Git tags:
```bash
git tag v0.2.0
git push origin v0.2.0
# Workflows automatically build and publish
```

---

## Uninstallation

### macOS

```bash
# Remove MayaFlux
rm -rf ~/MayaFlux
rm -rf /Applications/Weave.app

# Clean environment (optional)
nano ~/.zshenv
# Remove MAYAFLUX_ROOT and related lines
```

### Windows

1. Settings → Apps → Apps & Features
2. Find "Weave" → Uninstall
3. (Optional) Remove `C:\MayaFlux\` folder

```powershell
# Or via command line
wmic product where name="Weave" call uninstall
```

---

## Contributing

Weave is part of the MayaFlux ecosystem. Contributions are welcome!

**To contribute:**
1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-improvement`)
3. Make changes and test on your platform
4. Submit a pull request with detailed description

**Development guidelines:**
- Test installers on fresh VMs before submitting
- Update documentation for new features
- Follow existing code style (Swift for macOS, C# for Windows, Bash for Linux)
- Include error handling and user feedback

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

---

## Documentation

- **[MayaFlux Docs](https://github.com/MayaFlux/MayaFlux)** — Framework architecture, tutorials, API reference
- **[Getting Started](../docs/Getting_Started.md)** — First MayaFlux project walkthrough
- **[Installation FAQ](docs/FAQ.md)** — Common questions and solutions
- **[Build Operations](docs/BuildOps.md)** — Advanced build and customization

---

## License

**GNU General Public License v3.0 (GPLv3)**

Weave is part of MayaFlux. See [LICENSE](LICENSE) for full terms.

---

## Support & Contact

- **Issues**: Report bugs on [GitHub Issues](https://github.com/MayaFlux/Weave/issues)
- **Discussions**: Join [GitHub Discussions](https://github.com/MayaFlux/Weave/discussions)
- **Installation Help**: See Troubleshooting section above or check installation logs
- **Framework Questions**: See [MayaFlux Repository](https://github.com/MayaFlux/MayaFlux)

---

## Roadmap

| Phase | Timeline | Goals |
|-------|----------|-------|
| **Phase 1** | Now | macOS & Windows installers ✓, CLI tool, project creator |
| **Phase 2** | Q4 2025 | Linux installer, improved dependency management, cross-platform CI/CD |
| **Phase 3** | Q1 2026 | Weave package manager for community templates, plugin registry |
| **Phase 4+** | TBD | Self-updating installers, advanced dependency resolution |

---

**Weave makes getting started with MayaFlux effortless. Download, run, create, build.**
