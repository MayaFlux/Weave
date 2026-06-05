#!/usr/bin/env python3
"""Installation mode - multi-step setup with GTK4"""

import gi

gi.require_version("Gtk", "4.0")

from gi.repository import Gtk, GLib
import asyncio
import subprocess
import os
import threading
import shutil
import tempfile
import fcntl
import termios
import errno
import pty
import re
from pathlib import Path


# ==============================================================================
# Shared helper
# ==============================================================================


def _swap(container, new_child):
    """Clear container and append new_child."""
    while container.get_first_child():
        container.remove(container.get_first_child())
    container.append(new_child)


# ==============================================================================
# Step 0: Confirmation
# ==============================================================================


class ConfirmationStep:
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
        info.set_markup(
            "This will install:\n"
            "  MayaFlux framework\n"
            "  Build tools (CMake, Git, compiler)\n"
            "  Dependencies (FFmpeg, RtAudio, Vulkan SDK)\n\n"
            "<b>Requires:</b> Internet connection, ~2GB disk space"
        )
        info.set_wrap(True)
        info.set_halign(Gtk.Align.START)
        box.append(info)

        spacer = Gtk.Box()
        spacer.set_vexpand(True)
        box.append(spacer)

        _swap(container, box)

    def run(self, done_cb):
        done_cb(True)


# ==============================================================================
# Step 1: Release channel
# ==============================================================================


class ReleaseTypeStep:
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
            "<span size='small'>Production-ready release. Tested and stable.\n"
            "Recommended for general use.</span>"
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
            "<span size='small'>Latest features. May contain bugs.\n"
            "For testing and early access.</span>"
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

        _swap(container, box)

    def run(self, done_cb):
        done_cb(True)

    def get_release_type(self):
        return "dev" if self.dev_radio.get_active() else "stable"


# ==============================================================================
# Step 2: Install MayaFlux
# ==============================================================================


class DependenciesStep:
    """Install MayaFlux via native package manager using a PTY."""

    def __init__(self):
        self.release_type = "stable"
        self._password = ""
        self._done_cb = None

    def build_ui(self, container):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=15)
        box.set_margin_top(20)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup(
            "<span size='16000' weight='bold'>Step 1: Install MayaFlux</span>"
        )
        title.set_halign(Gtk.Align.START)
        box.append(title)

        self.status = Gtk.Label(label="Enter your password and click Install.")
        self.status.set_halign(Gtk.Align.START)
        box.append(self.status)

        pw_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)

        self._pw_entry = Gtk.PasswordEntry()
        self._pw_entry.set_show_peek_icon(True)
        self._pw_entry.set_property("placeholder-text", "sudo password")
        self._pw_entry.set_hexpand(True)
        self._pw_entry.connect(
            "changed", lambda e: setattr(self, "_password", e.get_text())
        )
        pw_row.append(self._pw_entry)

        self._install_btn = Gtk.Button(label="Install")
        self._install_btn.add_css_class("suggested-action")
        self._install_btn.connect("clicked", self._on_install_clicked)
        pw_row.append(self._install_btn)

        box.append(pw_row)

        scroll = Gtk.ScrolledWindow()
        scroll.set_hexpand(True)
        scroll.set_vexpand(True)
        scroll.set_min_content_height(250)
        self.log_view = Gtk.TextView()
        self.log_view.set_editable(False)
        self.log_view.add_css_class("monospace")
        scroll.set_child(self.log_view)
        box.append(scroll)

        _swap(container, box)

    _ANSI_RE = re.compile(r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\([a-zA-Z]")

    def _strip_ansi(self, text):
        return self._ANSI_RE.sub("", text)

    def run(self, done_cb):
        self._done_cb = done_cb

    def _on_install_clicked(self, btn):
        if not self._password:
            self.status.set_text("Password required.")
            return
        btn.set_sensitive(False)
        self._pw_entry.set_sensitive(False)
        threading.Thread(target=self._install_thread, daemon=True).start()

    def _install_thread(self):
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            ok = loop.run_until_complete(self._install())
        finally:
            loop.close()
        if self._done_cb:
            GLib.idle_add(self._done_cb, ok)

    async def _install(self):
        distro = self._detect_distro()
        if distro == "unknown":
            GLib.idle_add(self._log, "ERROR: Unsupported distribution.")
            GLib.idle_add(self.status.set_text, "ERROR: Unsupported distribution.")
            self._zero_password()
            return False

        GLib.idle_add(self._log, f"Detected distribution: {distro}")
        cmds = self._build_commands(distro)

        try:
            for cmd in cmds:
                GLib.idle_add(self._log, f"Running: {' '.join(cmd)}")
                ok = await self._run_pty(cmd)
                if not ok:
                    GLib.idle_add(self._log, f"ERROR: Command failed: {' '.join(cmd)}")
                    GLib.idle_add(self.status.set_text, "ERROR: Installation failed.")
                    return False
        finally:
            self._zero_password()

        GLib.idle_add(self.status.set_text, "MayaFlux installed successfully.")
        return True

    async def _run_pty(self, cmd):
        master_fd, slave_fd = pty.openpty()

        attrs = termios.tcgetattr(slave_fd)
        attrs[3] &= ~termios.ECHO
        termios.tcsetattr(slave_fd, termios.TCSANOW, attrs)

        pid = os.fork()
        if pid == 0:
            os.setsid()
            fcntl.ioctl(slave_fd, termios.TIOCSCTTY, 0)
            os.dup2(slave_fd, 0)
            os.dup2(slave_fd, 1)
            os.dup2(slave_fd, 2)
            os.close(master_fd)
            os.close(slave_fd)
            env = os.environ.copy()
            env.pop("LD_LIBRARY_PATH", None)
            env.pop("LD_PRELOAD", None)
            os.execvpe(cmd[0], cmd, env)
            os._exit(1)

        os.close(slave_fd)

        os.write(master_fd, b"y\n")
        os.write(master_fd, (self._password + "\n").encode())

        flags = fcntl.fcntl(master_fd, fcntl.F_GETFL)
        fcntl.fcntl(master_fd, fcntl.F_SETFL, flags | os.O_NONBLOCK)

        buf = b""
        loop = asyncio.get_event_loop()

        def _read_ready():
            nonlocal buf
            try:
                data = os.read(master_fd, 4096)
                buf += data
                lines = buf.split(b"\n")
                buf = lines[-1]
                for line in lines[:-1]:
                    text = line.decode(errors="replace").rstrip("\r")
                    if text:
                        GLib.idle_add(self._log, self._strip_ansi(text))
            except OSError as e:
                if e.errno not in (errno.EAGAIN, errno.EIO):
                    GLib.idle_add(self._log, f"ERROR reading PTY: {e}")
                return

            check = buf.decode(errors="replace").lower()
            if check.endswith("[y/n]:") or check.endswith("[y/n]: "):
                os.write(master_fd, b"y\n")
            elif "password" in check and check.endswith(": "):
                os.write(master_fd, (self._password + "\n").encode())

        loop.add_reader(master_fd, _read_ready)

        status = 0
        while True:
            try:
                wp, status = os.waitpid(pid, os.WNOHANG)
                if wp == pid:
                    break
            except ChildProcessError:
                break
            await asyncio.sleep(0.05)

        loop.remove_reader(master_fd)

        try:
            while True:
                data = os.read(master_fd, 4096)
                if not data:
                    break
                for line in data.split(b"\n"):
                    text = line.decode(errors="replace").rstrip("\r")
                    if text:
                        GLib.idle_add(self._log, text)
        except OSError:
            pass

        os.close(master_fd)
        return os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0

    def _detect_distro(self):
        if shutil.which("pacman"):
            return "arch"
        if shutil.which("dnf"):
            return "fedora"
        if shutil.which("apt-get"):
            return "ubuntu"
        return "unknown"

    def _build_commands(self, distro):
        if distro == "arch":
            pkg = "mayaflux-dev-bin" if self.release_type == "dev" else "mayaflux"
            if shutil.which("yay"):
                return [["yay", "-S", "--noconfirm", pkg]]
            if shutil.which("paru"):
                return [["paru", "-S", "--noconfirm", pkg]]
            build_dir = tempfile.mkdtemp()
            return [
                ["git", "clone", f"https://aur.archlinux.org/{pkg}.git", build_dir],
                ["bash", "-c", f"cd {build_dir} && makepkg -si --noconfirm"],
            ]

        if distro == "fedora":
            pkg = "mayaflux-dev" if self.release_type == "dev" else "mayaflux"
            copr_pkg = "mayaflux-dev" if self.release_type == "dev" else "mayaflux"
            return [
                ["sudo", "dnf", "copr", "enable", "-y", "ranjithshegde/spirv-cross"],
                [
                    "sudo",
                    "dnf",
                    "copr",
                    "enable",
                    "-y",
                    "ranjithshegde/asio-standalone",
                ],
                ["sudo", "dnf", "copr", "enable", "-y", f"ranjithshegde/{copr_pkg}"],
                ["sudo", "dnf", "install", "-y", pkg],
            ]

        if distro == "ubuntu":
            ppa = "mayaflux-dev" if self.release_type == "dev" else "mayaflux"
            pkg = "mayaflux-edge" if self.release_type == "dev" else "mayaflux"
            return [
                ["sudo", "add-apt-repository", "-y", f"ppa:mayaflux/{ppa}"],
                ["sudo", "apt-get", "update"],
                ["sudo", "apt-get", "install", "-y", pkg],
            ]

    def _zero_password(self):
        self._password = ""
        GLib.idle_add(self._pw_entry.set_text, "")

    def _log(self, msg):
        buf = self.log_view.get_buffer()
        buf.insert(buf.get_end_iter(), msg + "\n", -1)
        self.log_view.scroll_to_iter(buf.get_end_iter(), 0, False, 0, 0)


# ==============================================================================
# Step 3: Complete
# ==============================================================================


class CompletionStep:
    def build_ui(self, container):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        box.set_margin_top(30)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup(
            "<span size='20000' weight='bold'>Installation Complete!</span>"
        )
        title.set_halign(Gtk.Align.CENTER)
        box.append(title)

        self._status = Gtk.Label(label="Installing Weave CLI...")
        self._status.set_halign(Gtk.Align.START)
        self._status.add_css_class("dim-label")
        box.append(self._status)

        info = Gtk.Label()
        info.set_markup(
            "<b>Next steps:</b>\n\n"
            "1. Restart your terminal\n\n"
            "2. Verify installation:\n"
            "   cmake --version\n"
            "   weave --version\n\n"
            "3. Create your first project:\n"
            "   weave new MyProject ~/Projects/\n\n"
            "4. Build and run:\n"
            "   cd ~/Projects/MyProject\n"
            "   mkdir build &amp;&amp; cd build\n"
            "   cmake .. &amp;&amp; make\n"
            "   ./MyProject"
        )
        info.set_wrap(True)
        info.set_halign(Gtk.Align.START)
        box.append(info)

        spacer = Gtk.Box()
        spacer.set_vexpand(True)
        box.append(spacer)

        _swap(container, box)

    def run(self, done_cb):
        threading.Thread(target=self._deploy_cli, args=(done_cb,), daemon=True).start()

    def _deploy_cli(self, done_cb):
        try:
            weave_root = Path(os.environ.get("WEAVE_ROOT", ""))
            if not weave_root or not weave_root.exists():
                GLib.idle_add(
                    self._status.set_text,
                    "Warning: WEAVE_ROOT not set, skipping CLI install.",
                )
                GLib.idle_add(done_cb, True)
                return

            script_src = weave_root / "lib" / "scripts" / "create_project.sh"
            if not script_src.exists():
                raise FileNotFoundError(f"create_project.sh not found at {script_src}")

            templates_src = weave_root / "lib" / "templates"
            bin_dir = Path.home() / ".local" / "bin"
            share_dir = Path.home() / ".local" / "share" / "weave" / "templates"

            bin_dir.mkdir(parents=True, exist_ok=True)

            dest_launcher = bin_dir / "weave"
            shutil.copy2(script_src, dest_launcher)
            dest_launcher.chmod(0o755)

            if templates_src.exists():
                if share_dir.exists():
                    shutil.rmtree(share_dir)
                shutil.copytree(templates_src, share_dir)

            GLib.idle_add(
                self._status.set_text, "Weave CLI installed to ~/.local/bin/weave"
            )
        except Exception as e:
            print(f"[weave deploy] ERROR: {e}")
            GLib.idle_add(self._status.set_text, f"Warning: CLI install failed: {e}")

        GLib.idle_add(done_cb, True)


# ==============================================================================
# Wizard
# ==============================================================================


class InstallationMode(Gtk.ApplicationWindow):
    def __init__(self, app, script_dir):
        super().__init__(application=app)
        self.script_dir = script_dir
        self.set_title("Weave - Install MayaFlux")
        self.set_default_size(700, 600)

        self._deps_step = DependenciesStep()

        self.steps = [
            ConfirmationStep(),
            ReleaseTypeStep(),
            self._deps_step,
            CompletionStep(),
        ]
        self.current = 0

        root = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)

        header = Gtk.HeaderBar()
        self.step_label = Gtk.Label()
        header.set_title_widget(self.step_label)
        root.append(header)

        self.content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.content.set_vexpand(True)
        self.content.set_hexpand(True)
        root.append(self.content)

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

        filler = Gtk.Box()
        filler.set_hexpand(True)
        btn_box.append(filler)

        self.next_btn = Gtk.Button(label="Next")
        self.next_btn.add_css_class("suggested-action")
        self.next_btn.connect("clicked", self._on_next)
        btn_box.append(self.next_btn)

        root.append(btn_box)
        self.set_child(root)

        self._show_step(0)

    def _show_step(self, idx):
        self.current = idx
        self.step_label.set_text(f"Step {idx + 1} of {len(self.steps)}")

        if idx == 0:
            self.next_btn.set_label("Start Installation")
        elif idx == len(self.steps) - 1:
            self.next_btn.set_label("Finish")
        else:
            self.next_btn.set_label("Next")

        self.next_btn.set_sensitive(False)
        self.back_btn.set_sensitive(idx > 0)

        step = self.steps[idx]
        step.build_ui(self.content)
        child = self.content.get_first_child()
        print(f"[DEBUG] content first child after build_ui: {child}")
        print(f"[DEBUG] content visible: {self.content.get_visible()}")
        if child:
            print(f"[DEBUG] child visible: {child.get_visible()}")

        step.run(self._on_step_done)
        self.content.queue_draw()
        self.content.queue_resize()

    def _on_step_done(self, success):
        self.next_btn.set_sensitive(True)

    def _on_next(self, btn):
        if self.current < len(self.steps) - 1:
            if self.current == 1:
                release_step = self.steps[1]
                if hasattr(release_step, "get_release_type"):
                    self._deps_step.release_type = release_step.get_release_type()
            self._show_step(self.current + 1)
        else:
            self.close()

    def _on_back(self, btn):
        if self.current > 0:
            self._show_step(self.current - 1)
