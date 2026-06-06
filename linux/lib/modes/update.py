#!/usr/bin/env python3
"""Add community module to project - GTK4"""

import gi

gi.require_version("Gtk", "4.0")

from gi.repository import Gtk, Gio, GLib
import subprocess
import re
from pathlib import Path


class UpdateProjectMode(Gtk.ApplicationWindow):
    """Add a community module to an existing MayaFlux project"""

    def __init__(self, app, script_dir):
        super().__init__(application=app)
        self.script_dir = script_dir
        self.set_title("Weave - Add Community Module")
        self.set_default_size(600, 420)

        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)

        header = Gtk.HeaderBar()
        main_box.append(header)

        scroll = Gtk.ScrolledWindow()
        scroll.set_hexpand(True)
        scroll.set_vexpand(True)

        content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        content.set_margin_top(20)
        content.set_margin_start(30)
        content.set_margin_end(30)
        content.set_margin_bottom(20)

        title = Gtk.Label()
        title.set_markup("<span size='18000' weight='bold'>Add Community Module</span>")
        title.set_halign(Gtk.Align.START)
        content.append(title)

        # Project directory
        proj_label = Gtk.Label(label="Project Directory:")
        proj_label.set_halign(Gtk.Align.START)
        content.append(proj_label)

        proj_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self.proj_entry = Gtk.Entry()
        self.proj_entry.set_hexpand(True)
        self.proj_entry.set_placeholder_text("/path/to/your/project")
        proj_row.append(self.proj_entry)

        browse_btn = Gtk.Button(label="Browse…")
        browse_btn.connect("clicked", self._on_browse_project)
        proj_row.append(browse_btn)
        content.append(proj_row)

        # Module name
        name_label = Gtk.Label(label="Module Name:")
        name_label.set_halign(Gtk.Align.START)
        content.append(name_label)

        self.name_entry = Gtk.Entry()
        self.name_entry.set_placeholder_text("my_module")
        self.name_entry.connect("changed", self._on_name_changed)
        content.append(self.name_entry)

        self.name_error = Gtk.Label(label="")
        self.name_error.set_halign(Gtk.Align.START)
        self.name_error.add_css_class("error-label")
        content.append(self.name_error)

        # Output log
        log_label = Gtk.Label(label="Output:")
        log_label.set_halign(Gtk.Align.START)
        content.append(log_label)

        self.log_view = Gtk.TextView()
        self.log_view.set_editable(False)
        self.log_view.set_monospace(True)
        self.log_view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        self.log_buffer = self.log_view.get_buffer()

        log_scroll = Gtk.ScrolledWindow()
        log_scroll.set_min_content_height(240)
        log_scroll.set_child(self.log_view)
        content.append(log_scroll)

        scroll.set_child(content)
        main_box.append(scroll)

        # Buttons
        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        btn_box.set_margin_top(10)
        btn_box.set_margin_bottom(10)
        btn_box.set_margin_start(30)
        btn_box.set_margin_end(30)
        btn_box.set_halign(Gtk.Align.END)

        cancel = Gtk.Button(label="Cancel")
        cancel.connect("clicked", lambda b: self.close())
        btn_box.append(cancel)

        self.add_btn = Gtk.Button(label="Add Module")
        self.add_btn.add_css_class("suggested-action")
        self.add_btn.connect("clicked", self._on_add)
        btn_box.append(self.add_btn)

        main_box.append(btn_box)
        self.set_child(main_box)

    def _on_browse_project(self, btn):
        dialog = Gtk.FileChooserDialog(
            title="Select Project Directory",
            action=Gtk.FileChooserAction.SELECT_FOLDER,
            transient_for=self,
        )
        dialog.add_buttons(
            "_Cancel", Gtk.ResponseType.CANCEL, "_Open", Gtk.ResponseType.OK
        )

        current = self.proj_entry.get_text()
        if Path(current).is_dir():
            dialog.set_current_folder(Gio.File.new_for_path(current))

        def on_response(dialog, response_id):
            if response_id == Gtk.ResponseType.OK:
                f = dialog.get_file()
                if f:
                    self.proj_entry.set_text(f.get_path())
            dialog.close()

        dialog.connect("response", on_response)
        dialog.present()

    def _on_name_changed(self, entry):
        name = entry.get_text()
        if name and not re.match(r"^[a-z][a-z0-9_]*$", name):
            self.name_error.set_text(
                "Must be snake_case: lowercase letters, digits, underscores, no leading digit"
            )
        else:
            self.name_error.set_text("")

    def _append_log(self, text):
        end = self.log_buffer.get_end_iter()
        self.log_buffer.insert(end, text + "\n")
        self.log_view.scroll_to_iter(self.log_buffer.get_end_iter(), 0, False, 0, 0)

    def _on_add(self, btn):
        proj = self.proj_entry.get_text().strip()
        name = self.name_entry.get_text().strip()

        if not proj or not Path(proj).is_dir():
            self._show_error("Project directory does not exist.")
            return
        if not Path(proj, "CMakeLists.txt").exists():
            self._show_error("Not a MayaFlux project (no CMakeLists.txt found).")
            return
        if not name:
            self._show_error("Module name is required.")
            return
        if not re.match(r"^[a-z][a-z0-9_]*$", name):
            self._show_error("Module name must be snake_case.")
            return

        self.add_btn.set_sensitive(False)
        self.add_btn.set_label("Adding…")
        self.log_buffer.set_text("")

        script = Path(self.script_dir) / "create_project.sh"

        def run():
            try:
                result = subprocess.run(
                    ["bash", str(script), "update", proj, name],
                    capture_output=True,
                    text=True,
                    timeout=120,
                )
                GLib.idle_add(self._on_done, result)
            except subprocess.TimeoutExpired:
                GLib.idle_add(self._on_error, "Timed out waiting for git clone.")
            except Exception as e:
                GLib.idle_add(self._on_error, str(e))

        import threading

        threading.Thread(target=run, daemon=True).start()

    def _on_done(self, result):
        output = (result.stdout or "") + (result.stderr or "")
        for line in output.splitlines():
            self._append_log(line)

        if result.returncode == 0:
            dialog = Gtk.AlertDialog(
                message="Module Added",
                detail=f"Rebuild your project to include it.",
            )
            dialog.choose(self, None, self._on_dismissed, None)
        else:
            self.add_btn.set_sensitive(True)
            self.add_btn.set_label("Add Module")

        return False

    def _on_error(self, message):
        self._append_log(f"Error: {message}")
        self.add_btn.set_sensitive(True)
        self.add_btn.set_label("Add Module")
        return False

    def _on_dismissed(self, dialog, result, user_data=None):
        try:
            dialog.choose_finish(result)
        except Exception:
            pass
        self.close()

    def _show_error(self, message):
        dialog = Gtk.AlertDialog(message="Error", detail=message)
        dialog.choose(self, None, lambda d, r, u: d.choose_finish(r), None)
