#!/usr/bin/env python3
"""Project creation mode - GTK4 compatible"""

import gi

gi.require_version("Gtk", "4.0")

from gi.repository import Gtk, Gio, GLib
import subprocess
import os
import sys
from pathlib import Path


class ProjectCreationMode(Gtk.ApplicationWindow):
    """Create new MayaFlux project"""

    def __init__(self, app, template_dir, script_dir):
        super().__init__(application=app)
        self.template_dir = template_dir
        self.script_dir = script_dir
        self.set_title("Weave - Create Project")
        self.set_default_size(600, 500)

        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)

        header = Gtk.HeaderBar()
        main_box.append(header)

        scroll = Gtk.ScrolledWindow()
        scroll.set_hexpand(True)
        scroll.set_vexpand(True)

        content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        content.set_margin_top(20)
        content.set_margin_start(30)
        content.set_margin_end(30)
        content.set_margin_bottom(20)

        title = Gtk.Label()
        title.set_markup("<span size='18000' weight='bold'>New Project</span>")
        title.set_halign(Gtk.Align.START)
        content.append(title)

        name_label = Gtk.Label(label="Project Name:")
        name_label.set_halign(Gtk.Align.START)
        content.append(name_label)

        self.name_entry = Gtk.Entry()
        self.name_entry.set_placeholder_text("MyProject")
        self.name_entry.set_text("MyProject")
        self.name_entry.connect("changed", self._update_preview)
        content.append(self.name_entry)

        loc_label = Gtk.Label(label="Location:")
        loc_label.set_halign(Gtk.Align.START)
        content.append(loc_label)

        loc_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        loc_box.set_homogeneous(False)

        self.loc_entry = Gtk.Entry()
        self.loc_entry.set_text(str(Path.home() / "Projects"))
        self.loc_entry.set_hexpand(True)
        self.loc_entry.connect("changed", self._update_preview)
        loc_box.append(self.loc_entry)

        browse_btn = Gtk.Button(label="Browse...")
        browse_btn.set_size_request(100, -1)
        browse_btn.connect("clicked", self._on_browse)
        loc_box.append(browse_btn)

        content.append(loc_box)

        self.lila_check = Gtk.CheckButton(label="Enable Lila (live coding)")
        content.append(self.lila_check)

        preview_label = Gtk.Label(label="Project will be created at:")
        preview_label.set_halign(Gtk.Align.START)
        preview_label.add_css_class("dim-label")
        content.append(preview_label)

        self.preview = Gtk.Label()
        self.preview.set_markup(f"<tt>{Path.home()}/Projects/MyProject</tt>")
        self.preview.set_halign(Gtk.Align.START)
        self.preview.set_selectable(True)
        content.append(self.preview)

        spacer = Gtk.Box()
        spacer.set_vexpand(True)
        content.append(spacer)

        scroll.set_child(content)
        main_box.append(scroll)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        btn_box.set_margin_top(10)
        btn_box.set_margin_bottom(10)
        btn_box.set_margin_start(30)
        btn_box.set_margin_end(30)
        btn_box.set_halign(Gtk.Align.END)

        cancel = Gtk.Button(label="Cancel")
        cancel.connect("clicked", lambda b: self.close())
        btn_box.append(cancel)

        self.create_btn = Gtk.Button(label="Create Project")
        self.create_btn.add_css_class("suggested-action")
        self.create_btn.connect("clicked", self._on_create)
        btn_box.append(self.create_btn)

        main_box.append(btn_box)
        self.set_child(main_box)

    def _on_browse(self, btn):
        """Open folder chooser dialog"""
        dialog = Gtk.FileChooserDialog(
            title="Select Project Location",
            action=Gtk.FileChooserAction.SELECT_FOLDER,
            transient_for=self,
        )

        dialog.add_buttons(
            "_Cancel",
            Gtk.ResponseType.CANCEL,
            "_Open",
            Gtk.ResponseType.OK,
        )

        current_path = self.loc_entry.get_text()
        if Path(current_path).is_dir():
            dialog.set_current_folder(Gio.File.new_for_path(current_path))

        def on_response(dialog, response_id):
            if response_id == Gtk.ResponseType.OK:
                file = dialog.get_file()
                if file:
                    self.loc_entry.set_text(file.get_path())
            dialog.close()

        dialog.connect("response", on_response)
        dialog.present()

    def _update_preview(self, widget):
        """Update preview path as user types"""
        name = self.name_entry.get_text() or "MyProject"
        loc = self.loc_entry.get_text() or str(Path.home())

        safe_name = "".join(c if c.isalnum() or c in "-_" else "_" for c in name)

        preview_path = f"{loc}/{safe_name}"
        self.preview.set_markup(f"<tt>{preview_path}</tt>")

    def _on_create(self, btn):
        """Create the project"""
        name = self.name_entry.get_text().strip()
        loc = self.loc_entry.get_text().strip()

        if not name:
            self._show_error("Project name cannot be empty")
            return

        if not loc:
            self._show_error("Please select a location")
            return

        if not Path(loc).is_dir():
            self._show_error(f"Directory does not exist:\n{loc}")
            return

        project_dir = Path(loc) / name
        if project_dir.exists():
            self._show_error(f"Project already exists:\n{project_dir}")
            return

        self.create_btn.set_sensitive(False)
        original_label = self.create_btn.get_label()
        self.create_btn.set_label("Creating...")

        name_captured = name
        loc_captured = loc
        label_captured = original_label

        GLib.timeout_add(
            100,
            lambda n=name_captured,
            l=loc_captured,
            lbl=label_captured: self._create_project_async(n, l, lbl),
        )

    def _create_project_async(self, name, loc, original_label):
        """Create project asynchronously"""
        try:
            loc = str(Path(loc).expanduser().resolve())
            env = os.environ.copy()
            env["WEAVE_TEMPLATE_DIR"] = str(self.template_dir)
            script_path = self.script_dir / "create_project.sh"

            cmd = [str(script_path), "new", name, loc]
            if self.lila_check.get_active():
                cmd.append("--with-lila")

            result = subprocess.run(
                cmd, capture_output=True, text=True, timeout=30, env=env
            )

            if result.returncode == 0:
                project_path = Path(loc) / name
                dialog = Gtk.AlertDialog(
                    message="Project Created Successfully!",
                    detail=f"Location:\n{project_path}\n\nNext steps:\ncd {project_path}\nmkdir build && cd build\ncmake .. && make",
                )
                dialog.choose(self, None, self._on_creation_complete, None)
            else:
                error_msg = result.stderr or result.stdout or "Unknown error"
                self._show_error(f"Failed to create project:\n{error_msg}")
                self.create_btn.set_sensitive(True)
                self.create_btn.set_label(original_label)
        except subprocess.TimeoutExpired:
            self._show_error("Project creation timed out")
            self.create_btn.set_sensitive(True)
            self.create_btn.set_label(original_label)
        except Exception as e:
            self._show_error(f"Error:\n{str(e)}")
            self.create_btn.set_sensitive(True)
            self.create_btn.set_label(original_label)

    def _on_creation_complete(self, dialog, result, user_data):
        """Handle project creation completion"""
        try:
            dialog.choose_finish(result)
            self.close()
        except Exception as e:
            print(f"Error closing dialog: {e}")

    def _show_error(self, message):
        """Show error dialog"""
        dialog = Gtk.AlertDialog(message="Error", detail=message)
        dialog.choose(self, None, lambda d, r, u: d.choose_finish(r), None)
