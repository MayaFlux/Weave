# Weave

**Installer and project tool for MayaFlux.**

Weave installs MayaFlux, creates projects, and manages community modules. Download it from [Releases](https://github.com/MayaFlux/Weave/releases).

---

## Installing MayaFlux

Launch Weave and choose **Install MayaFlux**. Weave downloads the framework, installs all dependencies, and configures your environment. Restart your terminal when it finishes.

If something goes wrong, see [Diagnostics and bug reports](#diagnostics-and-bug-reports).

---

## Creating a Project

Launch Weave and choose **Create Project**. Enter a name and pick a destination. Weave generates a ready-to-build project with the following structure:

```
MyProject/
├── CMakeLists.txt
├── CMakePresets.json
├── community.cmake
├── cmake/
│   ├── mayaflux.cmake
│   ├── shaders.cmake
│   └── build_community.cmake
├── src/
│   ├── main.cpp
│   └── user_project.hpp
├── data/
│   └── shaders/
├── validate.sh          # validate.bat and validate.ps1 on Windows
├── run-report.sh        # run-report.bat and run-report.ps1 on Windows
├── .vscode/             # if VS Code configuration was enabled
│   ├── settings.json
│   ├── tasks.json
│   └── launch.json
├── .gitignore
└── README.md
```

`src/user_project.hpp` is where you write your code. It defines two functions:

- `settings()` runs before the engine starts. Configure sample rate, buffer size, graphics API, logging, and so on.
- `compose()` is where you build your audio/visual processing graph.

On Windows and Linux, **Copy MayaFlux shader sources** optionally copies the installed `share/MayaFlux/src/shaders` tree into `data/shaders`, preserving nested paths and GLSL include files. It copies source files only; installed runtime SPIR-V remains in the MayaFlux installation.

### Engine configuration

A project loads `mayaflux.json` from its folder at startup when the file exists, or from the path given with `--config path/to/file.json`. The file sets audio, graphics, input, network and journal options; MayaFlux installs the Settings guide at `share/MayaFlux/guides/Settings.md`. Values set in `settings()` take precedence over the file, and `--config-override` reverses that. Relative file names in the journal and the project's file readers resolve against the project folder. Projects created with earlier versions of Weave keep their old `main.cpp`, so pass `--config` explicitly or copy the new template's `main.cpp`.

### Diagnostics and bug reports

Each new project includes two scripts. Run them from the project folder:

| | Linux and macOS | Windows |
|---|---|---|
| Check the project | `./validate.sh` | `.\validate.bat` |
| Collect a report | `./run-report.sh` | `.\run-report.bat` |

**`validate`** checks project files, presets, community module references, `mayaflux.json`, local shaders and the MayaFlux installation. It also lists the OS, the GPU and Vulkan driver, and whether the Vulkan validation layer is installed. On Windows it also checks tools and terminal settings. It does not configure, build, launch or modify anything, and Weave never runs it automatically. To check another folder, pass it as the first argument on Linux and macOS, or add `-ProjectRoot "C:\Path\To\Project"` on Windows.

On Windows, `validate.ps1` is also installed with Weave, so you can check the installation before you have a project:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:MAYAFLUX_ROOT\share\weave\templates\validate.ps1"
```

It makes no changes and does not contact the network. If `MAYAFLUX_ROOT` is missing, use the script's full installed path and add `-MayaFluxRoot "C:\Your MayaFlux Folder"`.

**`run-report`** runs the built project with the report configuration that MayaFlux installs at `share/MayaFlux/report/mayaflux.report.json`, which turns on the Vulkan validation layers and writes a complete log. Build the project first, reproduce the problem, then close the application. The script saves the console output to `mayaflux_run.txt` and the application writes `data/log/mayaflux_report.log`. Earlier logs are kept with a `.previous-` suffix, and the script returns the application's exit code. If the application stops before creating its report, attach the console log and say so.

| Need | Linux and macOS | Windows |
|---|---|---|
| Several build configurations | `--configuration Debug` | `-Configuration Debug` |
| Custom output location | `--executable path/to/binary` | `-Executable "path\to\MyProject.exe"` |
| Custom MayaFlux installation | `--mayaflux-root PATH` | `-MayaFluxRoot "C:\Your MayaFlux Folder"` |

Without an explicit installation, the scripts use `MAYAFLUX_ROOT`, then `/usr` and `/usr/local` on Linux or the installed Homebrew formula on macOS. On Windows they read `MAYAFLUX_ROOT` from the process, user or machine environment. The `.bat` launchers start the PowerShell scripts, keep the terminal's environment and pass the exit codes through. The `.ps1` scripts also run directly when your execution policy allows it. Existing projects keep their files; the scripts are generated when a project is created.

To report a problem, run both scripts and open an issue with the [Weave project template](https://github.com/MayaFlux/MayaFlux/issues/new/choose).

`CMakeLists.txt` is yours to edit. Add source files, link libraries, and set compiler flags as needed. The MayaFlux integration lives in `cmake/mayaflux.cmake` and is included automatically at the end of `CMakeLists.txt`.

### Building

`CMakePresets.json` ships with every project, giving IDEs and editors a zero-config starting point. It defines `debug` and `release` presets, both using Ninja into `build/`.

```bash
# using presets
cmake --preset release
cmake --build --preset release
./build/MyProject

# or without presets
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel
./build/MyProject
```

On Windows, the binary lands in `build\MyProject.exe`. CLion, VS Code with the CMake extension, and Visual Studio all pick up the presets automatically when you open the project folder.

### Live Coding (Lila)

To enable live coding, check **Enable Live Coding (Lila)** in the project creation dialog. This links your project against `MayaFlux::MayaFluxHost`, embedding the Lila JIT compiler in your process. You can then connect [LilaCode](https://github.com/MayaFlux/LilaCode) (VS Code) or [lila.nvim](https://github.com/MayaFlux/lila.nvim) (Neovim) to evaluate C++ code against your running application in real time.

---

## Community Modules

Community modules are C++ source libraries that compile directly into your project. There is no plugin boundary or ABI to worry about; modules are just code.

### Adding Modules

In Weave, open your project and choose **Add Community Module**. Enter the module name and Weave will:

1. Fetch the registry from [community-sources-registry](https://github.com/MayaFlux/community-sources-registry)
2. Check that your MayaFlux version meets the module's minimum requirement
3. Clone the module into `community/<name>/`
4. Register it in `community.cmake`

Rebuild your project after adding modules.

Modules that already exist in `community/` are skipped. You can add multiple modules at once.

A module can declare the system packages, manual installs and data assets it needs in its `community.json`. Weave reads and validates these declarations and tells you which modules declare them, but it does not install anything they list in this release. After adding a module, read its `community.json` and install what it lists with your package manager.

### How Modules Integrate

`community.cmake` lists module names, one per line. At configure time, `cmake/build_community.cmake` reads this file and calls `add_community()` for each entry, which includes the module's own `<name>.cmake`, builds it as an OBJECT library, and links it into your project. No manual CMake edits needed.

### Creating a Module

Choose **Create Community Module** in Weave. Enter a name (snake_case), a description, a minimum MayaFlux version, and whether the module requires Lila. Weave scaffolds:

```
my_module/
├── src/                        <- your C++ sources go here
├── test/
│   ├── CMakeLists.txt
│   └── test_my_module.cpp
├── my_module.cmake
├── CMakePresets.json
├── community.json
└── .gitignore
```

`my_module.cmake` defines the module as an OBJECT library linking `MayaFlux::MayaFluxLib`. If Lila is required, it also links `MayaFlux::MayaFluxHost`. Users call your constructors directly from their `compose()`.

`community.json` carries registry metadata: name, description, minimum MayaFlux version, and whether Lila is required.

The [community manifest contract](docs/COMMUNITY_MANIFEST.md) defines this metadata and typed declarations for community-module packages, manual installations, environment configuration, and verified assets.

The `test/` directory contains a standalone CMake project for building and testing your module independently. `CMakePresets.json` at the module root points both presets at `test/build`, so any IDE opened on the module folder just works:

```bash
cmake --preset release
cmake --build --preset release
```

The module is initialized as a git repository. Push it to GitHub and submit a PR to [community-sources-registry](https://github.com/MayaFlux/community-sources-registry) to list it publicly.

---

## Links

- [MayaFlux](https://github.com/MayaFlux/MayaFlux)
- [LilaCode (VS Code)](https://github.com/MayaFlux/LilaCode)
- [lila.nvim (Neovim)](https://github.com/MayaFlux/lila.nvim)
- [Community Registry](https://github.com/MayaFlux/community-sources-registry)

---

## License

GNU General Public License v3.0. See [LICENSE](LICENSE) for full terms.
