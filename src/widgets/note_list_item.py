from gi.repository import Gtk, Gdk, Gio  # type: ignore
from ..utils import logger
from .rename_popover import RenamePopover


@Gtk.Template(resource_path="/com/dagimg/noty/ui/note_list_item.ui")
class NoteListItem(Gtk.Box):
    __gtype_name__ = "NoteListItem"

    note_name_label = Gtk.Template.Child()

    def __init__(self):
        super().__init__()
        self.note_object = None

        self._setup_click_gesture()

        self.rename_popover = RenamePopover()
        self.rename_popover.set_parent(self)

    def _setup_click_gesture(self):
        self.click_gesture = Gtk.GestureClick.new()
        self.click_gesture.set_button(Gdk.BUTTON_SECONDARY)
        self.click_gesture.connect("pressed", self._on_right_click)
        self.add_controller(self.click_gesture)

    def _on_right_click(self, gesture, n_press, x, y):
        popover = Gtk.PopoverMenu.new_from_model(self._create_context_menu())
        popover.set_parent(self)
        popover.set_position(Gtk.PositionType.BOTTOM)

        rect = Gdk.Rectangle()
        rect.x = x
        rect.y = y
        rect.width = 1
        rect.height = 1
        popover.set_pointing_to(rect)

        popover.popup()

    def _create_context_menu(self):
        menu = Gio.Menu.new()

        rename_action = Gio.MenuItem.new("Rename", "item.rename")
        menu.append_item(rename_action)

        return menu

    def _show_rename_popover(self):
        if self.note_object:
            self.rename_popover.set_note(self.note_object)
            self.rename_popover.popup()

    def bind_to_note(self, note_object):
        self.note_object = note_object

        if hasattr(note_object, "get_name"):
            note_name = note_object.get_name()
        else:
            note_name = str(note_object)

        if self.note_name_label:
            self.note_name_label.set_text(note_name)
            if self.note_name_label.get_parent() is None:
                self.append(self.note_name_label)
        else:
            logger.error("Could not find note_name_label!")

    def unbind(self, note_object):
        self.note_object = None
        if self.note_name_label:
            self.note_name_label.set_text(" ")
