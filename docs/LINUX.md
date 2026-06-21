# Weave for Linux

**Platform:** Linux x86_64

---

## Supported Distributions

Weave detects your distribution automatically and installs MayaFlux via the native package manager. Three distributions are supported:

| Distribution | Minimum version | Package manager | Channel: Stable | Channel: Development |
| ------------ | --------------- | --------------- | --------------- | -------------------- |
| Arch Linux   | rolling         | AUR             | `mayaflux`      | `mayaflux-dev-bin`   |
| Fedora       | 43              | COPR            | `mayaflux`      | `mayaflux-dev`       |
| Ubuntu       | 25              | PPA             | `mayaflux`      | `mayaflux-edge`      |

Only one channel can be installed at a time. If a conflicting package is already installed, Weave will ask you to confirm its removal before proceeding.

---

## Installing MayaFlux

1. Download `Weave-X.X.X-linux.tar.gz` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. Extract it:
   ```bash
   tar -xzf Weave-X.X.X-linux.tar.gz -C ~/.local/
   ```
3. Launch Weave:
   ```bash
   ~/.local/Weave-X.X.X/Weave
   ```
4. Choose **Install MayaFlux**
5. Select a release channel. Stable is recommended for most users
6. Enter your sudo password when prompted. It is needed once to install system packages and is not stored
7. Restart your terminal when complete

### What gets installed

- MayaFlux framework via your distribution's package manager
- `weave` CLI tool symlinked at `~/.local/bin/weave`

### What each distro does

**Arch Linux** : installs from the AUR using `yay` or `paru` if available, otherwise falls back to cloning and running `makepkg`.

**Fedora** : enables three COPR repositories (`ranjithshegde/spirv-cross`, `ranjithshegde/asio-standalone`, and `ranjithshegde/mayaflux` or `ranjithshegde/mayaflux-dev`) then installs via `dnf`.

**Ubuntu** : adds the `ppa:mayaflux/mayaflux` or `ppa:mayaflux/mayaflux-dev` PPA, runs `apt-get update`, then installs.

---

## Creating a Project

Open Weave and choose **Create Project**. Enter a name, pick a destination, optionally enable **Live Coding (Lila)** and **VS Code configuration**, then click **Create Project**.

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

---

## Community Modules

### Adding a module

Open Weave and choose **Add Community Module**, select your project directory, and enter the module name. Weave fetches the registry, checks version compatibility, clones the module into `community/<name>/`, and registers it in `community.cmake`. Rebuild your project after adding modules.

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
rm -rf ~/.local/Weave-X.X.X
rm ~/.local/bin/weave

# Remove MayaFlux — Arch
sudo pacman -R mayaflux         # or mayaflux-dev-bin

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
