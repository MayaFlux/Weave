#!/usr/bin/env python3
"""Weave - MayaFlux installer and project creator for Linux"""

import os
import sys
from pathlib import Path

_main_dir = Path(__file__).parent
_lib_dir = _main_dir.parent
if str(_lib_dir) not in sys.path:
    sys.path.insert(0, str(_lib_dir))

try:
    from lib.config import get_config

    cfg = get_config()

    for key, value in cfg.get_env_vars().items():
        os.environ.setdefault(key, value)

except Exception as e:
    print(f"Error loading Weave configuration: {e}", file=sys.stderr)
    sys.exit(1)

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")

from gi.repository import Gtk, Adw
from enum import Enum

from lib.modes.installation import InstallationMode
from lib.modes.project import ProjectCreationMode
from lib.modes.community import CommunityModuleMode
from lib.ui.theme import setup_css


class Mode(Enum):
    INSTALLATION = 1
    PROJECTS = 2


class ModeSelector(Gtk.ApplicationWindow):
    """Modal to choose Install or Projects"""

    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Weave - Select Mode")
        self.set_default_size(500, 300)
        self.set_modal(True)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        box.set_margin_top(30)
        box.set_margin_bottom(30)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup(
            "<span size='18000' weight='bold'>What would you like to do?</span>"
        )
        title.set_halign(Gtk.Align.START)
        box.append(title)

        install_btn = Gtk.Button(label="Install MayaFlux")
        install_btn.set_size_request(-1, 80)
        install_btn.add_css_class("suggested-action")
        install_btn.connect("clicked", self._on_install_clicked)
        box.append(install_btn)

        projects_btn = Gtk.Button(label="Projects")
        projects_btn.set_size_request(-1, 80)
        projects_btn.add_css_class("suggested-action")
        projects_btn.connect("clicked", self._on_projects_clicked)
        box.append(projects_btn)

        self.set_child(box)
        self.selected_mode = None

    def _on_install_clicked(self, btn):
        self.selected_mode = Mode.INSTALLATION
        self.close()

    def _on_projects_clicked(self, btn):
        self.selected_mode = Mode.PROJECTS
        self.close()


class ProjectsMode(Gtk.ApplicationWindow):
    """Choose between project actions"""

    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Weave - Projects")
        self.set_default_size(500, 380)
        self.set_modal(True)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        box.set_margin_top(30)
        box.set_margin_bottom(30)
        box.set_margin_start(30)
        box.set_margin_end(30)

        title = Gtk.Label()
        title.set_markup("<span size='18000' weight='bold'>Projects</span>")
        title.set_halign(Gtk.Align.START)
        box.append(title)

        create_btn = Gtk.Button(label="Create Project")
        create_btn.set_size_request(-1, 70)
        create_btn.add_css_class("suggested-action")
        create_btn.connect("clicked", self._on_create_clicked)
        box.append(create_btn)

        update_btn = Gtk.Button(label="Update Project (Community Modules)")
        update_btn.set_size_request(-1, 70)
        update_btn.add_css_class("suggested-action")
        update_btn.connect("clicked", self._on_update_clicked)
        box.append(update_btn)

        community_btn = Gtk.Button(label="Create Community Module")
        community_btn.set_size_request(-1, 70)
        community_btn.add_css_class("suggested-action")
        community_btn.connect("clicked", self._on_community_clicked)
        box.append(community_btn)

        self.set_child(box)
        self.selected_action = None

    def _on_create_clicked(self, btn):
        self.selected_action = "create"
        self.close()

    def _on_update_clicked(self, btn):
        self.selected_action = "update"
        self.close()

    def _on_community_clicked(self, btn):
        self.selected_action = "community"
        self.close()


class WeaveApp(Adw.Application):
    """Main application"""

    def __init__(self):
        super().__init__(application_id="com.mayaflux.weave")
        self.connect("activate", self._on_activate)

    def _on_activate(self, app):
        setup_css(app)

        icon_path = Path(__file__).parent.parent / "resources" / "weave.png"
        if icon_path.exists():
            Gtk.Window.set_default_icon_from_file(str(icon_path))

        selector = ModeSelector(self)
        selector.present()
        selector.connect("close-request", self._on_mode_selected, selector)

    def _on_mode_selected(self, window, selector):
        if selector.selected_mode == Mode.INSTALLATION:
            main_window = InstallationMode(self, cfg.scripts_dir)
            main_window.present()
        elif selector.selected_mode == Mode.PROJECTS:
            projects = ProjectsMode(self)
            projects.present()
            projects.connect("close-request", self._on_action_selected, projects)
        else:
            self.quit()
        return False

    def _on_action_selected(self, window, projects):
        if projects.selected_action == "create":
            main_window = ProjectCreationMode(self, cfg.templates_dir, cfg.scripts_dir)
            main_window.present()
        elif projects.selected_action == "update":
            pass  # TODO: UpdateProjectMode
        elif projects.selected_action == "community":
            main_window = CommunityModuleMode(self, cfg.templates_dir, cfg.scripts_dir)
            main_window.present()
        else:
            self.quit()
        return False


def main():
    app = WeaveApp()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
