import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")

from gi.repository import Gtk  # noqa: E402 # type: ignore


@Gtk.Template(resource_path="/com/dagimg/noty/gtk/help-overlay.ui")
class HelpOverlay(Gtk.ShortcutsWindow):
    __gtype_name__ = "HelpOverlay"

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
