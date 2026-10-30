# Developing Weave

The current application is Rust with a Slint GUI. Install the Rust toolchain and the native build tools for your platform. Development dependencies are separate from the downloadable packages that users run.

## Run and test

```sh
cargo run --locked
cargo run --locked -- --help
cargo test --locked
cargo fmt --check
```

On Linux, the GUI build also needs the native display, font, and graphics development libraries listed in [the workflow](../.github/workflows/build.yml). On macOS, use Apple Silicon and the Xcode command-line tools. On Windows, use the x64 MSVC toolchain and Windows SDK.

## Build packages

Run the packaging scripts on their native operating systems. They build the executable and assemble the downloadable product under `target/packaging/` unless an output directory is supplied. Replace the example version below with the version in `Cargo.toml`.

### Windows

Use PowerShell 7:

```powershell
./packaging/windows/scripts/package.ps1
./packaging/windows/scripts/verify.ps1 -Package ./target/packaging/windows/output/Weave-0.6.0-Windows-x64.exe
./tests/windows-package.ps1 -Package ./target/packaging/windows/output/Weave-0.6.0-Windows-x64.exe
```

Use this packaging build for distribution. It selects the x64 target and static MSVC runtime; an ordinary development build is not a substitute for the standalone executable. The verifier runs the copied executable and generated-project checks outside the checkout, then launches the GUI. `-SkipGui` is available for CLI-only local checks. CI attempts GUI startup with `-AllowMissingGpu`: only the exact backend error reporting no GPU adapters is recorded as unavailable coverage. Other startup errors fail validation. Local verification requires successful GPU startup by default; test it on a Windows host with a supported GPU before release.

The downloadable executable needs no sidecar files. Its license is available through `--license` and its embedded build metadata through `--package-info`. Package validation checks both from the copied executable.

### macOS

```sh
rustup target add aarch64-apple-darwin
bash packaging/macos/scripts/package.sh /tmp/weave-macos
bash packaging/macos/scripts/verify.sh /tmp/weave-macos/Weave-0.6.0-macOS-arm64.dmg
```

Local builds use ad-hoc signing without credentials. Set `WEAVE_MACOS_SIGNING_IDENTITY` when using a configured signing identity.

### Linux

Build directly on Ubuntu 26.04 or an equivalent glibc 2.43 environment:

```sh
bash packaging/linux/scripts/package.sh /tmp/weave-linux
bash packaging/linux/scripts/verify.sh /tmp/weave-linux/Weave-0.6.0-Linux-x86_64.AppImage
```

From another Linux host, the container wrapper uses Podman or Docker:

```sh
bash packaging/linux/scripts/package-container.sh
bash packaging/linux/scripts/verify.sh target/packaging/linux/output/Weave-0.6.0-Linux-x86_64.AppImage
```

## CI and releases

[The workflow](../.github/workflows/build.yml) runs native tests and package validation. Pushes to `main` and version tags trigger builds. Other branches, including `develop`, use workflow dispatch; its platform checkboxes allow selective artifact builds.

Rust dependencies and compiled workspace artifacts are cached even when later checks fail. Downloaded Linux packaging tools are saved once both tool files exist, independently of subsequent package validation.

Full successful builds publish all three platforms. Versions come from `Cargo.toml`; release tags are `v<version>` and `v<version>-dev`. Main publishes stable versions, while manual builds of other branches refresh the development prerelease and force-move its tag. Stable tags and published stable assets are preserved. Partial platform builds upload CI artifacts without publishing a release.

Test release assembly and failure handling locally without contacting GitHub:

```sh
bash tests/release.sh
```

These tests use controlled artifacts and fake Git/GitHub commands. The publishing command in `packaging/scripts/release.sh` is for the release job and changes remote releases and tags.

The former implementations and their guides are archived under `legacy/` for reference.
