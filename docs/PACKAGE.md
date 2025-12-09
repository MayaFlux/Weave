# Weave Package & Configuration Guide

Comprehensive reference for dependencies, installation details, project configuration, and development.

---

## What Gets Installed

### MayaFlux Framework

- **Core library** - libMayaFluxLib (Unix) or MayaFluxLib.dll (Windows)
- **Lila JIT compiler** - Live code modification support
- **Headers** - Full C++23 API
- **CMake configuration** - Automatic dependency discovery via `find_package(MayaFlux)`

### Build Tools & Languages

- **CMake 3.25+** - Cross-platform build system
- **Git** - Version control
- **C++23 compiler** - GCC 12+ (Linux), Clang 15+ (macOS), MSVC 2022+ (Windows)
- **Ninja build system** - Optional faster build backend

### Graphics & Rendering (Required)

- **Vulkan SDK** - GPU compute and graphics
  - Includes vulkan headers, loaders, layers, validation tools
  - Windows: Installer-guided setup
  - macOS/Linux: Automatic via package manager
- **GLFW 3.4+** - Windowing and input handling
- **GLM** - Vector/matrix mathematics

### Audio Processing (Required)

- **FFmpeg** - Audio/video encoding and decoding
  - libavcodec, libavformat, libavutil, libswresample, libswscale
- **RtAudio** - Real-time audio I/O backend
  - Windows: WASAPI
  - macOS: Core Audio
  - Linux: ALSA, JACK, PulseAudio

### Development Libraries (Required)

- **LLVM 21+** - Compiler framework (for Lila JIT)
- **Clang** - C++ compiler frontend (for Lila code generation)
- **Eigen3** - Linear algebra library
- **STB headers** - Image and audio processing
- **MagicEnum** - Type reflection utilities
- **oneDPL** (macOS) / **TBB** (Linux) - Parallel algorithms
- **libxml2** - XML serialization (via CMakeLists.txt)

### Platform-Specific Threading Backends

- **Windows** - PPL (built into MSVC, no separate install needed)
- **macOS** - oneDPL (Installed via Homebrew)
- **Linux** - TBB (Threading Building Blocks)

**All of these are required.** There are no "optional" dependencies in MayaFlux. Every package listed above is necessary for the framework to function properly.

---

## Installation Directory Layouts

### macOS

```
/Library/Weave/                 - Weave CLI tool and templates
  └── templates/
      ├── CMakeLists.txt
      ├── main.cpp
      ├── user_project.hpp
      └── vscode/
/Applications/Weave.app         - Weave project creator GUI
~/.local/bin/weave              - Symlink to CLI tool
~/.zshenv                       - Environment variables (sources Homebrew MayaFlux env)

# MayaFlux location (set by Homebrew, varies):
# Usually: /opt/homebrew/opt/mayaflux-dev/ (Apple Silicon)
# Or:      /usr/local/opt/mayaflux-dev/ (Intel)
# Or:      /Library/MayaFlux/ (if installed separately)
```

### Windows

```
C:\MayaFlux\
├── bin\                        - Executables and runtime DLLs
│   ├── MayaFluxLib.dll
│   ├── Lila.dll
│   ├── llvm-lib.exe
│   ├── clang-cl.exe
│   └── ...
├── lib\                        - Import libraries and CMake configs
│   ├── MayaFluxLib.lib
│   ├── Lila.lib
│   └── cmake/
│       └── MayaFlux/
│           ├── MayaFluxConfig.cmake
│           └── MayaFluxConfigVersion.cmake
├── include\                    - Public headers
│   └── MayaFlux/
│       ├── MayaFlux.hpp        - Main include
│       └── ...
└── share\
    └── MayaFlux/
        ├── shaders/
        └── runtime/
└── share\weave\               - Weave project templates and scripts
    ├── templates/
    └── scripts/
        ├── install_package.ps1
        └── packages.psd1

C:\VulkanSDK\X.X.X\          - Vulkan SDK installation

C:\Program Files\...           - Build tools and libraries
├── LLVM_Libs\                 - LLVM libraries
├── FFmpeg\                    - Audio/video library
├── glfw\                      - Window/input library
├── RtAudio\                   - Real-time audio I/O
└── ...
```

### Linux

```
~/.local/Weave-X.X.X/          - Weave application
├── lib/
│   ├── main.py
│   ├── cli.py
│   ├── config.py
│   ├── modes/
│   ├── ui/
│   └── templates -> ../templates
├── scripts/
│   ├── create_project.sh
│   ├── install_deps.sh
│   └── build_distribution.sh
├── Weave                       - Launcher script
└── weave-config.json

~/.local/bin/weave             - Symlink to Weave launcher

~/MayaFlux/                    - MayaFlux framework (or via package manager)
├── bin/
├── lib/
    └── cmake/
        └── MayaFlux/
├── include/
└── share/
    └── MayaFlux/
        └── shaders/


# OR installed via package manager:
# Arch: /usr/lib, /usr/include (from mayaflux-dev-bin AUR)
# Fedora: /usr/lib64, /usr/include (from mayaflux-dev COPR)
```

---

## Environment Variables

Weave automatically sets these during installation:

| Variable            | Value                                 | Purpose                                 |
| ------------------- | ------------------------------------- | --------------------------------------- |
| `MAYAFLUX_ROOT`     | Installation directory (see layouts)  | Framework root for CMake discovery      |
| `CMAKE_PREFIX_PATH` | Includes `$MAYAFLUX_ROOT`             | Allows `find_package(MayaFlux)` to work |
| `PATH`              | Includes MayaFlux `bin/` directory    | CLI tools and build executables         |
| `VULKAN_SDK`        | Vulkan installation path (macOS only) | GPU compute support                     |

**To reload after installation (without restarting):**

```bash
# macOS/Linux
source ~/.zshenv
# or
source ~/.bashrc

# Windows
# Close and reopen PowerShell/CMD
```

---

## Generated Project Structure

Users receive this structure when creating a new project:

```
MyProject/
├── CMakeLists.txt              # Build configuration (auto-detects MayaFlux)
├── README.md                   # Project documentation
├── .gitignore                  # Version control exclusions
├── .vscode/                    # VS Code configuration (optional)
│   ├── settings.json           # C++ language server and formatting
│   ├── tasks.json              # Build tasks (Configure, Build, Run)
│   └── launch.json             # Debug launch configurations
└── src/
    ├── main.cpp                # Application entry point
    └── user_project.hpp        # Your MayaFlux code (empty template)
```

### Key Files to Edit

- **`src/user_project.hpp`** — Define two functions:
  - `settings()` — Configure sample rate, buffer size, graphics, logging
  - `compose()` — Build your audio/visual processing logic

- **`CMakeLists.txt`** — Customize build:
  - Add additional `.cpp` source files
  - Link external libraries
  - Set custom compiler flags
  - Configure Lila live coding support

- **`.vscode/settings.json`** — Adjust IDE behavior:
  - C++ language server settings
  - Code formatting rules
  - Include path hints

---

## Configuring the Generated CMakeLists.txt

The generated project CMakeLists.txt is designed to "just work" but is fully customizable.

### Adding Custom Source Files

```cmake
add_executable(${PROJECT_NAME}
    src/main.cpp
    src/user_project.hpp
    src/my_custom_module.cpp      # Add your files here
    src/audio_processor.cpp
    src/graphics_renderer.cpp
)
```

### Enabling Lila (Live Coding JIT)

If you created the project without Lila support, enable it manually:

```cmake
# Around line 70-80, look for the @LILA_LINK_BLOCK@ replacement section
# Replace the comment with:

if(TARGET MayaFlux::Lila)
    target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::Lila)
    message(STATUS "Lila live coding enabled")
else()
    message(WARNING "Lila not found - live coding disabled")
endif()
```

Then rebuild:

```bash
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
```

### Linking Additional Libraries

Example: Link against Eigen for custom linear algebra:

```cmake
find_package(Eigen3 REQUIRED)
target_link_libraries(${PROJECT_NAME} PRIVATE Eigen3::Eigen)
```

Example: Use fmt for string formatting:

```cmake
find_package(fmt REQUIRED)
target_link_libraries(${PROJECT_NAME} PRIVATE fmt::fmt)
```

### Setting Compiler Flags

```cmake
# Add optimization flags
target_compile_options(${PROJECT_NAME} PRIVATE
    $<$<CXX_COMPILER_ID:MSVC>:/O2 /arch:AVX2>
    $<$<NOT:$<CXX_COMPILER_ID:MSVC>>:-O3 -march=native>
)

# Add debug symbols
target_compile_options(${PROJECT_NAME} PRIVATE
    $<$<CONFIG:Debug>:-g3>
)

# Enable C++23 specific extensions (already set, but can adjust)
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
```

### Changing Build Output Directory

By default, binaries go to `build/` (or `build/Debug/` / `build/Release/` on Windows).

To customize:

```cmake
# At the top of CMakeLists.txt, after project() declaration:
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/lib")
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/lib")
```

### Advanced: Custom Installation Rules

To install your built application to a system location:

```cmake
install(TARGETS ${PROJECT_NAME}
    RUNTIME DESTINATION bin
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)

# Also install headers if your project is a library
install(DIRECTORY src/
    DESTINATION include/${PROJECT_NAME}
    FILES_MATCHING PATTERN "*.hpp"
)
```

Then build and install:

```bash
cd build
cmake --build . --parallel
cmake --install . --config Release --prefix ~/.local
```

---

## Building Projects

### Command Line (All Platforms)

```bash
# Create build directory
mkdir build && cd build

# Configure with Release optimization
cmake .. -DCMAKE_BUILD_TYPE=Release

# Build with parallel jobs
cmake --build . --parallel

# Run the application
# macOS/Linux:
./MyProject

# Windows:
.\Release\MyProject.exe
```

### Using VS Code

1. **Open project folder** — `code .`
2. **CMake extension** — Should auto-detect configuration
3. **Run build task** — Terminal → Run Task → "Build Project"
4. **Debug** — Press F5 (uses gdb/lldb/MSVC debugger)
5. **View output** — Check the output binary in `build/` directory

### Using Visual Studio (Windows)

1. **Open folder** — File → Open Folder → Select project directory
2. **CMake configuration** — VS detects CMakeLists.txt automatically
3. **Build** — Build → Build All (or Ctrl+Shift+B)
4. **Run** — Debug → Start Debugging (F5)
5. **Output** — Check `build/Debug/` or `build/Release/`

### Using Make (Unix-like)

```bash
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release -G "Unix Makefiles"
make -j$(nproc)
```

### Using Ninja (Faster)

```bash
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release -G Ninja
ninja
```

---

## Cross-Platform Build Behavior

The generated CMakeLists.txt automatically handles platform differences:

### macOS & Linux

- **Runtime paths** — Uses `rpath` to embed library search paths in the executable
- **Library discovery** — Dependencies found automatically at runtime
- **No additional setup** — No need to set `LD_LIBRARY_PATH` or `DYLD_LIBRARY_PATH`

Example of what's automatic:

```cmake
if(PLATFORM_MACOS)
    set_target_properties(${PROJECT_NAME} PROPERTIES
        BUILD_RPATH "@loader_path/../lib;${MayaFlux_LIB_DIR}"
        INSTALL_RPATH "@loader_path/../lib;${MayaFlux_LIB_DIR}"
        BUILD_WITH_INSTALL_RPATH TRUE
        MACOSX_RPATH ON
    )
elseif(PLATFORM_LINUX)
    set_target_properties(${PROJECT_NAME} PROPERTIES
        INSTALL_RPATH "$ORIGIN/../lib:$ORIGIN:${MayaFlux_LIB_DIR}"
        BUILD_WITH_INSTALL_RPATH TRUE
    )
endif()
```

### Windows

- **DLL copying** — Post-build step copies MayaFlux DLLs to output directory
- **Flat structure** — Everything (exe + DLLs) in same folder
- **Portable distribution** — Zip the output folder and run on other machines (no installation needed)

Example of what's automatic:

```cmake
if(WIN32)
    add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
            "$ENV{MAYAFLUX_ROOT}/bin/MayaFluxLib.dll"
            $<TARGET_FILE_DIR:${PROJECT_NAME}>
    )
endif()
```

### MayaFlux Discovery (All Platforms)

CMakeLists.txt searches for MayaFlux in this order:

1. `MAYAFLUX_ROOT` environment variable (highest priority, set by Weave)
2. Standard installation paths:
   - macOS: `/Library/MayaFlux/`, `~/MayaFlux/`
   - Linux: `/usr/local/lib/cmake/MayaFlux/`, `~/.local/lib/cmake/MayaFlux/`
   - Windows: `C:\MayaFlux\lib\cmake\MayaFlux\`
3. If not found → CMake error with helpful message

---

## Troubleshooting CMake Configuration

### "MayaFlux not found" Error

**Cause:** CMake can't locate MayaFlux installation

**Check environment:**

```bash
# macOS/Linux
echo $MAYAFLUX_ROOT

# Windows
echo %MAYAFLUX_ROOT%
```

**If empty, reload environment:**

```bash
# macOS/Linux
source ~/.zshenv    # or ~/.bashrc
cd project/build && cmake ..

# Windows
# Close and reopen PowerShell/CMD completely
```

**If still missing, set manually:**

```bash
# macOS/Linux
export MAYAFLUX_ROOT=/path/to/MayaFlux
cd build && cmake .. -DCMAKE_BUILD_TYPE=Release

# Windows (PowerShell)
$env:MAYAFLUX_ROOT = "C:\path\to\MayaFlux"
cd build; cmake .. -DCMAKE_BUILD_TYPE=Release
```

**Or pass to CMake directly:**

```bash
cmake .. -DCMAKE_PREFIX_PATH=/path/to/MayaFlux/lib/cmake
```

### Compiler Version Mismatch

**Error:** "unsupported by language binding"

**Cause:** Compiler doesn't support C++23

**Fix:** Update compiler:

```bash
# macOS
brew install llvm    # Use clang from LLVM

# Linux (Arch)
sudo pacman -S gcc   # Ensure latest

# Linux (Fedora)
sudo dnf install gcc-c++

# Linux (Ubuntu)
sudo apt install build-essential

# Windows
# Install Visual Studio Build Tools 2022 with C++ workload
```

### Linking Errors with Dependencies

**Error:** "undefined reference to `Eigen3::Eigen`"

**Cause:** Dependency not properly found

**Solution:** Make sure `find_package()` is before `target_link_libraries()`:

```cmake
# CORRECT ORDER:
find_package(Eigen3 REQUIRED)     # Find it first
add_executable(${PROJECT_NAME} ...)
target_link_libraries(${PROJECT_NAME} PRIVATE Eigen3::Eigen)  # Then link

# INCORRECT ORDER (will fail):
add_executable(${PROJECT_NAME} ...)
target_link_libraries(${PROJECT_NAME} PRIVATE Eigen3::Eigen)
find_package(Eigen3 REQUIRED)
```

---

## Contributing to Weave

Contributions welcome! See [DEVELOP.md](DEVELOP.md) for:

- Building Weave from source
- Testing on your platform
- CI/CD integration details
- Pull request guidelines
- Code style for Swift/C#/Python/Bash

---

## Key Learnings from Weave Development

1. **All dependencies are required** — There is no "optional" in MayaFlux. Every package listed is necessary for the framework to function.

2. **Explicit user consent** — The installer shows every step and lets users see what's happening. No background magic.

3. **Environment variables matter** — Weave carefully sets `MAYAFLUX_ROOT`, `CMAKE_PREFIX_PATH`, and `PATH` so projects automatically find the framework.

4. **Platform-specific DLL handling** — Windows requires explicit DLL copying; Unix uses rpath. The generated CMakeLists.txt handles both automatically.

5. **CMake integration is critical** — The `MayaFluxConfig.cmake` file provides automatic dependency resolution. Users shouldn't have to manually hunt for libraries.

6. **Testing on clean systems** — Always verify installers on fresh machines without development tools pre-installed.

---

## Links

- **[macOS Installation Guide](MACOS.md)** — Complete setup walkthrough
- **[Windows Installation Guide](WINDOWS.md)** — Step-by-step instructions
- **[Linux Installation Guide](LINUX.md)** — Distribution support and setup
- **[Development Guide](DEVELOP.md)** — Contributing to Weave
- **[Main README](README.md)** — Quick start and overview
- **[MayaFlux Framework](https://github.com/MayaFlux/MayaFlux)** — Learn the API and framework design
