# Weave

**Installer and project tool for MayaFlux.**

Weave installs MayaFlux, creates projects, and manages community modules. Download it from [Releases](https://github.com/MayaFlux/Weave/releases).

---

## Installing MayaFlux

Launch Weave and choose **Install MayaFlux**. Weave downloads the framework, installs all dependencies, and configures your environment. Restart your terminal when it finishes.

---

## Creating a Project

Launch Weave and choose **Create Project**. Enter a name and pick a destination. Weave generates a ready-to-build project with the following structure:

```
MyProject/
├── CMakeLists.txt
├── CMakePresets.json
├── community.cmake
├── cmake/
│   ├── mayaflux.cmake
│   ├── shaders.cmake
│   └── build_community.cmake
├── src/
│   ├── main.cpp
│   └── user_project.hpp
├── data/
│   └── shaders/
├── .vscode/             # if VS Code configuration was enabled
│   ├── settings.json
│   ├── tasks.json
│   └── launch.json
├── .gitignore
└── README.md
```

`src/user_project.hpp` is where you write your code. It defines two functions:

- `settings()` runs before the engine starts. Configure sample rate, buffer size, graphics API, logging, and so on.
- `compose()` is where you build your audio/visual processing graph.

`CMakeLists.txt` is yours to edit. Add source files, link libraries, and set compiler flags as needed. The MayaFlux integration lives in `cmake/mayaflux.cmake` and is included automatically at the end of `CMakeLists.txt`.

### Building

`CMakePresets.json` ships with every project, giving IDEs and editors a zero-config starting point. It defines `debug` and `release` presets, both using Ninja into `build/`.

```bash
# using presets
cmake --preset release
cmake --build --preset release
./build/MyProject

# or without presets
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel
./build/MyProject
```

On Windows, the binary lands in `build\MyProject.exe`. CLion, VS Code with the CMake extension, and Visual Studio all pick up the presets automatically when you open the project folder.

### Live Coding (Lila)

To enable live coding, check **Enable Live Coding (Lila)** in the project creation dialog. This links your project against `MayaFlux::MayaFluxHost`, embedding the Lila JIT compiler in your process. You can then connect [LilaCode](https://github.com/MayaFlux/LilaCode) (VS Code) or [lila.nvim](https://github.com/MayaFlux/lila.nvim) (Neovim) to evaluate C++ code against your running application in real time.

---

## Community Modules

Community modules are C++ source libraries that compile directly into your project. There is no plugin boundary or ABI to worry about; modules are just code.

### Adding Modules

In Weave, open your project and choose **Add Community Module**. Enter the module name and Weave will:

1. Fetch the registry from [community-sources-registry](https://github.com/MayaFlux/community-sources-registry)
2. Check that your MayaFlux version meets the module's minimum requirement
3. Clone the module into `community/<name>/`
4. Register it in `community.cmake`

Rebuild your project after adding modules.

Modules that already exist in `community/` are skipped. You can add multiple modules at once.

### How Modules Integrate

`community.cmake` lists module names, one per line. At configure time, `cmake/build_community.cmake` reads this file and calls `add_community()` for each entry, which includes the module's own `<name>.cmake`, builds it as an OBJECT library, and links it into your project. No manual CMake edits needed.

### Creating a Module

Choose **Create Community Module** in Weave. Enter a name (snake_case), a description, a minimum MayaFlux version, and whether the module requires Lila. Weave scaffolds:

```
my_module/
├── src/                        <- your C++ sources go here
├── test/
│   ├── CMakeLists.txt
│   └── test_my_module.cpp
├── my_module.cmake
├── CMakePresets.json
├── community.json
└── .gitignore
```

`my_module.cmake` defines the module as an OBJECT library linking `MayaFlux::MayaFluxLib`. If Lila is required, it also links `MayaFlux::MayaFluxHost`. Users call your constructors directly from their `compose()`.

`community.json` carries registry metadata: name, description, minimum MayaFlux version, and whether Lila is required.

The `test/` directory contains a standalone CMake project for building and testing your module independently. `CMakePresets.json` at the module root points both presets at `test/build`, so any IDE opened on the module folder just works:

```bash
cmake --preset release
cmake --build --preset release
```

The module is initialized as a git repository. Push it to GitHub and submit a PR to [community-sources-registry](https://github.com/MayaFlux/community-sources-registry) to list it publicly.

---

## Links

- [MayaFlux](https://github.com/MayaFlux/MayaFlux)
- [LilaCode (VS Code)](https://github.com/MayaFlux/LilaCode)
- [lila.nvim (Neovim)](https://github.com/MayaFlux/lila.nvim)
- [Community Registry](https://github.com/MayaFlux/community-sources-registry)

---

## License

GNU General Public License v3.0. See [LICENSE](LICENSE) for full terms.
