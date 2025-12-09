# Weave for Linux

Complete guide for installing, using, and troubleshooting Weave on Linux.

## Installation

### Prerequisites

- Linux x86_64 (kernel 5.0+)
- Python 3.8+
- GTK4
- ~2GB free disk space
- Internet connection

**Distribution Requirements:**

Weave requires modern toolchain versions (GCC 15, LLVM 21, GLFW 3.4+ with Wayland support). Check your distro:

- **Arch Linux** - Run `pacman -Syu` (always up-to-date)
- **Fedora** - Fedora 43 or later
- **Ubuntu** - Ubuntu 25 or later
- **openSUSE** - Tumbleweed (rolling release, same as Arch)

Advanced users with custom compiler builds can use Weave on older distributions.

### Install from Tarball

1. **Download** `Weave-X.X.X-linux.tar.gz` from [Releases](https://github.com/MayaFlux/Weave/releases)
2. **Extract to `.local`:**
   ```bash
   tar -xzf Weave-X.X.X-linux.tar.gz -C ~/.local/
   ```
3. **Launch Weave GUI:**
   ```bash
   ~/.local/Weave-X.X.X/Weave
   ```
4. **Weave GUI opens:**
   - Select "Install MayaFlux" mode
   - Follow step-by-step installer
   - May prompt for password (sudo needed for system packages)
5. **When complete**, you can create projects

### Installed components

- **`~/.local/Weave-X.X.X/`** - Weave application, CLI tool, templates
- **MayaFlux framework** - Via package manager:
  - Arch: `mayaflux-dev-bin` from AUR
  - Fedora: `mayaflux-dev` from custom COPR
- **`~/.local/bin/weave`** - Symlink to CLI tool (added to PATH)
- **`~/.bashrc` or `~/.zshrc`** - Environment variables (MAYAFLUX_ROOT, CMAKE_PREFIX_PATH, PATH)

### Post-Installation

**Reload environment variables:**

```bash
# For bash
source ~/.bashrc

# For zsh
source ~/.zshrc
```

Or restart your terminal.

**Verify installation:**

```bash
echo $MAYAFLUX_ROOT
# Should output: ~/MayaFlux

weave --version
# Should show version number
```

---

## Creating Projects

### Using Weave GUI

1. Run: `~/.local/Weave-X.X.X/Weave`
2. Select **"Create Project"** mode
3. Enter **Project Name** (e.g., "MyFirstProject")
4. Click **"Browse..."** to select location
5. Optional: Enable "Live Coding (Lila)" or "VS Code configuration"
6. Click **"Create Project"**
7. Success shows your project location

### Using CLI Tool

```bash
# Basic project
weave new MyProject ~/Projects/

# With live coding enabled
weave new MyProject ~/Projects/ --with-lila

# Without VS Code setup
weave new MyProject ~/Projects/ --no-vscode
```

---

## Building & Running

### Quick Start

```bash
cd MyProject
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
./MyProject
```

### With VS Code

1. Open project folder: `code .`
2. VS Code should auto-detect build configuration
3. Terminal → Run Task → "Build Project"
4. Press F5 to debug (with gdb/lldb)

### Manual Build with Make

```bash
cd MyProject/build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc)
./MyProject
```

### With Ninja

```bash
cd MyProject/build
cmake .. -DCMAKE_BUILD_TYPE=Release -G Ninja
ninja
./MyProject
```

---

## Environment Variables

After installation, these are set in your shell config:

| Variable            | Value                                            | Purpose            |
| ------------------- | ------------------------------------------------ | ------------------ |
| `MAYAFLUX_ROOT`     | `~/MayaFlux` or `/usr/` (if via package manager) | Framework location |
| `CMAKE_PREFIX_PATH` | Includes `$MAYAFLUX_ROOT`                        | CMake discovery    |
| `PATH`              | Includes `~/.local/bin`                          | CLI tools          |

**To apply immediately without restarting:**

```bash
source ~/.bashrc   # or ~/.zshrc
```

---

## Troubleshooting

### "GTK4 not found" during GUI launch

**Fix:**

```bash
# Arch Linux
sudo pacman -S gtk4

# Fedora
sudo dnf install gtk4

# Ubuntu/Debian
sudo apt install libgtk-4-dev

# openSUSE
sudo zypper install gtk4-devel
```

Then run Weave again.

### "weave: command not found"

**Cause:** Environment variables not loaded

**Fix:**

```bash
source ~/.bashrc    # or ~/.zshrc
weave new MyProject
```

Or restart your terminal completely.

### CMake can't find MayaFlux

**Verify environment is set:**

```bash
echo $MAYAFLUX_ROOT
# Should show: ~/MayaFlux or /home/username/MayaFlux
```

**If empty, reload:**

```bash
source ~/.bashrc    # or ~/.zshrc
```

**If still not found, set manually in CMake:**

```bash
cd build
cmake .. -DCMAKE_PREFIX_PATH=~/.local/Weave/lib/cmake/MayaFlux -DCMAKE_BUILD_TYPE=Release
```

### "No suitable asset found" during download

**This shouldn't happen.** Weave automatically detects your distribution and installs MayaFlux via the appropriate package manager (AUR for Arch, COPR for Fedora).

If you see this error:

1. Verify you're on a supported distribution (Arch or Fedora 43+)
2. Check installation log: `~/.weave_install.log`
3. Report as issue on GitHub

### Build errors with C++23 features

**Ensure you have a modern compiler:**

```bash
g++ --version     # Should be 12+
clang++ --version # Should be 16+
```

**Update compiler:**

```bash
# Arch Linux
sudo pacman -S gcc

# Fedora
sudo dnf install gcc-c++

# Ubuntu/Debian
sudo apt install build-essential

# openSUSE
sudo zypper install gcc-c++
```

### Permission denied on ~/.local/bin/weave

**Fix permissions:**

```bash
chmod +x ~/.local/bin/weave
chmod +x ~/.local/Weave-X.X.X/Weave
```

### Dependency installation asks for password

**This is normal.** Installing system packages requires sudo. Provide your password when prompted.

### "python3 not found" during GUI launch

**Install Python:**

```bash
# Arch Linux
sudo pacman -S python

# Fedora
sudo dnf install python3

# Ubuntu/Debian
sudo apt install python3

# openSUSE
sudo zypper install python3
```

---

## Uninstalling

### Remove Everything

```bash
# Remove Weave installation
rm -rf ~/.local/Weave-*

# Remove CLI symlink
rm ~/.local/bin/weave

# Remove environment setup (optional)
nano ~/.bashrc    # or ~/.zshrc
# Find and delete lines containing MAYAFLUX_ROOT, CMAKE_PREFIX_PATH additions
```

### Remove MayaFlux Package

```bash
# Arch Linux
yay -R mayaflux-dev-bin

# Fedora
sudo dnf remove mayaflux-dev
```

---

## FAQ

**Q: Can I use Weave on Ubuntu 24 / Fedora 42 / older Arch?**

A: Weave requires modern toolchains:

- **GCC 15** (C++20/23 features)
- **LLVM 21** (live coding JIT)
- **GLFW 3.4+** with Wayland support

Check your distro versions. If you want to use Weave on older systems, you'll need to install newer compilers yourself (not officially supported).

**Q: Can I use Weave CLI and GUI together?**

A: Yes. Use whichever is more convenient. Both create the same project structure.

**Q: Can I build with different compilers?**

A: Yes. Set compiler before building:

```bash
export CC=gcc-15 CXX=g++-15
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
```

Or with Clang:

```bash
export CC=clang CXX=clang++
# ... same cmake commands
```

**Q: How do I debug my project?**

A: With VS Code and gdb/lldb installed:

```bash
# Install debugger (if not already installed)
# Arch: sudo pacman -S gdb
# Fedora: sudo dnf install gdb

# Open project in VS Code and press F5
code .
# Then F5 to debug
```

---

## Links

- **[MayaFlux Framework](https://github.com/MayaFlux/MayaFlux)** - Learn the API
- **[Back to README](../README.md)** - Overview and quick start
- **[FAQ](FAQ.md)** - Cross-platform questions
