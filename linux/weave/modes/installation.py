#!/usr/bin/env python3
"""Installation mode - multi-step setup with GTK4"""

import gi

gi.require_version("Gtk", "4.0")

from gi.repository import Gtk, GLib
import asyncio
import subprocess
import os
import json
import urllib.request
import urllib.error
import tarfile
import time
from pathlib import Path
from typing import Optional, Dict


class ConfirmationStep:
    """Step 0: Confirmation before starting"""

    def build_ui(self, container):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        box.set_margin_top(30)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup("<span size='18000' weight='bold'>Ready to Install?</span>")
        title.set_halign(Gtk.Align.START)
        box.append(title)

        info = Gtk.Label()
        info.set_markup("""This will install:
• MayaFlux framework
• Build tools (CMake, Git, compiler)
• Dependencies (FFmpeg, RtAudio, Vulkan SDK)
• Weave project creator

<b>Requires:</b> Internet connection, ~2GB disk space""")
        info.set_wrap(True)
        info.set_halign(Gtk.Align.START)
        box.append(info)

        spacer = Gtk.Box()
        spacer.set_vexpand(True)
        box.append(spacer)

        while container.get_first_child():
            container.remove(container.get_first_child())
        container.append(box)

    async def execute(self):
        return True


class SystemCheckStep:
    """Step 1: System verification"""

    def build_ui(self, container):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=15)
        box.set_margin_top(20)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup("<span size='16000' weight='bold'>Step 1: System Check</span>")
        title.set_halign(Gtk.Align.START)
        box.append(title)

        self.status = Gtk.Label(label="Checking...")
        self.status.set_halign(Gtk.Align.START)
        box.append(self.status)

        scroll = Gtk.ScrolledWindow()
        scroll.set_hexpand(True)
        scroll.set_vexpand(True)
        scroll.set_min_content_height(250)

        self.log = Gtk.TextView()
        self.log.set_editable(False)
        self.log.add_css_class("monospace")
        scroll.set_child(self.log)
        box.append(scroll)

        while container.get_first_child():
            container.remove(container.get_first_child())
        container.append(box)

    async def execute(self):
        checks = {
            "64-bit system": lambda: __import__("struct").calcsize("P") == 8,
            "Python 3.8+": lambda: __import__("sys").version_info >= (3, 8),
            "CMake": lambda: self._has_command("cmake"),
            "Git": lambda: self._has_command("git"),
        }

        all_pass = True
        for name, check in checks.items():
            result = check()
            self._log(f"{'✓' if result else '✗'} {name}")
            if not result:
                all_pass = False
            await asyncio.sleep(0.2)

        self.status.set_text("✓ Check complete" if all_pass else "✗ Some checks failed")
        return all_pass

    def _has_command(self, cmd):
        try:
            subprocess.run(
                [cmd, "--version"], capture_output=True, timeout=5, check=True
            )
            return True
        except:
            return False

    def _log(self, msg):
        buf = self.log.get_buffer()
        buf.insert(buf.get_end_iter(), msg + "\n", -1)
        self.log.scroll_to_iter(buf.get_end_iter(), 0, False, 0, 0)


class DownloadStep:
    """Step 2: Download MayaFlux (skip for Arch)"""

    def build_ui(self, container):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=15)
        box.set_margin_top(20)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup(
            "<span size='16000' weight='bold'>Step 2: Download MayaFlux</span>"
        )
        title.set_halign(Gtk.Align.START)
        box.append(title)

        self.status = Gtk.Label(label="Preparing...")
        self.status.set_halign(Gtk.Align.START)
        box.append(self.status)

        self.progress = Gtk.ProgressBar()
        box.append(self.progress)

        scroll = Gtk.ScrolledWindow()
        scroll.set_hexpand(True)
        scroll.set_vexpand(True)
        scroll.set_min_content_height(200)

        self.log = Gtk.TextView()
        self.log.set_editable(False)
        self.log.add_css_class("monospace")
        scroll.set_child(self.log)
        box.append(scroll)

        while container.get_first_child():
            container.remove(container.get_first_child())
        container.append(box)

    async def execute(self):
        if self._is_arch_linux():
            self._log("✓ Arch Linux detected - MayaFlux available in AUR")
            self._log("")
            self._log("Install with: yay -S mayaflux-dev-bin")
            self._log("          or: paru -S mayaflux-dev-bin")
            self._log("")
            self._log("Skipping binary download step.")
            self.status.set_text("ℹ Arch Linux - use AUR for MayaFlux")
            return True

        root = Path.home() / "MayaFlux"

        if (root / "lib" / "libMayaFluxLib.so").exists():
            self._log("✓ Already installed")
            self.status.set_text("✓ Already installed")
            return True

        try:
            self._log("Fetching latest release...")
            release = await self._fetch_release()
            if not release:
                self._log("✗ Failed to fetch release")
                self.status.set_text("✗ Download failed")
                return False

            self._log(f"✓ Found: {release['tag']}")

            asset = self._find_asset(release)
            if not asset:
                self._log("✗ No suitable asset found for this platform")
                self.status.set_text("✗ Asset not found")
                return False

            self._log(f"Downloading {asset['name']}...")

            root.mkdir(parents=True, exist_ok=True)
            download_path = root / "mayaflux.tar.gz"

            if not await self._download(asset["browser_download_url"], download_path):
                self._log("✗ Download failed")
                self.status.set_text("✗ Download failed")
                return False

            self._log("✓ Download complete")
            self._log("Extracting...")
            with tarfile.open(download_path, "r:gz") as tar:
                tar.extractall(root)
            download_path.unlink()

            self._log("✓ Extracted successfully")
            self.status.set_text("✓ Downloaded and extracted")
            return True
        except Exception as e:
            self._log(f"✗ Error: {e}")
            self.status.set_text("✗ Failed")
            return False

    def _is_arch_linux(self) -> bool:
        """Check if running on Arch Linux"""
        try:
            result = subprocess.run(
                ["pacman", "--version"], capture_output=True, timeout=5, check=False
            )
            return result.returncode == 0
        except:
            return False

    async def _fetch_release(self) -> Optional[Dict]:
        """Fetch latest release from GitHub API"""
        url = "https://api.github.com/repos/MayaFlux/MayaFlux/releases"
        max_retries = 3

        for attempt in range(max_retries):
            try:
                req = urllib.request.Request(url)
                req.add_header("User-Agent", "Weave-Installer/1.0")

                with urllib.request.urlopen(req, timeout=10) as response:
                    if response.status != 200:
                        self._log(f"✗ GitHub API returned status {response.status}")
                        return None

                    releases = json.loads(response.read().decode())

                    if not releases:
                        self._log("✗ No releases found")
                        return None

                    release = releases[0]
                    tag = release.get("tag_name")

                    if not tag:
                        self._log("✗ No tag_name in release")
                        return None

                    self._log(f"✓ Found release: {tag}")

                    return {"tag": tag, "assets": release.get("assets", [])}

            except urllib.error.HTTPError as e:
                if e.code == 403:
                    self._log("✗ GitHub API rate limit exceeded")
                    return None
                elif e.code == 404:
                    self._log("✗ Releases not found")
                    return None
                else:
                    if attempt < max_retries - 1:
                        wait = 2**attempt
                        self._log(f"⚠ HTTP {e.code}, retrying in {wait}s...")
                        await asyncio.sleep(wait)
                    else:
                        self._log(f"✗ HTTP error {e.code}")
                    continue

            except (urllib.error.URLError, json.JSONDecodeError) as e:
                if attempt < max_retries - 1:
                    wait = 2**attempt
                    self._log(f"⚠ Error: {e}, retrying in {wait}s...")
                    await asyncio.sleep(wait)
                else:
                    self._log(f"✗ Failed: {e}")
                    continue

        return None

    def _find_asset(self, release: Dict) -> Optional[Dict]:
        """Find the appropriate asset for this platform"""
        import platform as plat

        assets = release.get("assets", [])
        if not assets:
            self._log("✗ No assets found in release")
            return None
        system = plat.system().lower()
        machine = plat.machine().lower()
        self._log(f"Looking for asset: {system}/{machine}")

        patterns = {
            "linux": ["Linux", "x86_64"],
            "darwin": ["macos", "arm64"],
            "windows": ["windows", "x86_64"],
        }

        target_patterns = patterns.get(system, [system])

        for asset in assets:
            name = asset.get("name", "")
            if all(p in name for p in target_patterns):
                self._log(f"✓ Found matching asset: {asset['name']}")
                return asset

        for asset in assets:
            if asset["name"].endswith(".tar.gz"):
                self._log(f"⚠ Using fallback asset: {asset['name']}")
                return asset

        return None

    async def _download(self, url: str, dest_path: Path) -> bool:
        """Download with progress tracking"""
        chunk_size = 8192
        downloaded = 0
        max_retries = 3

        for attempt in range(max_retries):
            try:
                req = urllib.request.Request(url)
                req.add_header("User-Agent", "Weave-Installer/1.0")

                with urllib.request.urlopen(req, timeout=30) as response:
                    total_size = response.headers.get("Content-Length")
                    if total_size:
                        total_size = int(total_size)

                    dest_path.parent.mkdir(parents=True, exist_ok=True)

                    with open(dest_path, "wb") as f:
                        while True:
                            chunk = response.read(chunk_size)
                            if not chunk:
                                break

                            f.write(chunk)
                            downloaded += len(chunk)

                            if total_size:
                                frac = downloaded / total_size
                                GLib.idle_add(
                                    lambda f=frac: self.progress.set_fraction(f)
                                )

                            await asyncio.sleep(0)

                self._log(f"✓ Downloaded successfully")
                return True

            except (urllib.error.URLError, urllib.error.HTTPError) as e:
                if attempt < max_retries - 1:
                    wait = 2**attempt
                    self._log(f"⚠ Download failed, retrying in {wait}s...")
                    dest_path.unlink(missing_ok=True)
                    await asyncio.sleep(wait)
                else:
                    self._log(f"✗ Download failed after {max_retries} attempts")
                    dest_path.unlink(missing_ok=True)
                    return False

        return False

    def _log(self, msg):
        buf = self.log.get_buffer()
        buf.insert(buf.get_end_iter(), msg + "\n", -1)
        self.log.scroll_to_iter(buf.get_end_iter(), 0, False, 0, 0)


class DependenciesStep:
    """Step 3: Install dependencies"""

    def __init__(self, script_dir):
        self.script_dir = script_dir

    def build_ui(self, container):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=15)
        box.set_margin_top(20)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup(
            "<span size='16000' weight='bold'>Step 3: Install Dependencies</span>"
        )
        title.set_halign(Gtk.Align.START)
        box.append(title)

        self.status = Gtk.Label(label="Preparing...")
        self.status.set_halign(Gtk.Align.START)
        box.append(self.status)

        scroll = Gtk.ScrolledWindow()
        scroll.set_hexpand(True)
        scroll.set_vexpand(True)
        scroll.set_min_content_height(250)

        self.log = Gtk.TextView()
        self.log.set_editable(False)
        self.log.add_css_class("monospace")
        scroll.set_child(self.log)
        box.append(scroll)

        while container.get_first_child():
            container.remove(container.get_first_child())
        container.append(box)

    async def execute(self):
        # script = Path(__file__).parent.parent.parent / "scripts" / "install_deps.sh"
        script = self.script_dir / "install_deps.sh"
        if not script.exists():
            self._log("✗ install_deps.sh not found")
            self.status.set_text("✗ Script not found")
            return False

        os.chmod(script, 0o755)

        try:
            self._log(f"Running: {script}")
            process = await asyncio.create_subprocess_exec(
                str(script),
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.STDOUT,
            )

            async for line in process.stdout:
                self._log(line.decode().rstrip())
                await asyncio.sleep(0)

            await process.wait()

            if process.returncode == 0:
                self.status.set_text("✓ Dependencies installed")
                return True
            else:
                self._log(f"✗ Exit code: {process.returncode}")
                self.status.set_text("⚠ Installation completed with warnings")
                return True
        except Exception as e:
            self._log(f"✗ Error: {e}")
            self.status.set_text("⚠ Continuing anyway")
            return True

    def _log(self, msg):
        buf = self.log.get_buffer()
        buf.insert(buf.get_end_iter(), msg + "\n", -1)
        self.log.scroll_to_iter(buf.get_end_iter(), 0, False, 0, 0)


class EnvironmentStep:
    """Step 4: Setup environment"""

    def build_ui(self, container):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=15)
        box.set_margin_top(20)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup(
            "<span size='16000' weight='bold'>Step 4: Environment Setup</span>"
        )
        title.set_halign(Gtk.Align.START)
        box.append(title)

        scroll = Gtk.ScrolledWindow()
        scroll.set_hexpand(True)
        scroll.set_vexpand(True)

        self.log = Gtk.TextView()
        self.log.set_editable(False)
        self.log.add_css_class("monospace")
        scroll.set_child(self.log)
        box.append(scroll)

        while container.get_first_child():
            container.remove(container.get_first_child())
        container.append(box)

    async def execute(self):
        root = Path.home() / "MayaFlux"

        self._log("Setting MAYAFLUX_ROOT...")
        self._set_env("MAYAFLUX_ROOT", str(root))

        self._log("Setting PATH...")
        self._set_env("PATH", f"{root}/bin:$PATH")

        self._log("Setting CMAKE_PREFIX_PATH...")
        self._set_env("CMAKE_PREFIX_PATH", str(root))

        self._log("")
        self._log("✓ Environment configured")
        self._log("")
        self._log("Restart your shell to use new variables:")
        self._log("  source ~/.bashrc")
        self._log("  # or ~/.zshrc if using zsh")

        return True

    def _set_env(self, name, value):
        configs = [Path.home() / f for f in [".bashrc", ".zshrc", ".profile"]]

        for cfg in configs:
            if cfg.exists():
                content = cfg.read_text()
                if name not in content:
                    cfg.write_text(content + f'\nexport {name}="{value}"\n')
                    self._log(f"  ✓ Updated {cfg.name}")
                else:
                    self._log(f"  ✓ Already in {cfg.name}")

    def _log(self, msg):
        buf = self.log.get_buffer()
        buf.insert(buf.get_end_iter(), msg + "\n", -1)
        self.log.scroll_to_iter(buf.get_end_iter(), 0, False, 0, 0)


class CompletionStep:
    """Step 5: Done"""

    def build_ui(self, container):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        box.set_margin_top(30)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup(
            "<span size='20000' weight='bold'>✓ Installation Complete!</span>"
        )
        title.set_halign(Gtk.Align.CENTER)
        box.append(title)

        info = Gtk.Label()
        info.set_markup("""<b>Next steps:</b>

1. Restart your terminal/shell

2. Verify installation:
   cmake --version
   weave --version

3. Create your first project:
   weave new MyProject ~/Projects/

4. Build and run:
   cd ~/Projects/MyProject
   mkdir build && cd build
   cmake .. && make
   ./MyProject""")
        info.set_wrap(True)
        info.set_halign(Gtk.Align.START)
        box.append(info)

        spacer = Gtk.Box()
        spacer.set_vexpand(True)
        box.append(spacer)

        while container.get_first_child():
            container.remove(container.get_first_child())
        container.append(box)

    async def execute(self):
        return True


class InstallationMode(Gtk.ApplicationWindow):
    """Main installation window"""

    def __init__(self, app, script_dir):
        super().__init__(application=app)
        self.script_dir = script_dir
        self.set_title("Weave - Install MayaFlux")
        self.set_default_size(700, 600)

        self.steps = [
            ConfirmationStep(),
            SystemCheckStep(),
            DownloadStep(),
            DependenciesStep(script_dir),
            EnvironmentStep(),
            CompletionStep(),
        ]
        self.current = 0

        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)

        header = Gtk.HeaderBar()
        self.step_label = Gtk.Label()
        header.set_title_widget(self.step_label)
        main_box.append(header)

        self.content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        main_box.append(self.content)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        btn_box.set_margin_top(10)
        btn_box.set_margin_bottom(10)
        btn_box.set_margin_start(30)
        btn_box.set_margin_end(30)
        btn_box.set_halign(Gtk.Align.END)

        self.back_btn = Gtk.Button(label="Back")
        self.back_btn.set_sensitive(False)
        self.back_btn.connect("clicked", self._on_back)
        btn_box.append(self.back_btn)

        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        btn_box.append(spacer)

        self.next_btn = Gtk.Button(label="Next")
        self.next_btn.add_css_class("suggested-action")
        self.next_btn.connect("clicked", self._on_next)
        btn_box.append(self.next_btn)

        main_box.append(btn_box)
        self.set_child(main_box)

        self._show_step(0)

    def _show_step(self, idx):
        self.current = idx
        self.step_label.set_text(f"Step {idx + 1} of {len(self.steps)}")

        step = self.steps[idx]
        step.build_ui(self.content)

        self.back_btn.set_sensitive(idx > 0)

        if idx == 0:
            self.next_btn.set_label("Start Installation")
        elif idx == len(self.steps) - 1:
            self.next_btn.set_label("Finish")
        else:
            self.next_btn.set_label("Next")

        self.next_btn.set_sensitive(False)
        GLib.timeout_add(100, lambda: self._run_step(step))

    def _run_step(self, step):
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            loop.run_until_complete(step.execute())
        except Exception as e:
            print(f"Step error: {e}")
        finally:
            self.next_btn.set_sensitive(True)

    def _on_next(self, btn):
        if self.current < len(self.steps) - 1:
            self._show_step(self.current + 1)
        else:
            self.close()

    def _on_back(self, btn):
        if self.current > 0:
            self._show_step(self.current - 1)
