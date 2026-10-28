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
```

Project creation, community-module creation, and project updates use the same Rust operations from the GUI and CLI. Generated projects remain ordinary CMake directories; MayaFlux, CMake, compilers, and Git are host tools and are not bundled into projects.

## Build and test

```sh
cargo build --locked --release
cargo test --locked
```

The Slint UI is compiled into the executable during the build. `Cargo.lock` pins the application and UI compiler dependencies.

## Repository layout

- `src/`: shared Rust implementation, CLI/GUI wiring, and narrow platform adapters
- `ui/`: Slint theme, shared controls, screens, and result dialog
- `templates/`: embedded project and community-module templates
- `tests/`: Rust integration tests and shared fixtures
- `legacy/`: former per-platform applications, their platform test suites, and archived development guides
- `.github/`: CI and release workflows
- `Agents.md`: current implementation, packaging, platform, and test requirements

The active workflow builds and tests the Rust application on native Linux, macOS, and Windows runners. It runs on pushes to `main` and version tags; `develop` is manual through workflow dispatch. Rust distribution packaging is still in progress, so this workflow does not publish release artifacts yet. The archived applications and their CI helpers are retained for feature and packaging comparisons, but are not built or tested by active CI.

## Documentation

- [Community manifest contract](docs/COMMUNITY_MANIFEST.md)
- [Current development and distribution rules](Agents.md)
- [Archived development guide](legacy/docs/DEVELOP.md)

## License

GNU General Public License v3.0. See [LICENSE.txt](LICENSE.txt).
