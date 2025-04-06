from gi.repository import Gtk, GObject  # type: ignore


# Inherit from Gtk.Box instead of GObject.Object
@Gtk.Template(resource_path="/com/dagimg/noty/ui/note_list_item.ui")
class NoteListItem(Gtk.Box):
    __gtype_name__ = "NoteListItem"

    note_name_label = Gtk.Template.Child()

    def __init__(self):
        super().__init__()

    def bind_to_note(self, note_object):
        if not isinstance(note_object, GObject.Object) or not hasattr(
            note_object, "name"
        ):
            print("Warning: Attempting to bind to an invalid object in NoteListItem.")
            return

        if not self.note_name_label:
            print("Warning: note_name_label not found in NoteListItem.")
            return

        try:
            target, prop = self.note_name_label.get_property_binding("label")
            if target == note_object and prop.name == "name":
                self.note_name_label.unbind_property("label", note_object, "name")
        except Exception:
            pass

        try:
            note_object.bind_property(
                "name", self.note_name_label, "label", GObject.BindingFlags.SYNC_CREATE
            )
        except Exception as e:
            print(f"Error binding property 'name' in NoteListItem: {e}")

    def unbind(self, note_object):
        """Unbind properties when the item is recycled."""
        if not isinstance(note_object, GObject.Object) or not hasattr(
            note_object, "name"
        ):
            return
        if not self.note_name_label:
            return

        try:
            target, prop = self.note_name_label.get_property_binding("label")
            if target == note_object and prop.name == "name":
                self.note_name_label.unbind_property("label", note_object, "name")
        except Exception:
            pass
