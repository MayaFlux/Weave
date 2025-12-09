The README by design is a high-level overview.
This document provides detailed information about the MayaFlux Weave installer,
what it installs, environment variables, project structure, building user projects,
configuration options, installation layouts, contributing guidelines, troubleshooting tips, and roadmap.

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

## Environment Variables (Set by Weave)

| Variable            | Value                     | Purpose                     |
| ------------------- | ------------------------- | --------------------------- |
| `MAYAFLUX_ROOT`     | Installation directory    | MayaFlux framework location |
| `CMAKE_PREFIX_PATH` | Includes `$MAYAFLUX_ROOT` | CMake package discovery     |
| `PATH`              | Includes MayaFlux `bin/`  | CLI tools and executables   |
| `VULKAN_SDK`        | Vulkan installation path  | GPU compute support (macOS) |

---

## Generated Project Structure

Users get this when creating a new project:

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
    └── user_project.hpp        # User's MayaFlux code
```

### Key Files to Edit

- **`src/user_project.hpp`** — Define `settings()` function (sample rate, buffer size, graphics) and `compose()` function (audio/visual processing)
- **`CMakeLists.txt`** — Add custom sources, additional dependencies, or compiler flags
- **`.vscode/settings.json`** — Customize editor behavior, language server, formatting

---

## Building User Projects

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

## Configuration Options (User Guide)

### Custom CMake Options

Users can edit generated `CMakeLists.txt`:

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

Before running applications, users can set:

```bash
# Enable detailed logging
export MAYAFLUX_DEBUG=1

# Set custom sample rate (48000 is default)
export MAYAFLUX_SAMPLE_RATE=96000

# Enable graphics debugging
export MAYAFLUX_GRAPHICS_DEBUG=1
```

---

## Installation Directory Layouts

### macOS

```
/Library/Weave/             - Templates and CLI tool
/Applications/Weave.app     - Weave project creator GUI
~/.local/bin/weave          - Symlink to CLI tool
~/.zshenv                   - Environment variables added here
```

MayaFlux installed via Homebrew (location determined by Homebrew).

### Windows

```
C:\MayaFlux\
├── bin/                    - Executables and DLLs
├── lib/                    - Libraries and CMake configs
├── include/                - Headers
└── share\weave\            - Templates and scripts
    ├── templates\
    └── scripts\            - install_package.ps1, packages.psd1
```

Log file location: `%LOCALAPPDATA%\weave_install.log`

### Linux

```
~/.local/Weave-X.X.X/      - Weave application, CLI, templates
~/.local/bin/weave         - Symlink to CLI tool
```

MayaFlux installed via AUR (Arch), COPR (Fedora), or CI binaries (others) to system paths or `~/MayaFlux` (fallback).

---

## Contributing to Weave

Weave is part of the MayaFlux ecosystem. Contributions are welcome!

### Before Contributing

- Test installers on fresh systems
- Update relevant documentation
- Follow code style guidelines for your language
- Include proper error handling

### Common Development Tasks

#### Adding a New Installation Step (Windows)

1. Create new class implementing `IInstallationStep`
2. Implement `BuildUI()` method using `LayoutManager`
3. Implement async `Execute()` method for actual work
4. Add to steps array in `InstallationMode.cs`

#### Adding a New Project Template File

1. Add file to `templates/` directory
2. Update `Weave.csproj` to embed as resource (Windows)
3. Verify extraction in test

#### Updating Dependency Versions

1. Update in appropriate manifest:
   - `packages.psd1` (Windows)
   - `install_deps.sh` (Linux)
   - `create_pkg.sh` (macOS)
2. Test on clean system
3. Update documentation if needed

---

## Roadmap

| Phase        | Timeline | Goals                                                                                    |
| ------------ | -------- | ---------------------------------------------------------------------------------------- |
| **Phase 1**  | Now      | macOS & Windows installers ✓, GUI tools, CLI (macOS/Linux), template system              |
| **Phase 2**  | Q4 2025  | Linux installer (completed), enhanced dependency management                              |
| **Phase 3**  | Q1 2026  | Weave package manager for community templates, plugin registry                           |
| **Phase 4+** | TBD      | Self-updating installers, advanced dependency resolution, cross-platform synchronization |

### macOS: "Weave.app won't open"

```bash
sudo xattr -rd com.apple.quarantine /path/to/Weave.app
```

### Windows: Build fails with resource embedding

- Verify `.csproj` has correct `<EmbeddedResource>` entries
- Check file paths are relative and exist
- Clean and rebuild solution

### Linux: GTK4 not found

Install GTK4 development files:

```bash
# Arch
sudo pacman -S gtk4

# Fedora
sudo dnf install gtk4-devel

# Ubuntu
sudo apt install libgtk-4-dev
```

### All platforms: CMake version issues

Ensure CMake 3.25 or newer:

```bash
cmake --version
```

---

## Links

- **[macOS Guide](MACOS.md)** - Installation and usage
- **[Windows Guide](WINDOWS.md)** - Installation and usage
- **[Linux Guide](LINUX.md)** - Installation and usage
- **[FAQ](FAQ.md)** - Common questions
- **[MayaFlux Framework](https://github.com/MayaFlux/MayaFlux)** - Core project
