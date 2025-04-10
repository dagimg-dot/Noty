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
import argparse
import logging

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")

from gi.repository import Gio, Adw, GLib  # type: ignore # noqa: E402
from .windows.window import NotyWindow  # noqa: E402
from .windows.preferences import PreferencesDialog  # noqa: E402
from .windows.help_overlay import HelpOverlay  # noqa: E402
from .services.conf_manager import ConfManager  # noqa: E402
from .utils import logger  # noqa: E402
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
        self.create_action(
            "preferences", self.on_preferences_action, ["<primary>comma"]
        )
        self.create_action(
            "show-help-overlay", self.on_show_help_overlay, ["<primary>question"]
        )
        self.create_action("about", self.on_about_action)

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
        is_dev = APPLICATION_ID == "com.dagimg.dev.noty"
        about = Adw.AboutDialog(
            application_name="Noty",
            application_icon=APPLICATION_ID,
            developer_name="Dagim G. Astatkie",
            version=VERSION if not is_dev else f"{VERSION}-dev",
            developers=["Dagim G. Astatkie"],
            copyright="© 2025 Dagim G. Astatkie",
            issue_url="https://github.com/dagimg-dot/noty/issues",
            website="https://github.com/dagimg-dot/noty",
        )
        about.present(self.props.active_window)

    def on_preferences_action(self, widget, _):
        """Callback for the app.preferences action."""
        prefs_window = PreferencesDialog()
        prefs_window.present()

    def on_show_help_overlay(self, *args):
        """Show the keyboard shortcuts window when requested."""
        help_overlay = HelpOverlay()
        help_overlay.present()

    def _quit_action(self, *args):
        if self.win and self.win.file_manager.currently_open_path:
            try:
                buffer = self.win.source_buffer
                current_content = buffer.get_text(
                    buffer.get_start_iter(),
                    buffer.get_end_iter(),
                    True,
                )
                logger.info(
                    f"Saving file before quit: {self.win.file_manager.currently_open_path}"
                )
                self.win.file_manager.save_note_content(
                    self.win.file_manager.currently_open_path, current_content
                )
            except Exception as e:
                logger.error(f"Error saving file before quit: {e}")
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
        """Initialize the theme based on the user's preferences."""
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
            "system": Adw.ColorScheme.PREFER_DARK
            if is_dark
            else Adw.ColorScheme.PREFER_LIGHT,
        }

        if theme in theme_dict:
            style_manager.set_color_scheme(theme_dict[theme])
            logger.info(f"Applied theme: {theme}")


def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Noty - A simple note-taking application", add_help=True
    )

    parser.add_argument("--debug", action="store_true", help="Enable debug logging")
    parser.add_argument("--log-file", help="Path to the log file")
    parser.add_argument(
        "--log-level",
        choices=["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"],
        default="WARNING",
        help="Set the logging level",
    )

    # Parse only our known arguments, ignoring GTK ones
    args, unknown_args = parser.parse_known_args()

    if args.debug and unknown_args:
        logger.warning(f"Ignoring unknown arguments: {unknown_args}")

    return args


def main(version):
    """The application's entry point."""
    args = parse_args()

    log_level = getattr(logging, args.log_level)
    if args.debug:
        log_level = logging.DEBUG

    argv = [sys.argv[0]]

    logger.setup_logging(
        level=log_level, log_to_file=bool(args.log_file), log_file=args.log_file
    )

    logger.info(f"Starting Noty {version}")

    app = NotyApplication()

    # Pass a clean argv to avoid GTK parsing our debug flags
    return app.run(argv)
