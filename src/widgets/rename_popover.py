from gi.repository import Gtk, GObject  # type: ignore
from ..services.file_manager import FileManager
from ..services.conf_manager import ConfManager
from ..utils import logger
import os


@Gtk.Template(resource_path="/com/dagimg/noty/ui/rename_popover.ui")
class RenamePopover(Gtk.Popover):
    __gtype_name__ = "RenamePopover"

    __gsignals__ = {
        "rename-success": (
            GObject.SignalFlags.RUN_FIRST,
            None,
            (str,),
        ),
    }

    rename_entry = Gtk.Template.Child()
    rename_error_revealer = Gtk.Template.Child()
    rename_error_label = Gtk.Template.Child()
    rename_confirm_btn = Gtk.Template.Child()
    rename_cancel_btn = Gtk.Template.Child()

    def __init__(self):
        super().__init__()
        self.file_manager = FileManager()
        self.confman = ConfManager()

        self.current_note = None

        self.rename_entry.connect("changed", self._on_rename_entry_changed)
        self.rename_entry.connect("activate", self._on_rename_done)
        self.rename_confirm_btn.connect("clicked", self._on_rename_done)
        self.rename_cancel_btn.connect("clicked", self._on_rename_canceled)

        self.set_position(Gtk.PositionType.BOTTOM)
        self.set_autohide(True)

    def set_note(self, note_object):
        self.current_note = note_object
        if note_object:
            self.rename_entry.set_text(note_object.get_name())

    def _on_rename_entry_changed(self, entry):
        self.rename_error_revealer.set_reveal_child(False)

    def _on_rename_canceled(self, button):
        self.popdown()
        self.unparent()

    def _on_rename_done(self, widget):
        if not self.current_note:
            self.popdown()
            self.unparent()
            return

        new_name = self.rename_entry.get_text().strip()
        if not new_name:
            return

        use_md = self.confman.conf.get("use_file_extension", False)
        extension = ".md" if use_md else ""

        note_dir = os.path.dirname(self.current_note.get_file_path())
        new_file_path = os.path.join(note_dir, f"{new_name}{extension}")

        if (
            os.path.isfile(new_file_path)
            and new_file_path != self.current_note.get_file_path()
        ):
            self.rename_error_revealer.set_reveal_child(True)
            return

        success = self.file_manager.rename_note_by_path(
            self.current_note.get_file_path(), new_name
        )

        if success:
            logger.info(f"Renamed note to {new_name}")
            self.current_note.update_after_rename(new_file_path)
            self.emit("rename-success", new_name)
            self.popdown()
            self.unparent()
        else:
            self.rename_error_label.set_text("Failed to rename note")
            self.rename_error_revealer.set_reveal_child(True)
