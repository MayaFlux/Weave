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
import tarfile
from pathlib import Path


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
    """Step 2: Download MayaFlux"""

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
            self._log(f"Downloading {release['asset_name']}...")

            root.mkdir(parents=True, exist_ok=True)
            await self._download(release["url"], root / "mayaflux.tar.gz")

            self._log("✓ Download complete")
            self._log("Extracting...")
            with tarfile.open(root / "mayaflux.tar.gz", "r:gz") as tar:
                tar.extractall(root)
            (root / "mayaflux.tar.gz").unlink()

            self._log("✓ Extracted successfully")
            self.status.set_text("✓ Downloaded and extracted")
            return True
        except Exception as e:
            self._log(f"✗ Error: {e}")
            self.status.set_text("✗ Failed")
            return False

    async def _fetch_release(self):
        try:
            with urllib.request.urlopen(
                "https://api.github.com/repos/MayaFlux/MayaFlux/releases/latest",
                timeout=10,
            ) as r:
                data = json.loads(r.read())

            for asset in data.get("assets", []):
                if "linux" in asset["name"] and asset["name"].endswith(".tar.gz"):
                    return {
                        "tag": data["tag_name"],
                        "asset_name": asset["name"],
                        "url": asset["browser_download_url"],
                    }
            return None
        except Exception as e:
            self._log(f"✗ GitHub API error: {e}")
            return None

    async def _download(self, url, dest):
        def progress(block, size, total):
            if total > 0:
                frac = min((block * size) / total, 1.0)
                GLib.idle_add(lambda f=frac: self.progress.set_fraction(f))

        urllib.request.urlretrieve(url, dest, progress)

    def _log(self, msg):
        buf = self.log.get_buffer()
        buf.insert(buf.get_end_iter(), msg + "\n", -1)
        self.log.scroll_to_iter(buf.get_end_iter(), 0, False, 0, 0)


class DependenciesStep:
    """Step 3: Install dependencies"""

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
        script = Path(__file__).parent.parent.parent / "scripts" / "install_deps.sh"
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

    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Weave - Install MayaFlux")
        self.set_default_size(700, 600)

        self.steps = [
            ConfirmationStep(),
            SystemCheckStep(),
            DownloadStep(),
            DependenciesStep(),
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
