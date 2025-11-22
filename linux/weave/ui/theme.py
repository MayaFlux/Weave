#!/usr/bin/env python3
"""GTK4 dark theme setup"""

from pathlib import Path
from gi.repository import Gtk, Gdk


def setup_css(app):
    """Load and apply dark theme"""
    css_file = Path(__file__).parent / "dark.css"

    if not css_file.exists():
        return

    css_provider = Gtk.CssProvider()
    css_provider.load_from_path(str(css_file))

    Gtk.StyleContext.add_provider_for_display(
        Gdk.Display.get_default(), css_provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
    )
