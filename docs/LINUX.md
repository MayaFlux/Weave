# Weave for Linux

**Platform:** Linux x86_64

---

## Supported Distributions

Weave detects your distribution automatically and installs MayaFlux via the native package manager. Three distributions are supported:

| Distribution | Minimum version | Package manager | Channel: Stable | Channel: Development |
| ------------ | --------------- | --------------- | --------------- | -------------------- |
| Arch Linux   | rolling         | AUR             | `mayaflux-bin`  | `mayaflux-dev-bin`   |
| Fedora       | 43              | COPR            | `mayaflux`      | `mayaflux-dev`       |
| Ubuntu       | 25              | PPA             | `mayaflux`      | `mayaflux-edge`      |

Only one channel can be installed at a time. If a conflicting package is already installed, Weave will ask you to confirm its removal before proceeding.

---

## Installing MayaFlux

1. Download `Weave-linux.AppImage` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. Make it executable:
   ```bash
   chmod +x Weave-linux.AppImage
   ```
3. Launch Weave:
   ```bash
   ./Weave-linux.AppImage
   ```
4. Choose **Install MayaFlux**
5. Select a release channel. Stable is recommended for most users
6. Enter your sudo password when prompted. It is needed once to install system packages and is not stored
7. Restart your terminal when complete

### What gets installed

- MayaFlux framework via your distribution's package manager
- `weave` CLI tool symlinked at `~/.local/bin/weave`

### What each distro does

**Arch Linux** : clones the package from the AUR (`mayaflux-bin` or `mayaflux-dev-bin`) and installs it with `makepkg -si`, even if `yay` or `paru` is present. The packages repackage the prebuilt MayaFlux release, so nothing is compiled.

**Fedora** : enables three COPR repositories (`ranjithshegde/spirv-cross`, `ranjithshegde/asio-standalone`, and `ranjithshegde/mayaflux` or `ranjithshegde/mayaflux-dev`) then installs via `dnf`.

**Ubuntu** : adds the `ppa:mayaflux/mayaflux` or `ppa:mayaflux/mayaflux-dev` PPA, runs `apt-get update`, then installs.

---

## Creating a Project

Open Weave and choose **Create Project**. Enter a name, pick a destination, optionally enable **Live Coding (Lila)** and **Copy MayaFlux shader sources**, then click **Create Project**. Shader source copying preserves nested paths.

The generated project includes a `CMakePresets.json` with `debug` and `release` presets. Any CMake-aware IDE (VS Code, CLion) will pick these up automatically when you open the project folder.

### Building

```bash
cd MyProject

# with presets
cmake --preset release
cmake --build --preset release
./build/MyProject

# without presets
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel
./build/MyProject
```

### Diagnostics

Each new project includes `validate.sh`. Run it from the project folder when something does not work:

```bash
./validate.sh
```

It checks project files, presets, community module references, `mayaflux.json`, local shaders and the MayaFlux installation, and lists your OS, GPU and Vulkan driver details. It does not configure, build or change anything. Include its output when reporting a problem. See the [Weave README](../README.md#reporting-a-problem) for the report steps.

### Reporting a problem

Each new project also includes `run-report.sh`. Build the project, then run it from the project folder:

```bash
./run-report.sh
```

It runs the project with the report configuration that MayaFlux installs, so Vulkan validation is enabled and a complete log is written to `data/log/mayaflux_report.log`. Reproduce the problem, then close the application. The console output is saved to `mayaflux_run.txt`, and the script exits with the application's exit code. Attach both files, and the output of `./validate.sh`, to the issue. Existing logs are kept with a `.previous-` suffix.

The script finds MayaFlux through `--mayaflux-root`, then `MAYAFLUX_ROOT`, then `/usr` and `/usr/local`. Use `--configuration Debug` when more than one build configuration exists, `--executable path/to/binary` for a custom layout, or `--project-name` when the executable name differs from the project. Projects created before this script existed do not have it; create a new project or copy `run-report.sh` from the Weave templates folder into the project.

---

## Community Modules

### Adding a module

Open Weave and choose **Add Community Module**, select your project directory, and enter the module name. Weave fetches the registry, checks version compatibility, clones the module into `community/<name>/`, and registers it in `community.cmake`. Rebuild your project after adding modules. If the module declares dependencies in its `community.json`, Weave tells you but does not install them. Install them with your package manager.

From a terminal, `weave update <project-dir> <module> [module ...]` does the same. It accepts several names, separated by spaces or commas (`terrain_field,noise_bank`), skips duplicates, and registers the modules only after every one has been cloned and checked. The terminal command does not read `community.json`; use Weave to see what a module declares.

### Creating a module

Choose **Create Community Module** in Weave. Enter a name (snake_case), description, minimum MayaFlux version, and whether the module requires Lila. Weave scaffolds the module with `src/`, `test/`, a `CMakePresets.json`, and a git repository ready to push.

See the [Weave README](../README.md) for details on module structure and registry submission.

---

## Troubleshooting

### "weave: command not found"

`~/.local/bin` may not be on your PATH. Add it:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Add that line to your shell config to make it permanent, then restart your terminal.

### CMake can't find MayaFlux

Check that `MAYAFLUX_ROOT` is set:

```bash
echo $MAYAFLUX_ROOT
```

If empty, restart your terminal so the package manager's environment is picked up. If it is still not found, you can point CMake at it directly:

```bash
cmake -B build -DCMAKE_BUILD_TYPE=Release -DMAYAFLUX_ROOT=/path/to/mayaflux
```

### Switching channels

If you switch from Stable to Development (or vice versa), Weave detects the conflicting package and shows a confirmation dialog before removing it. Both channels cannot coexist.

### Installation fails on an unsupported distribution

Weave detects distros by checking for `pacman`, `dnf`, or `apt-get` in that order. If none are found, installation is not supported. Running Arch, Fedora, or Ubuntu inside a container or WSL should work if the package managers are available.

---

## Uninstalling

```bash
# Remove Weave
rm Weave-linux.AppImage
rm ~/.local/bin/weave

# Remove MayaFlux — Arch
sudo pacman -R mayaflux-bin     # or mayaflux-dev-bin

# Remove MayaFlux — Fedora
sudo dnf remove mayaflux        # or mayaflux-dev

# Remove MayaFlux — Ubuntu
sudo apt-get remove mayaflux    # or mayaflux-edge

# Remove environment config (optional)
# Edit ~/.bashrc or ~/.zshrc and delete the MAYAFLUX_ROOT lines
```

---

## Links

- [Weave README](../README.md)
- [MayaFlux](https://github.com/MayaFlux/MayaFlux)
- [LilaCode (VS Code)](https://github.com/MayaFlux/LilaCode)
- [lila.nvim (Neovim)](https://github.com/MayaFlux/lila.nvim)
