# Weave

**Installer and project tool for [MayaFlux](https://github.com/MayaFlux/MayaFlux).**

Weave installs MayaFlux, creates ready-to-build CMake projects, and helps you create and add community modules. Use the desktop app or the standalone CLI.

## Download

Get the package for your platform from [Releases](https://github.com/MayaFlux/Weave/releases).

| Platform | Package | Requirements |
| --- | --- | --- |
| Windows | EXE | x64, Windows 10 22H2 (build 19045) or later |
| macOS | DMG | Apple Silicon, macOS 26 or later |
| Linux | AppImage | x86_64, glibc 2.43 or later, FUSE |

- **Windows:** download and open **Weave-windows.exe**. Launch normally; Weave requests administrator consent only when installing host software.
- **macOS:** open the DMG and drag **Weave.app** to Applications. If Gatekeeper blocks the downloaded app, approve it through **System Settings > Privacy & Security > Open Anyway**.
- **Linux:** make the AppImage executable and launch it:

  ```sh
  chmod +x Weave-linux.AppImage
  ./Weave-linux.AppImage
  ```

Each download includes the executable, templates, and resources needed to run Weave. Your operating system supplies the display and graphics driver. Building generated projects requires MayaFlux, CMake, Ninja, and a C++ compiler.

## Install MayaFlux

Open Weave and choose **Install MayaFlux**, then select **Stable** or **Development**. Weave checks your system, installs MayaFlux and its build dependencies, and configures the environment where needed.

Installation uses winget on Windows, Homebrew on macOS, and your distribution's package manager on supported Linux distributions. Follow any authorization prompts and restart your terminal or Windows if requested. Weave asks before replacing another MayaFlux edition. If installation fails, review the reported error and retry; project operations remain available.

## Create a project

Choose **Projects > Create Project**, enter a name, and select an existing destination folder. You can enable live coding with Lila, include VS Code configuration, and copy installed MayaFlux shader sources.

Your project is an ordinary CMake directory. Write your code in `src/user_project.hpp`:

- `settings()` configures the engine before it starts.
- `compose()` builds your audio and visual processing graph.

Configure and build from the project folder:

```sh
cmake --preset release
cmake --build --preset release
```

Run `build/YourProject` on Linux and macOS, or `build\YourProject.exe` on Windows. A `debug` preset is also included. You can open the folder in your preferred CMake-capable editor or IDE and edit `CMakeLists.txt` as your project grows.

For live coding, enable **Live coding (Lila)** when creating the project, then connect [LilaCode](https://github.com/MayaFlux/LilaCode) for VS Code or [lila.nvim](https://github.com/MayaFlux/lila.nvim) for Neovim.

## Community modules

### Add modules to a project

Choose **Projects > Manage Project**, select your project folder, and enter one or more module names under **Add Community Modules**. Weave looks them up in the [community registry](https://github.com/MayaFlux/community-sources-registry), checks their MayaFlux requirements, downloads them into `community/`, and registers them in `community.cmake`. Rebuild your project afterward.

Modules already present in the project are skipped. A module's `community.json` may also declare packages, manual installations, environment settings, and data assets. Weave validates these declarations but does not install the listed dependencies for you.

### Create a module

Choose **Projects > Create Community Module**. Enter a snake_case name, description, minimum MayaFlux version, and whether the module needs Lila. Weave creates the source folder, CMake configuration, `community.json`, and a test project.

Build and test from the generated module folder:

```sh
cmake --preset release
cmake --build --preset release
ctest --test-dir test/build
```

See the [community manifest contract](docs/COMMUNITY_MANIFEST.md) for the supported metadata and dependency declarations. To share a module, publish its repository and submit it to the community registry.

## Command line

The same executable provides a CLI without opening the desktop app. Use its full path if it is not on PATH. For help, run `.\Weave-windows.exe --help` on Windows, `/Applications/Weave.app/Contents/MacOS/weave --help` on macOS, or `./Weave-linux.AppImage --help` on Linux. Use `--license` to read the embedded license.

Examples below use `weave` as shorthand for that executable:

```sh
weave new AudioDemo /path/to/projects --with-lila
weave community audio_tools /path/to/modules --description "Audio helpers"
weave manage /path/to/AudioDemo terrain_field,audio_tools
weave install stable
weave install development --replace-conflict
```

## Diagnostics and bug reports

Generated projects include scripts for checking the project and collecting a report:

| Task | Linux and macOS | Windows |
| --- | --- | --- |
| Check project and installation | `./validate.sh` | `.\validate.bat` |
| Run a built project and collect logs | `./run-report.sh` | `.\run-report.bat` |

Run these from the project folder. Build the project before using `run-report`, reproduce the problem, then close the application. Attach `mayaflux_run.txt` and `data/log/mayaflux_report.log` when reporting a runtime problem. The Windows `.bat` launchers call the accompanying PowerShell scripts.

Report Weave problems through [Issues](https://github.com/MayaFlux/Weave/issues). For project runtime problems, use the [MayaFlux issue templates](https://github.com/MayaFlux/MayaFlux/issues/new/choose).

## Development

To build or contribute to Weave itself, see the [development guide](docs/DEVELOP.md).

## License

GNU General Public License v3.0. See [LICENSE.txt](LICENSE.txt).
