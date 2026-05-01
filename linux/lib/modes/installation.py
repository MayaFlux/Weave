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


class ReleaseTypeStep:
    """Step 0.5: Choose Stable or Development release"""

    def build_ui(self, container):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        box.set_margin_top(30)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup(
            "<span size='18000' weight='bold'>Select Release Channel</span>"
        )
        title.set_halign(Gtk.Align.START)
        box.append(title)

        subtitle = Gtk.Label()
        subtitle.set_markup("Choose which version of MayaFlux to install:")
        subtitle.set_halign(Gtk.Align.START)
        subtitle.add_css_class("dim-label")
        box.append(subtitle)

        stable_frame = Gtk.Frame()
        stable_frame.set_margin_top(10)
        stable_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        stable_box.set_margin_top(15)
        stable_box.set_margin_bottom(15)
        stable_box.set_margin_start(15)
        stable_box.set_margin_end(15)

        self.stable_radio = Gtk.CheckButton(label="Stable Release (Recommended)")
        self.stable_radio.set_active(True)
        stable_box.append(self.stable_radio)

        stable_desc = Gtk.Label()
        stable_desc.set_markup(
            "<span size='small'>Production-ready release. Tested and stable.\nRecommended for general use and production environments.</span>"
        )
        stable_desc.set_halign(Gtk.Align.START)
        stable_desc.set_margin_start(25)
        stable_desc.add_css_class("dim-label")
        stable_box.append(stable_desc)

        stable_frame.set_child(stable_box)
        box.append(stable_frame)

        dev_frame = Gtk.Frame()
        dev_frame.set_margin_top(10)
        dev_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        dev_box.set_margin_top(15)
        dev_box.set_margin_bottom(15)
        dev_box.set_margin_start(15)
        dev_box.set_margin_end(15)

        self.dev_radio = Gtk.CheckButton(label="Development Release")
        self.dev_radio.set_group(self.stable_radio)
        dev_box.append(self.dev_radio)

        dev_desc = Gtk.Label()
        dev_desc.set_markup(
            "<span size='small'>Latest features and improvements. May contain bugs.\nFor testing and early access to new functionality.</span>"
        )
        dev_desc.set_halign(Gtk.Align.START)
        dev_desc.set_margin_start(25)
        dev_desc.add_css_class("dim-label")
        dev_box.append(dev_desc)

        dev_frame.set_child(dev_box)
        box.append(dev_frame)

        spacer = Gtk.Box()
        spacer.set_vexpand(True)
        box.append(spacer)

        while container.get_first_child():
            container.remove(container.get_first_child())
        container.append(box)

    async def execute(self):
        return True

    def get_release_type(self):
        release_type = "dev" if self.dev_radio.get_active() else "stable"
        return release_type


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


class DependenciesStep:
    """Step 3: Install dependencies"""

    def __init__(self, script_dir):
        self.script_dir = script_dir
        self.release_type = "stable"

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
        script = self.script_dir / "install_deps.sh"
        if not script.exists():
            self._log("✗ install_deps.sh not found")
            self.status.set_text("✗ Script not found")
            return False

        os.chmod(script, 0o755)

        try:
            self._log(f"[DEBUG] self.release_type = {self.release_type}")
            self._log(f"Running: {script} (release: {self.release_type})")
            process = await asyncio.create_subprocess_exec(
                str(script),
                self.release_type,
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
            ReleaseTypeStep(),
            SystemCheckStep(),
            DependenciesStep(script_dir),
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
            if self.current == 1:
                step = self.steps[1]

                if hasattr(step, "get_release_type"):
                    self.release_type = step.get_release_type()
                    self.steps[3].release_type = self.release_type
                    self.steps[4].release_type = self.release_type

            self._show_step(self.current + 1)
        else:
            self.close()

    def _on_back(self, btn):
        if self.current > 0:
            self._show_step(self.current - 1)
