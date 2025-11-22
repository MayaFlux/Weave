# Weave

**Project initialization, dependency management, and installation framework for MayaFlux.**

Weave handles everything needed to get MayaFlux running on your system: downloading the latest framework, installing dependencies, managing environment setup, and providing tools to scaffold new projects.

---

## Overview

MayaFlux is a unified multimedia processing architecture. Weave ensures you can **install it, configure it, and start building with it**—across macOS and Windows.

Instead of juggling separate downloads, manual dependency installation, and environment configuration, Weave automates the entire setup process while providing both GUI and CLI tools for project creation.

### What Weave Does

- **Downloads and installs MayaFlux** from the latest GitHub release
- **Manages all dependencies** (build tools, graphics libraries, audio backends, development SDKs)
- **Configures environment variables** for seamless development
- **Provides project creation tools** (GUI on both platforms, CLI on macOS) to scaffold new MayaFlux applications
- **Handles platform-specific setup** with intelligent fallbacks and validation

---

## Current Platform Support

| Platform          | Status  | Installer               | Installation       | Project Creator                   |
| ----------------- | ------- | ----------------------- | ------------------ | --------------------------------- |
| **macOS 14+**     | ✓ Ready | `.pkg` package          | Terminal + GUI     | `Weave.app` (GUI) + `weave` (CLI) |
| **Windows 10/11** | ✓ Ready | `.exe` (self-contained) | GUI (step-by-step) | `Weave.exe` (GUI)                 |
| **Linux**         | ✓ Ready | `bin` (self-contained)  | GUI (step-by-step) | `Weave` (GUI) + `Weave` (CLI)     |

---

## Quick Start

### macOS

1. **Download** the latest `Weave-X.X.X.pkg` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. **Double-click** to install (administrator password required)
3. **Terminal opens automatically** showing installation progress in real-time
4. **Weave.app** launches when complete—create your first project
5. **Build and run**:
   ```bash
   cd MyProject
   mkdir build && cd build
   cmake .. && make
   ./MyProject
   ```

### Windows

1. **Download** the latest `Weave-X.X.X.exe` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. **Double-click** to launch (UAC prompt for administrator privileges)
3. **Select a mode:**
   - "Install MayaFlux" - Downloads framework and dependencies
   - "Create Project" - Opens project creator GUI
4. **Follow the step-by-step installer** with progress tracking
5. **Build and run**:
   ```powershell
   cd MyProject
   mkdir build
   cd build
   cmake .. -DCMAKE_BUILD_TYPE=Release
   cmake --build . --config Release
   .\Release\MyProject.exe
   ```

### macOS Command Line (after installation)

```bash
# Create a new project
weave new MyProject ~/Projects/

# With live coding enabled
weave new MyProject ~/Projects/ --with-lila

# Skip VS Code setup
weave new MyProject ~/Projects/ --no-vscode
```

---

## Detailed Installation Guides

### macOS Installation Flow

The `.pkg` installer uses a composite package design with three transparent components orchestrated by the system:

1. **Weave-files** - Templates and CLI tool
2. **Weave-gui** - Universal Weave.app (arm64 + x86_64)
3. **Weave-launcher** - Postinstall coordinator

**What you see when you double-click the installer:**

1. Standard macOS installer dialog appears
2. Select installation location and click "Install"
3. **Terminal window opens automatically** (this is intentional)
4. Real-time progress appears showing:
   - Homebrew verification/installation
   - MayaFlux framework download (~100+ MB from GitHub)
   - Dependency installation (ffmpeg, rtaudio, glfw, eigen, etc.)
   - STB headers and Vulkan SDK setup
   - Environment configuration
5. Installation complete message
6. Close Terminal when finished

**Important notes:**

- The Terminal stays open intentionally—you see what's happening
- Total time: 10-30 minutes (varies with internet speed)
- All output also logged to `~/.weave_install.log` for reference
- Password may be requested by Homebrew—provide it when prompted

**Installation directories:**

```
/Library/Weave/             - Templates and CLI tool
/Applications/Weave.app     - Weave project creator GUI
/Library/MayaFlux/          - MayaFlux framework
~/.local/bin/weave          - Symlink to CLI tool
~/.zshenv                   - Environment variables added here
```

**After installation:**
Restart your terminal or run `source ~/.zshenv` to load environment variables immediately.

---

### Windows Installation & Usage

Windows gets a single, self-contained `Weave.exe` that handles all installation and project creation through a professional GUI interface.

#### Single Executable Design

- **Size:** ~15-20 MB (includes templates, dependency scripts, .NET 8 runtime)
- **Self-contained:** No external prerequisites (just Windows 10/11 64-bit)
- **Embedded resources:** All templates and PowerShell dependency scripts are built into the executable
- **Architecture:** .NET WinForms with dark theme, global layout manager for consistent UI

#### Installation Workflow

**Step 1: Launch and Mode Selection**

1. Double-click `Weave-X.X.X.exe`
2. UAC (User Access Control) dialog appears requesting administrator privileges
3. **Mode selection dialog** appears:
   - "Install MayaFlux" - Fresh installation or update
   - "Create Project" - Add a new project to existing installation

#### Install MayaFlux Mode (Detailed Steps)

**Step 1: System Check**

- Verifies Windows 64-bit (32-bit not supported)
- Checks for 7-Zip (required for extraction)
- Verifies administrator privileges
- Shows warnings if issues detected (installation continues anyway)

**Step 2: Download MayaFlux**

- Connects to GitHub API to fetch latest release
- Downloads framework binary (~100+ MB)
- **Real-time progress bar** shows download status
- Extracts to `C:\MayaFlux\` (default, configurable during installation)
- Verifies DLL files present and valid

**Step 3: Install Dependencies**

- Runs embedded `install_package.ps1` PowerShell script
- Intelligently detects already-installed packages (skips those)
- Installs build tools and libraries:
  - Build Tools: CMake, Git, Ninja, 7-Zip
  - Graphics: Vulkan SDK, GLFW
  - Audio: FFmpeg, RtAudio
  - Math/Utilities: LLVM, Eigen3, GLM, STB, MagicEnum
- Takes 10-20 minutes depending on internet speed
- Can be skipped if you have dependencies already installed

**Step 4: Environment Setup**

- Sets `MAYAFLUX_ROOT` environment variable
- Adds MayaFlux to system `PATH`
- Sets `CMAKE_PREFIX_PATH` for CMake discovery
- **Important:** Close and reopen your terminal/PowerShell for changes to take effect

**Step 5: Templates Installation**

- Extracts embedded project templates to `C:\MayaFlux\share\weave\templates\`
- Validates template integrity

**Step 6: Completion**

- Shows installation summary with all paths
- Button to view detailed installation log
- Ready to create projects

**Installation directories:**

```
C:\MayaFlux\
├── bin/              - Executables and DLLs
├── lib/              - Libraries and CMake configs
├── include/          - Headers
└── share\weave\      - Templates and scripts
    ├── templates\
    └── scripts\      - install_package.ps1, packages.psd1
```

**Log file location:** `%LOCALAPPDATA%\weave_install.log`

#### Create Project Mode

1. Run `Weave.exe` again and select "Create Project" (or skip installation if already done)
2. **Project Creator GUI** opens with:
   - **Project Name** - Text input (e.g., "AudioVisualizer")
   - **Project Location** - Path browser with Browse button
   - **Enable Live Coding (Lila)** - Optional checkbox
   - **Configure for VS Code** - Checkbox (enabled by default)
   - **Output log** - Real-time display of project generation

3. Click **"Create Project"** to scaffold new project
4. Success dialog shows project location and build instructions

#### What Gets Installed on First Run

| Component                 | Details                              |
| ------------------------- | ------------------------------------ |
| **MayaFlux Framework**    | Core library, headers, CMake configs |
| **Build Tools**           | CMake, Git, Ninja, 7-Zip             |
| **Graphics Stack**        | Vulkan SDK, GLFW, GLM                |
| **Audio Stack**           | FFmpeg, RtAudio                      |
| **Development Libraries** | LLVM, Eigen3, STB, MagicEnum, oneDPL |

**Environment variables configured:**

- `MAYAFLUX_ROOT` = installation directory (e.g., `C:\MayaFlux`)
- `PATH` += `%MAYAFLUX_ROOT%\bin`
- `CMAKE_PREFIX_PATH` = installation directory

**System impact:**

- Clean integration via environment variables only
- No registry entries
- No services or background processes
- Easy uninstall via Windows Settings

---

## Weave.app (macOS GUI)

Weave.app is a universal macOS application for creating new projects with a friendly graphical interface.

### Launch Methods

- **Automatically** after installation completes
- From `/Applications/Weave.app`
- From Spotlight search (`cmd+space`, type "Weave")
- From dock after first launch

### Architecture

- **Universal binary:** Optimized for both Apple Silicon (arm64) and Intel (x86_64) Macs
- **Built with:** SwiftUI and AppKit frameworks
- **Size:** ~50 MB app bundle
- **No external dependencies:** Self-contained, included in `.pkg` installer

### Using Weave.app

1. **Enter project details:**
   - Project name
   - Target directory (click "Browse..." to select)

2. **Optional settings:**
   - Enable Lila (live coding JIT compiler)
   - Configure for VS Code

3. **Click "Create Project"**
   - Real-time output shows what's being generated
   - Success/error feedback in dialog

4. **Build your project:**
   ```bash
   cd <project-directory>
   mkdir build && cd build
   cmake .. && make
   ./MyProject
   ```

---

## Project Creation

### Generated Project Structure

```
MyProject/
├── CMakeLists.txt              # Build configuration (auto-detects MayaFlux)
├── README.md                   # Project documentation
├── .vscode/                    # VS Code configuration (optional)
│   ├── settings.json           # C++ language server, IntelliSense
│   ├── tasks.json              # Build tasks (Configure, Build, Run)
│   └── launch.json             # Debug launch configurations
└── src/
    ├── main.cpp                # Application entry point
    └── user_project.hpp        # Your MayaFlux code
```

### Key Files to Edit

- **`src/user_project.hpp`** — Define `settings()` function (sample rate, buffer size, graphics) and `compose()` function (your audio/visual processing)
- **`CMakeLists.txt`** — Add custom sources, additional dependencies, or compiler flags
- **`.vscode/settings.json`** — Customize editor behavior, language server, formatting

---

## Building Projects

### From Command Line

```bash
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
```

### Cross-Platform CMakeLists.txt Behavior

Generated projects automatically handle platform differences:

**macOS/Linux:**

- Uses rpath to embed library search paths in executable
- Libraries found automatically at runtime
- No need to set `LD_LIBRARY_PATH`

**Windows:**

- Post-build step copies MayaFlux DLLs to executable output directory
- Executables run without additional PATH setup
- Useful for distribution (all files in one folder)

**MayaFlux Discovery (all platforms):**

- Checks `MAYAFLUX_ROOT` environment variable (set by Weave installer)
- Falls back to standard install locations
- Provides clear error message if not found

### With VS Code (if configured)

1. Open project folder in VS Code
2. Tasks → Run Task → "Build Project"
3. Press F5 to debug with integrated debugger (lldb on macOS, MSVC on Windows)

### Platform-Specific Run Commands

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

Weave automatically configures these variables during installation:

| Variable            | Value                     | Purpose                     |
| ------------------- | ------------------------- | --------------------------- |
| `MAYAFLUX_ROOT`     | Installation directory    | MayaFlux framework location |
| `CMAKE_PREFIX_PATH` | Includes `$MAYAFLUX_ROOT` | CMake package discovery     |
| `PATH`              | Includes MayaFlux `bin/`  | CLI tools and executables   |
| `VULKAN_SDK`        | Vulkan installation path  | GPU compute support (macOS) |

**To reload environment after installation:**

**macOS/Linux:**

```bash
source ~/.zshenv
# Or restart your terminal
```

**Windows:**

```powershell
# Close PowerShell/CMD and reopen to reload environment
# Or run: refreshenv (in some terminals)
```

**To verify environment:**

```bash
# macOS/Linux
echo $MAYAFLUX_ROOT

# Windows
echo %MAYAFLUX_ROOT%
```

---

## What Gets Installed

### MayaFlux Framework

- Core library (libMayaFluxLib / MayaFluxLib.dll)
- Lila JIT compiler for live code modification
- Headers and CMake configuration files

### Build Tools & Languages

- CMake 3.25+
- Git
- C++23 compiler (GCC/Clang/MSVC)
- Ninja build system

### Graphics & Rendering

- Vulkan SDK (GPU compute support)
- GLFW (windowing and input)
- GLM (mathematics library)

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

- libxml2 (data serialization)
- 7-Zip (archive handling)

---

## Troubleshooting

### Installation Issues

**macOS: "Weave.app won't open" or "damaged application"**

```bash
# Remove quarantine attribute
sudo xattr -rd com.apple.quarantine /Applications/Weave.app
```

**macOS: Installer hangs or is slow**

- Check internet connection (dependencies are downloaded)
- Homebrew may prompt for password—provide it when requested
- Installation time varies: 10-30 minutes typically
- Check progress: `tail -f ~/.weave_install.log` in another terminal

**Windows: "Administrator privileges required"**

- Right-click `Weave-*.exe` → "Run as administrator"
- Or enable UAC if it's disabled in your system

**Windows: PowerShell execution policy error**

- This is handled automatically by Weave
- If you see errors, check `%LOCALAPPDATA%\weave_install.log`

### Build Issues

**"MayaFlux not found" during CMake**

```bash
# Verify installation location
echo $MAYAFLUX_ROOT    # macOS/Linux
echo %MAYAFLUX_ROOT%   # Windows

# If empty, reload environment:
source ~/.zshenv       # macOS/Linux
# Or restart terminal (Windows)

# Or set manually:
export MAYAFLUX_ROOT=/path/to/MayaFlux    # macOS/Linux
set MAYAFLUX_ROOT=C:\path\to\MayaFlux      # Windows CMD
```

**"Command 'weave' not found" (macOS)**

- Ensure `~/.local/bin` is in `PATH`
- Restart terminal or run `source ~/.zshenv`
- Check installation log: `cat ~/.weave_install.log`

**Compiler errors with C++23 features**

- Ensure modern compiler:
  - GCC 12+
  - Clang 16+
  - MSVC 2022+
- Verify `CMAKE_CXX_STANDARD=23` in generated `CMakeLists.txt`

**"weave: command not found" but installation said it succeeded (macOS)**

- Run: `source ~/.zshenv` to load environment variables
- Or restart your terminal completely

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

## Uninstallation

### macOS

```bash
# Remove Weave.app
rm -rf /Applications/Weave.app

# Remove MayaFlux framework and dependencies
rm -rf /Library/MayaFlux
rm -rf /Library/Weave

# Remove CLI tool
rm ~/.local/bin/weave

# Remove environment setup (optional)
nano ~/.zshenv
# Find and remove lines starting with: export MAYAFLUX_ROOT, CMAKE_PREFIX_PATH, etc.
```

Note: Homebrew packages installed as dependencies are NOT removed. To clean those:

```bash
brew autoremove    # Removes packages no longer needed
```

### Windows

**Option 1: Settings GUI**

1. Settings → Apps → Apps & Features
2. Find "Weave" → Click "Uninstall"
3. Confirm removal

**Option 2: Command Line**

```powershell
wmic product where name="Weave" call uninstall
```

**Optional: Remove installation directory**

```powershell
# Remove MayaFlux directory (all files)
Remove-Item -Path "C:\MayaFlux" -Recurse -Force
```

---

## Building Weave from Source

For developers contributing to Weave itself:

### Prerequisites

- Git
- CMake 3.25+
- C++20 compiler
- Platform-specific tools:
  - **macOS:** Xcode Command Line Tools, Homebrew, Swift 5.9+
  - **Windows:** Visual Studio Build Tools 2022, WinGet, .NET 8 SDK
  - **Linux:** GCC 12+, standard build utils

### Clone and Build

```bash
git clone https://github.com/MayaFlux/Weave.git
cd Weave

# macOS
./macos/scripts/create_pkg.sh 0.1.1
# Output: build/macos/Weave-0.1.1.pkg

# Windows
.\windows\scripts\create_installer.ps1 -Version 0.1.1
# Output: build\windows\Weave-0.1.1.exe

# Linux
./linux/scripts/create_installer.sh 0.1.1
```

### Project Structure

```
Weave/
├── macos/
│   ├── scripts/
│   │   ├── build_weave_app.sh      # Builds universal Weave.app
│   │   ├── create_pkg.sh            # Packages .pkg installer
│   │   └── complete_installation.sh # Real-time installation script
│   ├── Weave.sh                     # CLI tool source
│   ├── WeaveGUI.swift               # Weave.app GUI source
│   └── resources/
│       ├── Distribution.xml         # Installer orchestration
│       ├── welcome.html
│       └── conclusion.html
├── windows/
│   ├── scripts/
│   │   ├── create_installer.ps1     # Main build script
│   │   ├── install_package.ps1      # Generic dependency installer
│   │   └── packages.psd1            # Dependency definitions
│   ├── Weave/                       # Main C# project
│   │   ├── Program.cs               # Entry point
│   │   ├── MainWindow.cs            # Installation UI
│   │   ├── UI/
│   │   │   ├── Pages/               # Installation steps
│   │   │   ├── Layout/              # Global layout manager
│   │   │   └── Project/             # Project creator UI
│   │   └── Weave.csproj
│   ├── Shared/                      # Shared models and utilities
│   └── Weave.sln                    # Solution file
├── templates/
│   ├── CMakeLists.txt               # Project template
│   ├── main.cpp
│   ├── user_project.hpp
│   └── vscode/
│       ├── settings.json
│       ├── tasks.json
│       └── launch.json
├── .github/workflows/
│   ├── build.yml                    # CI/CD for all platforms
│   └── scripts/
│       ├── verify-macos-build.sh
│       └── generate-release-body.sh
└── README.md (this file)
```

---

## CI/CD Integration

Weave includes GitHub Actions workflows for automated builds:

### macOS Workflow (`build.yml`)

- Builds Weave.app as universal binary (arm64 + x86_64)
- Creates `.pkg` installer with orchestrated components
- Verifies binary architecture with `lipo`
- Generates SHA256 hashes for distribution
- Uploads artifacts to GitHub Releases

### Windows Workflow (`build.yml`)

- Compiles Weave.exe with .NET 8 SDK
- Creates self-contained executable
- Verifies all embedded resources present
- Generates SHA256 hashes
- Uploads to GitHub Releases

### Release Workflow

Trigger automated builds with Git tags:

```bash
git tag v0.2.0
git push origin v0.2.0
# Workflows automatically build and publish
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

- Test installers on fresh systems before submitting
- Update documentation for new features
- Follow existing code style:
  - Swift for macOS (SwiftUI patterns)
  - C# for Windows (WinForms best practices)
  - Bash/Zsh for shell scripts
- Include proper error handling and user feedback
- Verify both GUI and CLI tools work correctly

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

---

## Documentation

- **[MayaFlux Docs](https://github.com/MayaFlux/MayaFlux)** — Framework architecture, tutorials, API reference

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

| Phase        | Timeline | Goals                                                                                    |
| ------------ | -------- | ---------------------------------------------------------------------------------------- |
| **Phase 1**  | Now      | macOS & Windows installers ✓, GUI tools, CLI (macOS), template system                    |
| **Phase 2**  | Q4 2025  | Linux installer (completed), Windows CLI tool, enhanced dependency management            |
| **Phase 3**  | Q1 2026  | Weave package manager for community templates, plugin registry                           |
| **Phase 4+** | TBD      | Self-updating installers, advanced dependency resolution, cross-platform synchronization |

---

**Weave makes getting started with MayaFlux effortless. Download, run, create, build.**
