# Weave for macOS

**Platform:** macOS 15.0 (Sequoia) or later, Universal Binary (arm64 + x86_64)

---

## Installing MayaFlux

1. Download `Weave-macos.dmg` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. Open the DMG and double-click **Weave.app**
3. If an "unverified developer" warning appears, close it, go to **System Settings > Privacy & Security**, scroll down to find Weave, click **Open Anyway**, then double-click Weave.app again
4. Choose **Install MayaFlux**
5. Select a release channel : Stable is recommended for most users, Development gives you the latest features
6. If Homebrew is not installed, Weave will ask for your password once to set it up. Your password is not stored
7. Wait for installation to complete. Progress is shown in the app
8. Restart your terminal when done

### What gets installed

- Homebrew (if not already present)
- MayaFlux framework via `brew install mayaflux/mayaflux/mayaflux` (or `mayaflux-dev` for the development channel)
- `weave` CLI tool at `~/.local/bin/weave`
- Environment configuration added to `$ZDOTDIR/.zshenv`

### Environment

Weave adds the following to `$ZDOTDIR/.zshenv`:

```zsh
source "<homebrew-prefix>/env.sh"
export PATH="$HOME/.local/bin:$PATH"
```

The sourced `env.sh` sets `MAYAFLUX_ROOT`, `CMAKE_PREFIX_PATH`, `VULKAN_SDK`, and any other variables the framework requires. To apply without restarting your terminal:

```bash
source $ZDOTDIR/.zshenv
```

---

## Creating a Project

Open Weave.app and choose **Create Project**. Enter a name, pick a destination, optionally enable **Live Coding (Lila)** and **VS Code configuration**, then click **Create Project**.

The generated project includes a `CMakePresets.json` with `debug` and `release` presets. Any CMake-aware IDE (VS Code, CLion, Visual Studio) will pick these up automatically when you open the project folder.

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

Open Weave.app, choose **Add Community Module**, select your project directory, and enter the module name. Weave fetches the registry, checks version compatibility, clones the module into `community/<name>/`, and registers it in `community.cmake`. Rebuild your project after adding modules.

### Creating a module

Choose **Create Community Module** in Weave.app. Enter a name (snake_case), description, minimum MayaFlux version, and whether the module requires Lila. Weave scaffolds the module with `src/`, `test/`, a `CMakePresets.json`, and a git repository ready to push.

See the [Weave README](../README.md) for details on the module structure and how to submit to the community registry.

---

## Troubleshooting

### "weave: command not found"

Your shell environment has not been loaded yet. Run:

```bash
source $ZDOTDIR/.zshenv
```

Or restart your terminal.

### Weave.app won't open

If macOS says the app is damaged or from an unidentified developer:

```bash
xattr -rd com.apple.quarantine /path/to/Weave.app
```

Then try opening again.

### CMake can't find MayaFlux

Check that the environment is loaded:

```bash
echo $MAYAFLUX_ROOT
```

If empty, run `source $ZDOTDIR/.zshenv` and try again.

### Switching channels

If you want to switch between Stable and Development, Weave will detect the conflicting formula and ask you to confirm removal before installing the other. Only one channel can be installed at a time.

---

## Uninstalling

```bash
# Remove the MayaFlux framework
brew uninstall mayaflux        # or mayaflux-dev
brew autoremove

# Remove the CLI tool
rm ~/.local/bin/weave

# Remove environment config
# Edit $ZDOTDIR/.zshenv and delete the MayaFlux lines
```

---

## Links

- [Weave README](../README.md)
- [MayaFlux](https://github.com/MayaFlux/MayaFlux)
- [LilaCode (VS Code)](https://github.com/MayaFlux/LilaCode)
- [lila.nvim (Neovim)](https://github.com/MayaFlux/lila.nvim)
