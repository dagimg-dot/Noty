from gi.repository import Gtk  # type: ignore


# Inherit from Gtk.Box instead of GObject.Object
@Gtk.Template(resource_path="/com/dagimg/noty/ui/note_list_item.ui")
class NoteListItem(Gtk.Box):
    __gtype_name__ = "NoteListItem"

    note_name_label = Gtk.Template.Child()

    def __init__(self):
        super().__init__()

    def bind_to_note(self, note_object):
        if hasattr(note_object, "get_name"):
            note_name = note_object.get_name()
        else:
            note_name = str(note_object)

        if self.note_name_label:
            self.note_name_label.set_text(note_name)
            if self.note_name_label.get_parent() is None:
                self.append(self.note_name_label)
        else:
            print("ERROR: Could not find note_name_label!")

    def unbind(self, note_object):
        if self.note_name_label:
            self.note_name_label.set_text(" ")
