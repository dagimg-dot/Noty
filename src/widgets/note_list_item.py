from gi.repository import Gtk, GObject  # type: ignore
from ..utils import logger


@Gtk.Template(resource_path="/com/dagimg/noty/ui/note_list_item.ui")
class NoteListItem(Gtk.Box):
    __gtype_name__ = "NoteListItem"

    __gsignals__ = {
        "delete-requested": (GObject.SignalFlags.RUN_FIRST, None, (object,)),
        "rename-requested": (GObject.SignalFlags.RUN_FIRST, None, (object,)),
    }

    note_name_label = Gtk.Template.Child()

    def __init__(self):
        super().__init__()
        self.note_object = None

    def bind_to_note(self, note_object):
        self.note_object = note_object

        if hasattr(note_object, "get_name"):
            note_name = note_object.get_name()
            note_object.connect("notify::name", self._on_note_name_changed)
        else:
            note_name = str(note_object)

        if self.note_name_label:
            self.note_name_label.set_text(note_name)
            if self.note_name_label.get_parent() is None:
                self.append(self.note_name_label)
        else:
            logger.error("Could not find note_name_label!")

    def _on_note_name_changed(self, note_object, pspec):
        if self.note_name_label:
            self.note_name_label.set_text(note_object.get_name())

    def unbind(self, note_object):
        self.note_object = None
        if self.note_name_label:
            self.note_name_label.set_text(" ")
