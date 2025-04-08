# main.py
#
# Copyright 2025 Dagim G. Astatkie
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: GPL-3.0-or-later

import sys
import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")

from gi.repository import Gio, Adw, GLib  # type: ignore # noqa: E402
from .windows.window import NotyWindow  # noqa: E402
from .windows.preferences import PreferencesDialog  # noqa: E402
from .services.conf_manager import ConfManager  # noqa: E402
from . import APPLICATION_ID, VERSION  # noqa: E402


class NotyApplication(Adw.Application):
    """The main application singleton class."""

    def __init__(self):
        super().__init__(
            application_id=APPLICATION_ID, flags=Gio.ApplicationFlags.DEFAULT_FLAGS
        )
        GLib.set_application_name("Noty")
        GLib.set_prgname(APPLICATION_ID)
        self.confman = ConfManager()
        self.win = None

        self._initialize_theme()

        self.create_action("quit", self._quit_action, ["<primary>q"])
        self.create_action("about", self.on_about_action)
        self.create_action("preferences", self.on_preferences_action, ["<primary>p"])

    def do_activate(self):
        """Called when the application is activated.

        We raise the application's main window, creating it if
        necessary.
        """
        if not self.win:
            self.win = NotyWindow(application=self)

        self.win.present()

    def on_about_action(self, *args):
        """Callback for the app.about action."""
        about = Adw.AboutDialog(
            application_name="Noty",
            application_icon=APPLICATION_ID,
            developer_name="Dagim G. Astatkie",
            version=VERSION,
            developers=["Dagim G. Astatkie"],
            copyright="© 2025 Dagim G. Astatkie",
            issue_url="https://github.com/dagimg-dot/noty/issues",
        )
        about.present(self.props.active_window)

    def on_preferences_action(self, widget, _):
        """Callback for the app.preferences action."""
        print("app.preferences action activated")
        prefs_window = PreferencesDialog()
        prefs_window.present()

    def _quit_action(self, *args):
        if self.win and self.win.file_manager.currently_open_path:
            try:
                buffer = self.win.source_buffer
                current_content = buffer.get_text(
                    buffer.get_start_iter(),
                    buffer.get_end_iter(),
                    True,
                )
                print(
                    f"Saving file before quit: {self.win.file_manager.currently_open_path}"
                )
                self.win.file_manager.save_note_content(
                    self.win.file_manager.currently_open_path, current_content
                )
            except Exception as e:
                print(f"Error saving file before quit: {e}")
        self.quit()

    def create_action(self, name, callback, shortcuts=None):
        """Add an application action.

        Args:
            name: the name of the action
            callback: the function to be called when the action is
              activated
            shortcuts: an optional list of accelerators
        """
        action = Gio.SimpleAction.new(name, None)
        action.connect("activate", callback)
        self.add_action(action)
        if shortcuts:
            self.set_accels_for_action(f"app.{name}", shortcuts)

    def _initialize_theme(self):
        style_manager = Adw.StyleManager.get_default()
        theme = self.confman.conf.get("theme", "system")
        
        # Current system theme state
        is_dark = style_manager.get_color_scheme() in [
            Adw.ColorScheme.PREFER_DARK,
            Adw.ColorScheme.FORCE_DARK,
        ]
        
        # Apply the theme
        theme_dict = {
            "light": Adw.ColorScheme.FORCE_LIGHT,
            "dark": Adw.ColorScheme.FORCE_DARK,
            "system": Adw.ColorScheme.PREFER_DARK if is_dark else Adw.ColorScheme.PREFER_LIGHT,
        }
        
        if theme in theme_dict:
            style_manager.set_color_scheme(theme_dict[theme])
            print(f"Applied theme: {theme}")


def main(version):
    """The application's entry point."""
    app = NotyApplication()
    return app.run(sys.argv)
