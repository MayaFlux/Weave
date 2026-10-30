# Weave

Weave is MayaFlux's Rust application for creating CMake projects, creating community modules, and updating projects with community modules. The shared application lives at the repository root; the former Python, Swift, and .NET applications are archived under [`legacy/`](legacy/).

## Run from source

Install the Rust toolchain for development, then run the GUI:

```sh
cargo run --locked
```

The CLI runs independently without starting the GUI:

```sh
cargo run --locked -- --help
cargo run --locked -- new AudioDemo /path/to/projects --with-lila
cargo run --locked -- update /path/to/AudioDemo terrain_field,audio_tools
cargo run --locked -- community audio_tools /path/to/modules --description "Audio helpers"
cargo run --locked -- install stable
cargo run --locked -- install development --replace-conflict
```

Project creation, community-module creation, and project updates use the same Rust operations from the GUI and CLI. Generated projects remain ordinary CMake directories; MayaFlux, CMake, compilers, and Git are host tools and are not bundled into projects.

Windows installation is available from both the GUI and `weave install <stable|development>`. It checks/acquires 7-Zip and Microsoft C++ prerequisites, downloads and verifies the selected MayaFlux x64 release, verifies/installs CMake 3.31+, Git, Ninja, LLVM, FFmpeg Shared and Vulkan SDK through winget, configures the machine environment, and deploys Weave's embedded templates and tools. Installation targets `C:\MayaFlux` and requires Windows 10 22H2 (build 19045) or later. Windows requests native UAC consent for this host installation; project and community operations stay non-elevated. Missing winget or other host tools produce a recoverable error.

Switching stable/development streams asks for replacement approval in the GUI; the CLI uses `--replace-conflict`. Development builds use a 24-hour UTC freshness check. An already-current MayaFlux payload is verified, then the remaining setup stages still run. Installation logs append to `%LOCALAPPDATA%\weave_install.log`; completion reports any restart requirement. The Windows workflow has controlled Rust tests, but live installation and downloadable Windows package acceptance still require native validation.

## Build and test

```sh
cargo build --locked --release
cargo test --locked
```

The Slint UI and project templates are compiled into the executable during the build. `Cargo.lock` pins the application and UI compiler dependencies.

On x86_64 Linux, build the AppImage on Ubuntu 26.04 (or an equivalent glibc 2.43 build environment), then validate the downloadable artifact. On Arch or another newer Linux host, the container wrapper uses Podman or Docker to provide that build environment:

```sh
bash linux/scripts/package_weave_linux_container.sh
bash linux/scripts/verify_weave_linux_package.sh build/linux/rust-package/output/Weave-0.6.0-Linux-x86_64.AppImage
```

On Ubuntu 26.04 itself, `bash linux/scripts/package_weave_linux.sh /tmp/weave-linux` builds directly without a container.

The AppImage includes the Weave executable and its resources. The host needs FUSE to launch it, plus a supported Linux display service and graphics driver for the GUI. It does not need Rust, Python, GTK, or a separate Weave runtime. The supported executable ABI baseline is glibc 2.43, matching Ubuntu 26.04 as MayaFlux's minimum Ubuntu release. Host tools such as Git, CMake, and compilers remain separate dependencies for the operations that use them.

On an Apple Silicon Mac with the Rust target and Xcode tools installed, build and validate the macOS package with:

```sh
bash macos/scripts/package_weave_macos.sh /tmp/weave-macos
bash macos/scripts/verify_weave_macos_package.sh /tmp/weave-macos/Weave-0.6.0-macOS-arm64.dmg
```

The default app bundle is ad-hoc signed to seal its resources and allow integrity checks. It is not Developer ID signed or notarized, so this package is not a warning-free browser download; Gatekeeper may require the user to approve opening it.

## Repository layout

- `src/`: shared Rust implementation, CLI/GUI wiring, and narrow platform adapters
- `ui/`: Slint theme, shared controls, screens, and result dialog
- `templates/`: embedded project and community-module templates
- `tests/`: Rust integration tests and shared fixtures
- `legacy/`: former per-platform applications, their platform test suites, and archived development guides
- `.github/`: CI and release workflows
- `Agents.md`: current implementation, packaging, platform, and test requirements

The active workflow builds and tests the Rust application on native Linux, macOS, and Windows runners. It packages and validates the Linux x86_64 AppImage and the Apple Silicon `.app` and `.dmg`, then uploads them as workflow artifacts. The workflow runs on pushes to `main` and version tags; `develop` remains manual through workflow dispatch. The Windows package and GitHub Release publishing are still in progress. The archived applications and CI helpers are retained for feature and packaging comparisons, but are not built or tested by active CI.

## Documentation

- [Community manifest contract](docs/COMMUNITY_MANIFEST.md)
- [Current development and distribution rules](Agents.md)
- [Archived development guide](legacy/docs/DEVELOP.md)

## License

GNU General Public License v3.0. See [LICENSE.txt](LICENSE.txt).
