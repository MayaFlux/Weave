# Weave Development Guide

For contributors working on Weave itself.

---

## Repository Structure

```
Weave/
├── macos/
│   ├── WeaveGUI.swift              # SwiftUI installer and project creator
│   ├── Weave.sh                    # CLI tool (weave new / update / community)
│   └── scripts/
│       └── build_weave_app.sh      # Builds universal Weave.app and DMG
│
├── windows/
│   ├── Weave/
│   │   ├── Program.cs
│   │   ├── MainWindow.cs
│   │   ├── Steps/                  # Installation step views
│   │   ├── UI/
│   │   │   ├── Pages/              # Installation step pages
│   │   │   ├── Layout/             # Layout manager
│   │   │   └── Project/            # Project creator, community module views
│   │   └── Weave.csproj
│   ├── Shared/                     # Shared models and utilities
│   └── Weave.sln
│
├── linux/
│   ├── lib/
│   │   ├── main.py                 # GUI entry point
│   │   ├── cli.py                  # CLI tool
│   │   ├── config.py               # Configuration loader
│   │   ├── modes/                  # Installation, project, community modes
│   │   └── ui/                     # GTK4 theme and CSS
│   ├── scripts/
│   │   ├── build_appimage.sh       # AppImage build script
│   │   └── create_project.sh       # Project creation (CLI backend)
│   ├── Weave                       # Launcher script
│   ├── weave-config.json
│   └── pyproject.toml
│
├── templates/                      # Shared project templates (all platforms)
│   ├── CMakeLists.txt
│   ├── CMakePresets.json
│   ├── main.cpp
│   ├── user_project.hpp
│   ├── .gitignore
│   ├── cmake/
│   │   ├── mayaflux.cmake
│   │   ├── shaders.cmake
│   │   └── build_community.cmake
│   ├── community/
│   │   ├── module.cmake
│   │   ├── community.json
│   │   ├── CMakePresets.json
│   │   └── test/
│   │       └── CMakeLists.txt
│   └── vscode/
│       ├── settings.json
│       ├── tasks.json
│       └── launch.json
│
├── .github/
│   ├── workflows/
│   │   └── build.yml               # CI/CD pipeline
│   └── scripts/
│       ├── verify-macos-build.sh
│       ├── verify-windows-build.ps1
│       ├── verify-linux-files.sh
│       └── generate-release-body.sh
│
└── docs/
    ├── MACOS.md
    ├── WINDOWS.md
    ├── LINUX.md
    └── DEVELOP.md
```

---

## Building from Source

### macOS

Requires Xcode Command Line Tools and Swift 5.9+.

```bash
git clone https://github.com/MayaFlux/Weave.git
cd Weave
./macos/scripts/build_weave_app.sh 0.X.Y
# Output: build/macos/Weave.dmg
```

Produces a universal binary DMG (arm64 + x86_64). The app is ad-hoc signed. Users will need to allow it via System Settings > Privacy & Security on first launch.

### Windows

Requires .NET 8 SDK.

```powershell
git clone https://github.com/MayaFlux/Weave.git
cd Weave
dotnet publish windows/Weave/Weave.csproj `
  -c Release -p:Platform=x64 `
  -p:PublishSingleFile=true `
  -p:SelfContained=true `
  -p:RuntimeIdentifier=win-x64 `
  -p:IncludeNativeLibrariesForSelfExtract=true
# Output: self-contained Weave.exe
```

All templates and resources are embedded as `EmbeddedResource` entries in `Weave.csproj` at build time. Nothing is loaded from disk at runtime.

### Linux

Requires Podman. The build script re-execs itself inside an Ubuntu 25.10 container to ensure a consistent glibc and GTK4 baseline.

```bash
git clone https://github.com/MayaFlux/Weave.git
cd Weave
./linux/scripts/build_appimage.sh
# Output: build/linux/Weave-X.X.X-x86_64.AppImage
```

The AppImage bundles a Python standalone runtime, GTK4, libadwaita, and all typelibs. If Podman is not available the script runs directly, but output compatibility is not guaranteed.

---

## CI/CD

GitHub Actions builds all three platforms in parallel on every push. The release job runs after all three complete.

**Triggers:**

- Push to `main` — stable release build, uses the latest `v0.X.Y` tag
- Push of a `v0.X.Y` tag — stable release
- Push of a `v0.X.Y-dev` tag or any other branch — dev/prerelease build

**Artifacts:**

| Platform | File |
| -------- | ---- |
| macOS    | `Weave-macos.dmg` |
| Linux    | `Weave-linux.AppImage` |
| Windows  | `Weave-windows.zip` |

SHA256 hashes are computed per artifact and included in the GitHub Release body.

---

## Release Process

### 1. Update version numbers

- `linux/pyproject.toml` : `version = "0.X.Y"`
- `windows/Weave/Weave.csproj` : `<Version>0.X.Y</Version>`
- macOS version is read from the git tag at build time

### 2. Tag and push

For a stable release:

```bash
git tag v0.X.Y
git push origin v0.X.Y
```

For a dev/prerelease:

```bash
git tag v0.X.Y-dev
git push origin v0.X.Y-dev
```

CI builds all platforms, creates a GitHub Release, and attaches the artifacts automatically.

### 3. Generate release body (optional, if running locally)

```bash
.github/scripts/generate-release-body.sh \
  --version 0.X.Y \
  --macos-sha256 <hash> \
  --linux-sha256 <hash> \
  --windows-sha256 <hash> \
  --output release-body.md
```

---

## Code Style

**Swift (macOS)** : SwiftUI, 4-space indent, Apple Swift API guidelines.

**C# (Windows)** : .NET 8, WinForms, Microsoft C# conventions, property-based initialization, explicit error handling and logging throughout.

**Python (Linux)** : Python 3.8+, PEP 8, type hints where applicable, GTK4/libadwaita for GUI.

**Bash/zsh** : `set -euo pipefail` on all scripts, POSIX-compatible where possible, no hardcoded paths.

---

## Contributing

Before submitting a PR:

- Build and test on your platform
- Test on a clean system without MayaFlux pre-installed where possible
- Follow the commit style used in the repo (`feat(scope): description` with bullet-point body)
- Keep unrelated fixes in separate commits
- Update the relevant doc if any user-facing behaviour changes

---

## Links

- [Weave README](../README.md)
- [MayaFlux](https://github.com/MayaFlux/MayaFlux)
- [Community Registry](https://github.com/MayaFlux/community-sources-registry)
