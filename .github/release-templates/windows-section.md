## windows-section.md

**Platform:** Windows 10/11 (64-bit)  
**Architecture:** x86_64

### Installation

**Double-click `Weave-{{VERSION}}.exe`** to launch the installer.

The installer will:
1. Show a mode selection dialog (Install MayaFlux or Create Project)
2. Run step-by-step installation with progress tracking
3. Download MayaFlux framework (~100 MB)
4. Install dependencies (CMake, Vulkan SDK, FFmpeg, LLVM, etc.)
5. Configure environment variables
6. Extract project templates

**Total time:** 15-30 minutes (depends on internet and system)

### Installation Steps

1. **Mode Selection** - Choose "Install MayaFlux"
2. **System Check** - Verifies Windows 64-bit, 7-Zip availability, admin privileges
3. **Download MayaFlux** - Fetches latest framework from GitHub, shows progress bar
4. **Install Dependencies** - Runs PowerShell script to install build tools and libraries
5. **Environment Setup** - Sets system environment variables
6. **Templates** - Extracts project templates
7. **Complete** - Shows summary with log file location

### What Gets Installed

- **Weave.exe** - Single GUI application for all operations
- **MayaFlux Framework** - Downloaded to `C:\MayaFlux\` (default)
- **Build Tools** - CMake, Git, Ninja, 7-Zip
- **Graphics Stack** - Vulkan SDK, GLFW, GLM
- **Audio Stack** - FFmpeg, RtAudio
- **Development Libraries** - LLVM, Eigen3, STB, MagicEnum, oneDPL

### Environment Setup

After installation, your system has:
- `MAYAFLUX_ROOT` - Points to installation directory
- `PATH` - Updated with MayaFlux and build tools
- `CMAKE_PREFIX_PATH` - For CMake discovery

**Restart your terminal/PowerShell** for changes to take effect.

### Creating Your First Project

1. Run `Weave.exe` again (or click in Start Menu)
2. Select "Create Project" mode
3. Enter project name and select location
4. (Optional) Enable Lila or VS Code configuration
5. Click "Create Project"

The GUI shows real-time output of project generation.

---


