#!/usr/bin/env python3
"""Weave - MayaFlux installer and project creator for Linux"""

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")

from gi.repository import Gtk, Adw
import sys
from enum import Enum

from weave.modes.installation import InstallationMode
from weave.modes.project import ProjectCreationMode
from weave.ui.theme import setup_css


class Mode(Enum):
    INSTALLATION = 1
    PROJECT_CREATION = 2


class ModeSelector(Gtk.ApplicationWindow):
    """Modal to choose Install or Create Project"""

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

        project_btn = Gtk.Button(label="Create Project")
        project_btn.set_size_request(-1, 80)
        project_btn.add_css_class("suggested-action")
        project_btn.connect("clicked", self._on_project_clicked)
        box.append(project_btn)

        self.set_child(box)
        self.selected_mode = None

    def _on_install_clicked(self, btn):
        self.selected_mode = Mode.INSTALLATION
        self.close()

    def _on_project_clicked(self, btn):
        self.selected_mode = Mode.PROJECT_CREATION
        self.close()


class WeaveApp(Adw.Application):
    """Main application"""

    def __init__(self):
        super().__init__(application_id="com.mayaflux.weave")
        self.connect("activate", self._on_activate)

    def _on_activate(self, app):
        setup_css(app)

        selector = ModeSelector(self)
        selector.present()
        selector.connect("close-request", self._on_mode_selected, selector)

    def _on_mode_selected(self, window, selector):
        if selector.selected_mode == Mode.INSTALLATION:
            main_window = InstallationMode(self)
        elif selector.selected_mode == Mode.PROJECT_CREATION:
            main_window = ProjectCreationMode(self)
        else:
            self.quit()
            return

        main_window.present()
        return False


def main():
    app = WeaveApp()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
