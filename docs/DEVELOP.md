# Weave Development Guide

For contributors working on Weave itself.

---

## Repository Structure

```
Weave/
├── macos/
│   ├── WeaveGUI.swift              # SwiftUI installer and project creator
│   ├── Weave.sh                    # CLI tool (weave new / update / community)
│   ├── CommunityManifest.swift     # community.json parser and validator
│   ├── CommunityUpdate.swift       # Batch module preparation and registration
│   └── scripts/
│       └── build_weave_app.sh      # Builds Weave.app and DMG
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
│   ├── Shared/                     # Shared models and utilities, community manifest and update logic
│   └── Weave.sln
│
├── linux/
│   ├── lib/
│   │   ├── main.py                 # GUI entry point
│   │   ├── cli.py                  # CLI tool
│   │   ├── config.py               # Configuration loader
│   │   ├── community_manifest.py   # community.json parser and validator
│   │   ├── community_update.py     # Batch module preparation and registration
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
│   ├── validate.sh                 # Project diagnostics (Linux)
│   ├── run-report.sh               # Report runner (Linux)
│   ├── run-report-macos.sh         # Report runner (macOS)
│   ├── run-report.ps1              # Report runner (Windows)
│   ├── validate-macos.sh           # Project diagnostics (macOS)
│   ├── validate.ps1                # Project diagnostics (Windows)
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
├── tests/
│   └── vectors/
│       └── community/              # Manifest corpus run by all three platform suites
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

Requires Xcode 26 or later with the macOS 26 SDK.

```bash
git clone https://github.com/MayaFlux/Weave.git
cd Weave
./macos/scripts/build_weave_app.sh 0.X.Y
# Output: build/macos/Weave.dmg
```

Produces a DMG for macOS 26 or later. The app is ad-hoc signed. Users will need to allow it via System Settings > Privacy & Security on first launch.

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

The AppImage bundles a Python standalone runtime, GTK4, libadwaita, typelibs, and the EGL and GL libraries required to load GTK. If Podman is not available the script runs directly, but output compatibility is not guaranteed.

After building, run the Linux AppImage integration tests with:

```bash
python3 -m unittest discover -s tests/linux/integration -p 'test_*.py' -v
```

CI runs these tests on the built AppImage before uploading it. They extract the image, verify that the Python and GTK runtime loads from the bundle, and exercise the packaged installer process and CLI with local fixtures. A second runtime test runs in a minimal Ubuntu container with no GTK or graphics libraries installed. The AppImage still uses the host kernel, glibc, display server, hardware driver, shell, and native package manager.

The manual build workflow has a checkbox for each platform, enabled by default. It uploads artifacts for the selected platforms and publishes a release only when all three builds succeed.

### Manifest vectors

`tests/vectors/community/` holds `community.json` manifests that the Linux, macOS and Windows parsers must judge identically. A file named `valid-<case>.json` must be accepted and `invalid-<case>.json` must be rejected. Each runner enumerates the folder, passes the manifest's own `name` as the expected module name, compares only accepted or rejected, and fails if the folder is empty or holds a file with any other prefix.

- Linux: `tests/linux/test_community_manifest_vectors.py`
- macOS: `tests/macos/unit/CommunityManifestVectorTests.swift`
- Windows: `tests/windows/unit/CommunityManifestVectorTests.cs`

An invalid vector differs from a valid manifest by one fault, so it cannot be rejected for an unrelated reason. Name mismatch against the requested module is not a vector, because the runners always pass the manifest's own name. Each platform tests it in its own suite. Add a vector whenever [COMMUNITY_MANIFEST.md](COMMUNITY_MANIFEST.md) gains a rule. When a platform disagrees, fix the parser that departs from that document, or clarify the document.

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
