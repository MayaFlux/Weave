**Platform:** macOS 14.0+ (Sonoma or later)
**Architecture:** arm64, x86_64

### Installation

**Option 1: Double-click the .pkg file** (easiest)

**Option 2: Command line**
```bash
# User-level install (recommended)
sudo installer -pkg Weave-{{VERSION}}.pkg -target CurrentUserHomeDirectory

# System-wide install  
sudo installer -pkg Weave-{{VERSION}}.pkg -target /
```

After installation, **Weave.app will launch automatically** to help you create your first project.

### What Gets Installed

1. **MayaFlux Framework** (latest stable from GitHub releases)
2. **Weave Tools**
   - GUI: `/Applications/Weave.app` - graphical project creator
   - CLI: `~/.local/bin/weave` - command-line project creator
3. **Dependencies** (via Homebrew)
   - FFmpeg, RtAudio, GLFW, Eigen, oneDPL, etc.
   - Vulkan SDK for graphics support

