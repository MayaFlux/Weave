# Weave for Windows

Complete guide for installing, using, and troubleshooting Weave on Windows.

## Installation

### Prerequisites

- Windows 10 or later (64-bit only)
- ~2GB free disk space
- Internet connection
- Administrator privileges (for dependency installation)

### Install from Executable

1. **Download** `Weave-X.X.X.exe` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. **Double-click** the `.exe` file
3. **UAC (User Access Control) dialog** appears - click "Yes"
4. **Mode selection dialog** appears:
   - Choose **"Install MayaFlux"** for fresh installation
   - Choose **"Create Project"** if already installed
5. **Follow the step-by-step installer:**
   - **Step 1: System Check** - Verifies 64-bit Windows and admin privileges
   - **Step 2: Download MayaFlux** - Downloads framework from GitHub (~100+ MB)
   - **Step 3: Install Dependencies** - Installs build tools and libraries (10-20 min)
   - **Step 4: Environment Setup** - Configures system environment variables
   - **Step 5: Templates** - Extracts project templates
   - **Step 6: Complete** - Shows success summary
6. **Restart your terminal/PowerShell** for environment changes to take effect
7. **Create your first project** - Run Weave again and select "Create Project"

### What Gets Installed

- **`C:\MayaFlux\`** - MayaFlux framework, headers, CMake configs
- **`C:\Program Files\`** - Build tools (CMake, Git, Ninja, 7-Zip)
- **`C:\VulkanSDK\`** - Vulkan SDK for graphics
- **`C:\Program Files\FFmpeg\`** - Audio/video library
- **Environment Variables:**
  - `MAYAFLUX_ROOT` = `C:\MayaFlux`
  - `PATH` += `C:\MayaFlux\bin`
  - `CMAKE_PREFIX_PATH` = `C:\MayaFlux`

### Post-Installation

**Restart your terminal:**

Close PowerShell/CMD completely and reopen. Environment variables reload automatically.

**Verify installation:**

```powershell
echo $env:MAYAFLUX_ROOT
# Should output: C:\MayaFlux

cmake --version
# Should show CMake version
```

---

## Creating Projects

### Using Weave.exe (GUI)

1. Open `Weave.exe` (from Start Menu or wherever you saved it)
2. Select **"Create Project"** mode
3. Enter **Project Name** (e.g., "MyFirstProject")
4. Click **"Browse..."** to select location
5. Optional: Enable "Enable Live Coding (Lila)"
6. Click **"Create Project"**
7. Success dialog shows your project location

**Note:** GUI is the primary project creation method on Windows. CLI tool may be added in future versions.

---

## Building & Running

### Quick Start (Command Line)

```powershell
cd MyProject
mkdir build
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release --parallel
.\Release\MyProject.exe
```

### With Visual Studio

1. Open generated project folder in Visual Studio
2. CMake integration should auto-detect configuration
3. Build → Build Solution
4. Debug → Start Debugging (F5)

### Command Prompt Alternative

```cmd
cd MyProject\build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
Release\MyProject.exe
```

---

## Environment Variables

After installation, these are set in your system environment:

| Variable            | Value                      | Purpose            |
| ------------------- | -------------------------- | ------------------ |
| `MAYAFLUX_ROOT`     | `C:\MayaFlux`              | Framework location |
| `CMAKE_PREFIX_PATH` | Includes `%MAYAFLUX_ROOT%` | CMake discovery    |
| `PATH`              | Includes MayaFlux bin      | CLI tools          |

**Changes take effect after restarting terminal/PowerShell.**

### Manual Environment Variable Check

```powershell
# PowerShell
echo $env:MAYAFLUX_ROOT

# Command Prompt
echo %MAYAFLUX_ROOT%
```

### Setting Custom Paths

If you installed MayaFlux to a different location, set manually:

```powershell
# PowerShell
$env:MAYAFLUX_ROOT = "C:\Custom\Path\To\MayaFlux"
```

```cmd
# Command Prompt
set MAYAFLUX_ROOT=C:\Custom\Path\To\MayaFlux
```

---

## Troubleshooting

### Installation Fails with "Administrator privileges required"

**Solution:** Right-click `Weave.exe` → "Run as administrator"

### Dependency Installation Skips with Warnings

**Cause:** Some packages couldn't be installed

**Solution:**

- Check internet connection (dependencies are downloaded)
- Check installation log: `%LOCALAPPDATA%\weave_install.log`
- You can continue - dependencies can be manually installed later if needed

### "CMake not found" after installation

**Cause:** Terminal wasn't restarted after installation

**Solution:** Close PowerShell/CMD completely and reopen (not just a new tab)

### CMake can't find MayaFlux

**Verify environment is set:**

```powershell
echo $env:MAYAFLUX_ROOT
# Should show: C:\MayaFlux
```

**If empty, restart your terminal.**

**If still not found, set manually in CMake:**

```powershell
cd build
cmake .. -DCMAKE_PREFIX_PATH=C:\MayaFlux -DCMAKE_BUILD_TYPE=Release
```

### Build fails with "missing DLL"

**Cause:** MayaFlux DLLs not copied to output directory

**Solution:** Run build again (CMake post-build steps copy DLLs):

```powershell
cd build
cmake --build . --config Release
```

**If issue persists:**

```powershell
# Manually copy DLLs
copy C:\MayaFlux\bin\*.dll Release\
```

### PowerShell execution policy error

**Cause:** PowerShell blocked script execution

**This should be handled automatically by Weave.** If you see execution policy errors:

```powershell
# Temporarily allow for this session
Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process

# Then rebuild
cmake --build . --config Release
```

### Build errors with C++23 features

**Ensure you have Visual Studio with C++ workload:**

- Visual Studio 2022 (latest update)
- Or Visual Studio Build Tools 2022 with C++ workload

**Check compiler:**

```powershell
cl.exe
# Should show MSVC 193.x or higher
```

### "Weave.exe is not recognized"

**Cause:** PATH not updated or terminal wasn't restarted

**Solution:** Restart your terminal, or run Weave from its full path:

```powershell
C:\Users\YourName\Downloads\Weave.exe
```

---

## Uninstalling

### Remove Everything

```powershell
# Remove MayaFlux directory
Remove-Item -Path "C:\MayaFlux" -Recurse -Force

# Remove from Start Menu (automatic via Windows)
# Open Settings > Apps > Apps & Features, find "Weave" and uninstall
```

Or use Windows Settings:

1. Settings → Apps → Apps & Features
2. Find "Weave"
3. Click → "Uninstall"

### Clean Up Environment Variables (Optional)

1. Settings → System → Advanced system settings
2. Environment Variables
3. Under "System variables", find and delete:
   - `MAYAFLUX_ROOT`
   - Any additions to `PATH` containing MayaFlux
   - `CMAKE_PREFIX_PATH` (if only contains MayaFlux)

### Registry Cleanup (if needed)

Weave doesn't use registry. Safe to delete all folders above.

---

## FAQ

**Q: Can I install Weave to a different location?**

A: The installer uses `C:\MayaFlux` by default. To use a different location, set `MAYAFLUX_ROOT` environment variable after installation (see Environment Variables section).

**Q: Can I have multiple MayaFlux installations?**

A: Yes, but only one `MAYAFLUX_ROOT` at a time. Switch between them by changing the environment variable.

**Q: Why does installation take so long?**

A: The installer downloads the MayaFlux framework (~100+ MB) and dependencies. This depends on your internet speed. 10-30 minutes is normal.

**Q: Can I skip dependency installation?**

A: The installer runs dependency installation for you. If it fails for specific packages, you can manually install them later or continue without them.

**Q: Does Weave auto-update?**

A: Not yet. Download the new installer from Releases and run it again. It will update existing installations.

**Q: Is there a CLI tool on Windows?**

A: Not in the current version. Use the GUI (Weave.exe) to create projects. CLI may be added in future versions.

**Q: Can I build with MinGW instead of Visual Studio?**

A: Not officially supported. Weave is designed for MSVC. You may be able to use MinGW with CMake, but dependency compilation would be different.

**Q: How do I use PowerShell vs Command Prompt?**

A: Both work. PowerShell syntax shown uses `$env:` for variables; Command Prompt uses `%variable%`. Choose whichever you prefer.

---

## Links

- **[MayaFlux Framework](https://github.com/MayaFlux/MayaFlux)** - Learn the API
- **[Back to README](../README.md)** - Overview and quick start
- **[FAQ](FAQ.md)** - Cross-platform questions
