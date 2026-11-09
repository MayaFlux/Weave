# Weave

**Installer and project tool for [MayaFlux](https://github.com/MayaFlux/MayaFlux).**

Weave installs MayaFlux, creates CMake projects, and helps you maintain projects and community modules. Use the desktop app or the standalone CLI.

## Download

Get the package for your platform from [Releases](https://github.com/MayaFlux/Weave/releases).

| Platform | Package | Requirements |
| --- | --- | --- |
| Windows | EXE | x64, Windows 10 22H2 (build 19045) or later |
| macOS | DMG | Apple Silicon, macOS 26 or later |
| Linux | AppImage | x86_64, glibc 2.43 or later, FUSE |

- **Windows:** download and open the Windows x64 EXE. Launch normally; Weave requests administrator consent only when installing host software.
- **macOS:** open the DMG and drag **Weave.app** to Applications. If Gatekeeper blocks the downloaded app, approve it through **System Settings > Privacy & Security > Open Anyway**.
- **Linux:** install your distribution's FUSE support if it is missing. In your file manager, open the AppImage's **Properties > Permissions**, enable **Allow executing file as program** (the wording varies by desktop), then open it. If it is already executable, just open it. From a terminal, you can instead run:

  ```sh
  chmod +x Weave-linux.AppImage
  ./Weave-linux.AppImage
  ```

Each download includes the executable, templates, and resources needed to run Weave. Your operating system supplies the display and graphics driver. Building generated projects requires MayaFlux, CMake, Ninja, and a C++ compiler.

## Install MayaFlux

Open Weave and choose **Install MayaFlux**, then select **Stable** or **Development**. Weave checks your system, installs MayaFlux and its build dependencies, and configures the environment where needed.

**Make the Weave CLI available in new terminals** is selected by default. Leave it selected to use the `Weave` command after installation, then open a new terminal. Uncheck it to leave your existing CLI setup untouched. You can also run `Weave install-cli` separately.

To update MayaFlux, run installation again for your chosen stream. Development installs check build freshness and can reuse a recent, verified installation. Switching between Stable and Development requires confirmation before replacement.

Installation uses winget on Windows, Homebrew on macOS, and your distribution's package manager on supported Linux distributions. Follow any authorization prompts and restart your terminal or Windows if requested. Weave asks before replacing another MayaFlux edition. If installation fails, review the reported error and retry; project operations remain available.

On Windows, setup and project validation also check installed graphics driver versions. Driver update advice includes download links and remains visible when installation finishes or fails. It does not block installation or cause validation to fail.

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

Run `./build/YourProject` on Linux and macOS, or `.\build\YourProject.exe` on Windows. A `debug` preset is also included. You can open the folder in your preferred CMake-capable editor or IDE and edit `CMakeLists.txt` as your project grows.

For live coding, enable **Live coding (Lila)** when creating the project, then connect [LilaCode](https://github.com/MayaFlux/LilaCode) for VS Code or [lila.nvim](https://github.com/MayaFlux/lila.nvim) for Neovim.

## Manage a project

Choose **Projects > Manage Project**, then select the project folder containing `CMakeLists.txt`. All Manage actions use that selected project.

### Add project options

Select **Add Lila live coding**, **Copy MayaFlux shader sources**, or both, then choose **Apply Selected Options**. Weave skips options already applied and identical shader files. If shader files differ, review the conflicts before choosing whether to keep or replace them. Customized Lila CMake layouts that cannot be identified safely are reported for manual review. Rebuild after changing project options.

### Validate the project

Choose **Validate Project** to check the project files, MayaFlux installation, tools, and environment. Read the diagnostics and copy the selectable output when asking for help. Follow any `[FIX]` instructions, then use **Retry**. **Cancel** stops an active check.

Validation does not build or run the project. Driver update advice and other informational messages do not by themselves make validation fail.

### Run and collect a report

Build your project first, then choose **Run & Report**. Leave the optional fields empty for automatic executable and MayaFlux discovery. If multiple builds exist, select a configuration or choose an executable explicitly. Select a MayaFlux installation folder if the project uses a different installation from your current environment.

Reproduce the problem while Weave displays the application's output. Choose **Stop and collect** to request a normal shutdown and finish collecting diagnostics, or close the application normally. If the application does not respond, use **Force stop**; a forced termination may leave its report incomplete.

Weave shows the paths of the reports that were created. Save or attach `mayaflux_run.txt` and, when available, `data/log/mayaflux_report.log`. Previous reports are saved with a `.previous-…` suffix before another run.

If **Stop and collect** is unavailable for an older project, refresh its run/report script as described below. Force stop remains available.

### Refresh project files

Choose **Refresh Project Files** to compare the project with the templates in your current Weave download. Review the list before applying: all listed files start selected. **Uncheck files containing your own code or settings, including `src/user_project.hpp` and `src/main.cpp`, if you want to keep them.** To update only diagnostics, leave just the validation and run/report scripts and their Windows launchers selected.

Selected existing files are backed up under `.weave-refresh-backups/` before replacement. Community module registrations and shader sources are preserved. From the CLI, use `Weave manage <project-dir> --refresh` to preview, or `--refresh-apply` to apply; repeat `--refresh-skip <relative-path>` for each file you want to keep.

## Community modules

### Add modules to a project

Choose **Projects > Manage Project**, select your project folder, and enter one or more module names under **Add Community Modules**. Weave looks them up in the [community registry](https://github.com/MayaFlux/community-sources-registry), checks their MayaFlux requirements, downloads them into `community/`, and registers them in `community.cmake`. Rebuild your project afterward.

Existing module folders are checked and reused without replacing their contents. A module can request that you open a URL, such as a download page. Weave asks before each browser launch. Complete any required steps in the browser and place downloaded files at the destination shown by Weave. You can decline and open the reported link later; module registration continues, but those actions remain pending. Weave does not wait for downloads or verify that browser actions are complete.

A module's `community.json` may also declare packages, manual installations, environment settings, and data assets. Review those requirements and follow the module author's instructions. Weave validates these declarations but does not install those packages, apply those environment changes, or download those assets for you.

### Create a module

Choose **Projects > Create Community Module**. Enter a snake_case name, description, minimum MayaFlux version, and whether the module needs Lila. Weave creates the source folder, CMake configuration, `community.json`, and a test project.

Build and test from the generated module folder:

```sh
cmake --preset release
cmake --build --preset release
ctest --preset release
```

See the [community manifest contract](docs/community_manifest.md) for the supported metadata and dependency declarations. To share a module, publish its repository and submit it to the community registry.

## Command line

The same download provides a CLI without opening the desktop app. The command is **`Weave` with a capital W**. Run `Weave --help` to see all options and `Weave --license` to read the license.

If the command is not on PATH, use the executable's full path. For example, run `.\Weave-windows.exe --help` on Windows, `/Applications/Weave.app/Contents/MacOS/Weave --help` on macOS, or `./Weave-linux.AppImage --help` on Linux. CI artifact filenames also include the version and architecture; use the filename you downloaded.

Run `Weave install-cli` to make the command available in new terminals. Windows installs it under `%USERPROFILE%\.local\bin`; Linux uses `~/.local/bin`; macOS uses the executable inside `/Applications/Weave.app`. Restart your terminal afterward. `Weave install stable --no-cli` installs MayaFlux without setting up CLI access.

Examples below use `Weave` as shorthand for that executable:

```sh
Weave new AudioDemo /path/to/projects --with-lila
Weave community audio_tools /path/to/modules --description "Audio helpers"
Weave manage /path/to/AudioDemo terrain_field,audio_tools
Weave manage /path/to/AudioDemo --with-lila --copy-shader-sources
Weave manage /path/to/AudioDemo --copy-shader-sources --replace-shader-conflicts
Weave manage /path/to/AudioDemo --validate
Weave manage /path/to/AudioDemo --run-report --configuration Release
Weave manage /path/to/AudioDemo --run-report --executable "/path with spaces/AudioDemo"
Weave manage /path/to/AudioDemo --refresh
Weave install stable
Weave install-cli
Weave install development --replace-conflict
```

Quote paths containing spaces. When a community module requests a browser action, the CLI prompts for confirmation; answer `y` to open the link. A declined action is reported for manual completion.

## Diagnostics and bug reports

Generated projects include scripts for checking the project and collecting a report:

| Task | Linux and macOS | Windows |
| --- | --- | --- |
| Check project and installation | `./validate.sh` | `.\validate.bat` |
| Run a built project and collect logs | `./run-report.sh` | `.\run-report.bat` |

Run these from the project folder. Build the project before using `run-report`, reproduce the problem, then stop the application normally. **Stop and collect** and **Force stop** are controls in Weave's GUI. The Windows `.bat` launchers call the accompanying PowerShell scripts.

For validation problems, copy the complete diagnostic output. For runtime problems, attach `mayaflux_run.txt` and `data/log/mayaflux_report.log` if it was created. A failed or forced run may produce only the console log. These scripts can be used without keeping Weave open.

Report Weave problems through [Issues](https://github.com/MayaFlux/Weave/issues). For project runtime problems, use the [MayaFlux issue templates](https://github.com/MayaFlux/MayaFlux/issues/new/choose).

## Development

To build or contribute to Weave itself, see the [development guide](docs/develop.md).

## License

GNU General Public License v3.0. See [LICENSE.txt](LICENSE.txt).
