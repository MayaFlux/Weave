**Platform:** Windows 10/11 (64-bit)
**Architecture:** x86_64

### Installation

**Double-click the Weave.exe file**


The installer will launch the setup wizard. After installation, **Weave Project Creator will launch automatically**.

### What Gets Installed

1. **Weave Installer & Tools** (self-contained executable)
   - Project templates (embedded in exe, extracted at runtime)
   - Dependency installation scripts (embedded in exe)
   - GUI project creator

2. **MayaFlux Framework** (downloaded from GitHub releases during installation)
   - Core libraries and headers
   - CMake configuration files

3. **Dependencies** (installed via automated scripts)
   - Build Tools: CMake, Git, Ninja, 7-Zip
   - Graphics: Vulkan SDK, GLFW  
   - Audio: FFmpeg, RtAudio
   - Math & Libraries: LLVM, Eigen3, GLM, STB, MagicEnum
   - Visual Studio Build Tools (C++ compiler)

### Installation Steps

1. Run `Weave-{{VERSION}}.exe`
2. Follow the installation wizard
3. Choose installation directory (default: `C:\MayaFlux`)
4. Dependencies will be downloaded and installed
5. Project Creator launches automatically upon completion

### Creating Your First Project

**Using the GUI (recommended):**
1. Weave Project Creator launches automatically after installation
2. Enter project name and select location
3. Click "Create Project"

**Using the CLI:**
```powershell
weave new MyProject C:\Projects\
cd C:\Projects\MyProject
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release --parallel
.\Release\MyProject.exe
```

### Environment Setup

After installation, your system will have:
- `MAYAFLUX_ROOT` environment variable pointing to installation directory
- `PATH` updated with MayaFlux binaries and tools
- `CMAKE_PREFIX_PATH` configured for CMake discovery

Restart your terminal/PowerShell to use the new environment variables.

### System Requirements

- **OS:** Windows 10/11 (64-bit)
- **RAM:** 4GB minimum (8GB+ recommended for dependency compilation)
- **Disk:** ~5GB free space (for MayaFlux + dependencies)
- **Internet:** Required for initial downloads
- **Permissions:** Administrator privileges required for installation