#!/usr/bin/env python3
"""Community module creation mode - GTK4"""

import gi

gi.require_version("Gtk", "4.0")

from gi.repository import Gtk, Gio, GLib
import subprocess
import re
from pathlib import Path


class CommunityModuleMode(Gtk.ApplicationWindow):
    """Create a new MayaFlux community module"""

    def __init__(self, app, template_dir, script_dir):
        super().__init__(application=app)
        self.template_dir = template_dir
        self.script_dir = script_dir
        self.set_title("Weave - Create Community Module")
        self.set_default_size(600, 500)

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
        title.set_markup("<span size='18000' weight='bold'>New Community Module</span>")
        title.set_halign(Gtk.Align.START)
        content.append(title)

        # Module name
        name_label = Gtk.Label(label="Module Name (snake_case):")
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

        # Description
        desc_label = Gtk.Label(label="Description:")
        desc_label.set_halign(Gtk.Align.START)
        content.append(desc_label)

        self.desc_entry = Gtk.Entry()
        self.desc_entry.set_placeholder_text(
            "A short description of what this module does"
        )
        content.append(self.desc_entry)

        # Min version
        ver_label = Gtk.Label(label="Minimum MayaFlux Version:")
        ver_label.set_halign(Gtk.Align.START)
        content.append(ver_label)

        self.ver_entry = Gtk.Entry()
        self.ver_entry.set_text("0.4.0")
        content.append(self.ver_entry)

        # Needs Lila
        self.lila_check = Gtk.CheckButton(label="Requires Lila (live coding)")
        content.append(self.lila_check)

        # Destination
        dest_label = Gtk.Label(label="Destination:")
        dest_label.set_halign(Gtk.Align.START)
        content.append(dest_label)

        dest_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.dest_entry = Gtk.Entry()
        self.dest_entry.set_text(str(Path.home() / "Projects"))
        self.dest_entry.set_hexpand(True)
        self.dest_entry.connect("changed", self._update_preview)
        dest_box.append(self.dest_entry)

        browse_btn = Gtk.Button(label="Browse...")
        browse_btn.set_size_request(100, -1)
        browse_btn.connect("clicked", self._on_browse)
        dest_box.append(browse_btn)
        content.append(dest_box)

        # Preview
        preview_label = Gtk.Label(label="Module will be created at:")
        preview_label.set_halign(Gtk.Align.START)
        preview_label.add_css_class("dim-label")
        content.append(preview_label)

        self.preview = Gtk.Label()
        self.preview.set_markup(f"<tt>{Path.home()}/Projects/my_module</tt>")
        self.preview.set_halign(Gtk.Align.START)
        self.preview.set_selectable(True)
        content.append(self.preview)

        spacer = Gtk.Box()
        spacer.set_vexpand(True)
        content.append(spacer)

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

        self.create_btn = Gtk.Button(label="Create Module")
        self.create_btn.add_css_class("suggested-action")
        self.create_btn.connect("clicked", self._on_create)
        btn_box.append(self.create_btn)

        main_box.append(btn_box)
        self.set_child(main_box)

    def _on_name_changed(self, entry):
        self._update_preview(entry)
        name = entry.get_text()
        if name and not re.match(r"^[a-z][a-z0-9_]*$", name):
            self.name_error.set_text(
                "Must be snake_case: lowercase letters, digits, underscores, no leading digit"
            )
        else:
            self.name_error.set_text("")

    def _update_preview(self, widget):
        name = self.name_entry.get_text() or "my_module"
        dest = self.dest_entry.get_text() or str(Path.home())
        self.preview.set_markup(f"<tt>{dest}/{name}</tt>")

    def _on_browse(self, btn):
        dialog = Gtk.FileChooserDialog(
            title="Select Destination",
            action=Gtk.FileChooserAction.SELECT_FOLDER,
            transient_for=self,
        )
        dialog.add_buttons(
            "_Cancel",
            Gtk.ResponseType.CANCEL,
            "_Open",
            Gtk.ResponseType.OK,
        )
        current = self.dest_entry.get_text()
        if Path(current).is_dir():
            dialog.set_current_folder(Gio.File.new_for_path(current))

        def on_response(dialog, response_id):
            if response_id == Gtk.ResponseType.OK:
                f = dialog.get_file()
                if f:
                    self.dest_entry.set_text(f.get_path())
            dialog.close()

        dialog.connect("response", on_response)
        dialog.present()

    def _on_create(self, btn):
        name = self.name_entry.get_text().strip()
        dest = self.dest_entry.get_text().strip()
        desc = self.desc_entry.get_text().strip()
        min_ver = self.ver_entry.get_text().strip()
        needs_lila = self.lila_check.get_active()

        if not name:
            self._show_error("Module name is required")
            return
        if not re.match(r"^[a-z][a-z0-9_]*$", name):
            self._show_error("Module name must be snake_case")
            return
        if not dest or not Path(dest).is_dir():
            self._show_error(f"Destination directory does not exist:\n{dest}")
            return

        self.create_btn.set_sensitive(False)
        self.create_btn.set_label("Creating...")

        GLib.timeout_add(100, self._create_async, name, dest, desc, min_ver, needs_lila)

    def _create_async(self, name, dest, desc, min_ver, needs_lila):
        try:
            import os

            env = os.environ.copy()
            env["WEAVE_TEMPLATE_DIR"] = str(self.template_dir)

            script_path = Path(self.script_dir) / "create_project.sh"
            cmd = [str(script_path), "community", name, dest]

            result = subprocess.run(
                cmd, capture_output=True, text=True, timeout=30, env=env
            )

            if result.returncode != 0:
                self._show_error(result.stderr or result.stdout or "Unknown error")
                self.create_btn.set_sensitive(True)
                self.create_btn.set_label("Create Module")
                return

            # Patch community.json with GUI-supplied values
            import json

            module_dir = Path(dest) / name
            cj_path = module_dir / "community.json"
            with open(cj_path) as f:
                data = json.load(f)
            if desc:
                data["description"] = desc
            if min_ver:
                data["min_version"] = min_ver
            data["needs_lila"] = needs_lila
            with open(cj_path, "w") as f:
                json.dump(data, f, indent=2)
                f.write("\n")

            dialog = Gtk.AlertDialog(
                message="Community Module Created",
                detail=f"Location:\n{module_dir}",
            )
            dialog.choose(self, None, self._on_done, None)

        except subprocess.TimeoutExpired:
            self._show_error("Timed out")
            self.create_btn.set_sensitive(True)
            self.create_btn.set_label("Create Module")
        except Exception as e:
            self._show_error(str(e))
            self.create_btn.set_sensitive(True)
            self.create_btn.set_label("Create Module")

        return False

    def _on_done(self, dialog, result, user_data):
        try:
            dialog.choose_finish(result)
        except Exception:
            pass
        self.close()

    def _show_error(self, message):
        dialog = Gtk.AlertDialog(message="Error", detail=message)
        dialog.choose(self, None, lambda d, r, u: d.choose_finish(r), None)
