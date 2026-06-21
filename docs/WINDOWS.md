# Weave for Windows

**Platform:** Windows 10/11 (64-bit)

---

## Installing MayaFlux

1. Download `Weave-X.X.X.exe` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. Double-click the `.exe`. A UAC prompt appears —> click **Yes**
3. Choose **Install MayaFlux**
4. Follow the step-by-step installer:

| Step | What happens |
| ---- | ------------ |
| System check | Verifies 64-bit Windows, installs 7-Zip and MSVC build tools if missing |
| Download MayaFlux | Downloads the framework archive from GitHub and extracts it to `C:\MayaFlux` |
| Install dependencies | Installs CMake, Git, Ninja, LLVM, FFmpeg, and Vulkan SDK via winget |
| Environment setup | Sets system environment variables |
| Templates | Extracts project templates |

5. Restart your terminal when complete. Environment variables are set at the system level and take effect in any new terminal window.

### What gets installed

**`C:\MayaFlux\`** contains the full MayaFlux framework: headers, import libraries, CMake config, runtime DLLs, shaders, and all bundled dependencies (GLFW, RtAudio, Eigen3, STB, and others). These ship inside the framework archive rather than being installed separately.

The following are installed via winget and land in their standard locations:

- CMake
- Git
- Ninja
- LLVM
- FFmpeg (`Gyan.FFmpeg.Shared`)
- Vulkan SDK (`C:\VulkanSDK\`)
- MSVC Build Tools : VS 2026 on Windows 11, VS 2022 on Windows 10 (skipped if already present)

### Environment

Weave sets these as system environment variables:

| Variable            | Value             | Purpose                  |
| ------------------- | ----------------- | ------------------------ |
| `MAYAFLUX_ROOT`     | `C:\MayaFlux`     | Framework root           |
| `PATH`              | `+= C:\MayaFlux\bin` | Runtime DLLs and tools |
| `INCLUDE`           | `+= C:\MayaFlux\include` | Headers for editors  |
| `LIB`               | `+= C:\MayaFlux\lib` | Import libraries       |

These take effect in any new PowerShell or CMD window. The terminal you ran Weave from will not reflect them until restarted.

---

## Creating a Project

Open `Weave.exe` and choose **Create Project**. Enter a name, pick a destination, optionally enable **Live Coding (Lila)** and **VS Code configuration**, then click **Create Project**.

There is no CLI tool on Windows. All project operations go through the GUI.

The generated project includes a `CMakePresets.json` with `debug` and `release` presets. Visual Studio, VS Code with the CMake extension, and CLion all pick these up automatically when you open the project folder.

### Building

```powershell
cd MyProject

# with presets
cmake --preset release
cmake --build --preset release
.\build\MyProject.exe

# without presets
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release --parallel
.\build\Release\MyProject.exe
```

On Windows, the CMake build copies all required MayaFlux DLLs next to your executable automatically as a post-build step. If you move the executable, copy the DLLs alongside it.

---

## Community Modules

### Adding a module

Open Weave and choose **Add Community Module**, select your project directory, and enter the module name. Weave fetches the registry, checks version compatibility, clones the module into `community/<name>/`, and registers it in `community.cmake`. Rebuild your project after adding modules.

### Creating a module

Choose **Create Community Module** in Weave. Enter a name (snake_case), description, minimum MayaFlux version, and whether the module requires Lila. Weave scaffolds the module with `src/`, `test/`, a `CMakePresets.json`, and a git repository ready to push.

See the [Weave README](../README.md) for details on module structure and registry submission.

---

## Troubleshooting

### UAC prompt does not appear or installation fails immediately

Weave requires administrator privileges. Right-click `Weave.exe` and choose **Run as administrator**.

### CMake or other tools not found after installation

Open a new PowerShell or CMD window. Environment variables set by Weave are not visible in terminals that were already open when installation ran.

### CMake can't find MayaFlux

Verify `MAYAFLUX_ROOT` is set:

```powershell
echo $env:MAYAFLUX_ROOT
```

If empty, open a new terminal. If still empty, check that the environment step completed without errors during installation. You can point CMake at it directly in the meantime:

```powershell
cmake -B build -DCMAKE_BUILD_TYPE=Release -DMAYAFLUX_ROOT="C:\MayaFlux"
```

### Build fails with missing DLL at runtime

The CMake post-build step copies DLLs automatically. If a DLL is reported missing at runtime, re-run the build:

```powershell
cmake --build build --config Release
```

If that does not resolve it, copy manually:

```powershell
copy C:\MayaFlux\bin\*.dll build\Release\
```

### MSVC not found after installation

If the system check installed MSVC Build Tools and reported exit code `3010`, a system restart is required before the compiler is usable. Restart and run Weave again to verify.

---

## Uninstalling

```powershell
# Remove MayaFlux
Remove-Item -Recurse -Force C:\MayaFlux

# Remove environment variables (optional)
# Settings > System > Advanced system settings > Environment Variables
# Delete MAYAFLUX_ROOT and remove C:\MayaFlux\bin from PATH
```

Winget-installed tools (CMake, Git, Ninja, LLVM, FFmpeg, Vulkan SDK, MSVC) are independent installations. Remove them individually via **Settings > Apps** if no longer needed.

---

## Links

- [Weave README](../README.md)
- [MayaFlux](https://github.com/MayaFlux/MayaFlux)
- [LilaCode (VS Code)](https://github.com/MayaFlux/LilaCode)
- [lila.nvim (Neovim)](https://github.com/MayaFlux/lila.nvim)
