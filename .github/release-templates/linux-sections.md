## Linux Installation Guide

**Platform:** Linux (x86_64)
**Requirements:** Python 3.8+, GTK4, pip

### Installation

**Quick Install (all-in-one):**

```bash
# Extract the tarball
tar -xzf Weave-linux.tar.gz -C ~/.local/

# Run Weave (GUI mode)
~/.local/Weave-0.1.0/Weave

# Or use CLI mode
~/.local/Weave-0.1.0/scripts/create_project.sh new MyProject ~/Projects/
```

**From Package Manager (recommended for distributions):**

**Arch Linux (AUR):**

```bash
yay -S weave  # Coming soon
```

**Fedora:**

```bash
sudo dnf install weave  # Coming soon
```

**Ubuntu/Debian:**

```bash
sudo apt install weave  # Coming soon
```

### What Gets Installed

1. **Weave GUI Application** - GTK4-based graphical installer and project creator
2. **Weave CLI Tool** - Command-line project creation (`weave new <name>`)
3. **Project Templates** - CMakeLists.txt, source files, VS Code configuration
4. **Dependency Installer** - Automated setup for FFmpeg, RtAudio, Vulkan SDK, etc.

### Installation Steps

1. Extract tarball: `tar -xzf Weave-linux.tar.gz`
2. Run GUI: `./Weave/Weave` (GTK4 required)
3. Follow the installation wizard
4. Choose your Linux distribution (Arch, Fedora, Ubuntu/Debian, openSUSE)
5. Dependencies will be installed automatically
6. MayaFlux framework downloaded from GitHub
7. Environment variables configured automatically

### Creating Your First Project

**Using the GUI:**

1. Launch Weave application (from installed tarball or system menu)
2. Select "Create Project"
3. Enter project name and location
4. Click "Create Project"

**Using the CLI:**

```bash
weave new MyProject ~/Projects/
cd ~/Projects/MyProject
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
./MyProject
```

### Distribution Support

- **Arch Linux:** pacman package manager
- **Fedora:** dnf package manager
- **Ubuntu/Debian:** apt package manager
- **openSUSE:** zypper package manager

Weave automatically detects your distribution and uses the native package manager to install dependencies.

### System Requirements

- **OS:** Linux (x86_64)
- **Python:** 3.8 or later
- **GTK4:** For graphical interface
- **RAM:** 4GB minimum (8GB+ for builds)
- **Disk:** ~2GB for MayaFlux + dependencies
- **Internet:** Required for initial downloads
- **Permissions:** Standard user (sudo for system-wide dependency installation)

### Post-Installation

After installation, update your shell configuration:

```bash
# For bash
echo 'export MAYAFLUX_ROOT=~/MayaFlux' >> ~/.bashrc
echo 'export PATH=$MAYAFLUX_ROOT/bin:$PATH' >> ~/.bashrc
source ~/.bashrc

# For zsh
echo 'export MAYAFLUX_ROOT=~/MayaFlux' >> ~/.zshrc
echo 'export PATH=$MAYAFLUX_ROOT/bin:$PATH' >> ~/.zshrc
source ~/.zshrc
```

Then verify the installation:

```bash
weave new TestProject ~/test-project
cd ~/test-project && mkdir build && cd build
cmake .. && make && ./TestProject
```
