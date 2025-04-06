import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")

from gi.repository import Adw, Gtk, Pango, GLib, Gdk  # type: ignore # noqa: E402
from .services.file_manager import FileManager  # noqa: E402
from .services.conf_manager import ConfManager  # noqa: E402
from .models.note import Note  # noqa: E402


@Gtk.Template(resource_path="/com/dagimg/noty/ui/window.ui")
class NotyWindow(Adw.ApplicationWindow):
    __gtype_name__ = "NotyWindow"

    search_entry = Gtk.Template.Child()
    results_list_revealer = Gtk.Template.Child()
    notes_list_view = Gtk.Template.Child()
    text_editor = Gtk.Template.Child()
    notes_list_container = Gtk.Template.Child()

    def __init__(self, **kwargs):
        super().__init__(**kwargs)

        self.confman = ConfManager()
        self.file_manager = FileManager()

        print("Setting up list view...")
        self._setup_list_view()
        self._setup_text_view()
        self._connect_signals()

        self.text_editor.set_sensitive(False)  # No file open initially

        self.file_manager.reload_notes()

        # Debug output to see if we have notes
        model = self.file_manager.get_notes_model()
        print(f"Notes model has {model.get_n_items()} items")

        # Initially show the notes list
        self.results_list_revealer.set_reveal_child(True)

        self._apply_editor_settings()

        self.connect("close-request", self.on_close_request)

    def _setup_list_view(self):
        model = self.file_manager.get_notes_model()

        self.filter_model = Gtk.FilterListModel.new(model, None)
        self.sorter = Gtk.CustomSorter.new(self._sort_notes, None)
        self.sort_model = Gtk.SortListModel.new(self.filter_model, self.sorter)

        self.selection_model = Gtk.SingleSelection.new(self.sort_model)
        self.selection_model.connect("notify::selected-item", self._on_note_selected)

        factory = Gtk.SignalListItemFactory()
        factory.connect("setup", self._on_factory_setup)
        factory.connect("bind", self._on_factory_bind)
        factory.connect("unbind", self._on_factory_unbind)

        self.notes_list_view.set_model(self.selection_model)
        self.notes_list_view.set_factory(factory)

        self.notes_list_view.connect("activate", self._on_note_activated)

        # Add key event controller to the list view for keyboard navigation
        self.list_key_controller = Gtk.EventControllerKey.new()
        self.list_key_controller.connect("key-pressed", self._on_list_key_pressed)
        self.notes_list_container.add_controller(self.list_key_controller)

    def _setup_text_view(self):
        self.source_buffer = Gtk.TextBuffer()
        self.text_editor.set_buffer(self.source_buffer)

        self.source_buffer.connect("changed", self._on_buffer_changed)
        self.text_editor.connect("notify::has-focus", self._on_editor_focus_changed)

    def _connect_signals(self):
        self.search_entry.connect("changed", self._on_search_changed)
        self.search_entry.connect("activate", self._on_search_activate)
        self.search_entry.connect("notify::has-focus", self._on_search_focus_changed)
        self.file_manager.connect("note_changed", self._on_external_note_change)
        self.confman.connect("dark_mode_changed", self._apply_editor_settings)
        self.confman.connect("editor_color_scheme_changed", self._apply_editor_settings)
        self.confman.connect("font_size_changed", self._apply_editor_settings)
        self.confman.connect(
            "markdown_syntax_highlighting_changed", self._apply_editor_settings
        )
        self.confman.connect("sorting_method_changed", self._on_sorting_method_changed)

    def _on_search_focus_changed(self, widget, pspec):
        has_focus = widget.has_focus()
        if has_focus:
            self.results_list_revealer.set_reveal_child(True)

    def _on_list_key_pressed(self, controller, keyval, keycode, state):
        if keyval == Gdk.KEY_Escape:
            self.results_list_revealer.set_reveal_child(False)
            self.search_entry.grab_focus()
            return True
        return False

    # --- ListView Factory Callbacks ---

    def _on_factory_setup(self, factory, list_item):
        widget = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        widget.set_margin_top(4)
        widget.set_margin_bottom(4)
        widget.set_margin_start(8)
        widget.set_margin_end(8)

        label = Gtk.Label()
        label.set_halign(Gtk.Align.START)
        label.set_hexpand(True)
        label.set_ellipsize(Pango.EllipsizeMode.END)
        widget.append(label)

        widget.label = label

        list_item.set_child(widget)
        print("Factory Setup (Widget created)")  # Debug

    def _on_factory_bind(self, factory, list_item):
        widget = list_item.get_child()
        note_object = list_item.get_item()

        if note_object and widget and hasattr(widget, "label"):
            widget.label.set_text(note_object.get_name())
            print(f"Factory Bind: {note_object.get_name()}")  # Debug
        else:
            print("Factory Bind: Item/Widget type mismatch or missing")  # Debug

    def _on_factory_unbind(self, factory, list_item):
        widget = list_item.get_child()
        note_object = list_item.get_item()

        if note_object and widget and hasattr(widget, "label"):
            widget.label.set_text("")
            print(f"Factory Unbind: {note_object.get_name()}")  # Debug
        else:
            print("Factory Unbind: Item/Widget type mismatch or missing")  # Debug

    # --- Signal Handlers ---

    def _on_note_selected(self, selection_model, *args):
        selected_note = selection_model.get_selected_item()
        if isinstance(selected_note, Note):
            print(f"Note Selected: {selected_note.get_name()}")  # Debug
            activate_on_select = self.confman.conf.get("activate_row_on_select", False)
            if activate_on_select:
                self._load_note_into_editor(selected_note)
                self.results_list_revealer.set_reveal_child(False)
        else:
            print("Selection Cleared or Invalid")  # Debug

    def _on_note_activated(self, list_view, position):
        model = list_view.get_model()
        note_object = model.get_item(position)
        if isinstance(note_object, Note):
            print(f"Note Activated: {note_object.get_name()}")  # Debug
            self._load_note_into_editor(note_object)

            self.results_list_revealer.set_reveal_child(False)

            self.text_editor.grab_focus()

    def _load_note_into_editor(self, note_object):
        if not note_object:
            self.source_buffer.set_text("")
            self.text_editor.set_sensitive(False)
            self.file_manager.currently_open_path = None
            return

        if self.file_manager.currently_open_path:
            current_content = self.source_buffer.get_text(
                self.source_buffer.get_start_iter(),
                self.source_buffer.get_end_iter(),
                True,
            )
            self.file_manager.save_note_content(
                self.file_manager.currently_open_path, current_content
            )

        content = self.file_manager.load_note_content(note_object.get_file_path())
        self.source_buffer.handler_block_by_func(self._on_buffer_changed)
        self.source_buffer.set_text(content if content is not None else "")
        self.source_buffer.handler_unblock_by_func(self._on_buffer_changed)
        self.text_editor.set_sensitive(True)
        start_iter = self.source_buffer.get_start_iter()
        self.source_buffer.place_cursor(start_iter)

        self.search_entry.set_text(note_object.get_name())

    def _on_search_changed(self, search_entry):
        query = search_entry.get_text().lower().strip()
        print(f"Search Query: {query}")  # Debug
        if not hasattr(self, "filter_model"):
            return

        if not query:
            if self.filter_model.get_filter():
                self.filter_model.set_filter(None)
        else:
            note_filter = Gtk.CustomFilter.new(self._filter_notes, query)
            self.filter_model.set_filter(note_filter)

    def _filter_notes(self, note_object, filter_data):
        if isinstance(note_object, Note) and isinstance(filter_data, str):
            return filter_data in note_object.get_name().lower()
        return False

    def _on_search_activate(self, search_entry):
        name_to_open_or_create = search_entry.get_text().strip()
        if not name_to_open_or_create:
            return

        print(f"Search Activate: {name_to_open_or_create}")  # Debug

        target_note = None
        model = self.file_manager.get_notes_model()
        for i in range(model.get_n_items()):
            note = model.get_item(i)
            if note.get_name().lower() == name_to_open_or_create.lower():
                target_note = note
                break

        if target_note:
            self._load_note_into_editor(target_note)
            self.results_list_revealer.set_reveal_child(True)

            position = self._find_position_by_item(target_note)
            if position is not None:
                self.selection_model.set_selected(position)

            GLib.timeout_add(500, lambda: self._hide_revealer_and_return_false())

            self.text_editor.grab_focus()  # Focus editor
        else:
            print(f"Creating new note: {name_to_open_or_create}")  # Debug
            new_note = self.file_manager.create_note(name_to_open_or_create)
            if new_note:
                self._load_note_into_editor(new_note)
                self.results_list_revealer.set_reveal_child(False)
                self.text_editor.grab_focus()

    def _on_buffer_changed(self, text_buffer):
        pass  # For now, rely on focus-out

    def _on_editor_focus_changed(self, widget, param):
        if not widget.has_focus():
            print("Editor Focus Lost - Saving")  # Debug
            if self.file_manager.currently_open_path:
                content = self.source_buffer.get_text(
                    self.source_buffer.get_start_iter(),
                    self.source_buffer.get_end_iter(),
                    True,
                )
                self.file_manager.save_note_content(
                    self.file_manager.currently_open_path, content
                )
        return False  # Allow event propagation

    def _on_notes_reloaded(self, file_manager):
        print("Window notified: Notes Reloaded")
        if self.filter_model.get_filter():
            self.filter_model.get_filter().changed(Gtk.FilterChange.DIFFERENT)
        if self.sorter:
            self.sort_model.items_changed(0, 0, 0)

    def _on_external_note_change(self, file_manager, note_path):
        # TODO: Implement the InfoBar logic here,
        print(f"TODO: Show InfoBar for external change: {note_path}")
        # I can use Adw.InfoBar, set its message, make it visible,
        # and connect buttons to file_manager.load_note_content(note_path)
        # or file_manager.save_note_content(note_path, current_buffer_content, overwrite=True)

    def _sort_notes(self, note_a, note_b, *args):
        sort_method = self.confman.conf.get("sorting_method", "name")
        if isinstance(note_a, Note) and isinstance(note_b, Note):
            if sort_method == "name":
                name_a = note_a.get_name().lower()
                name_b = note_b.get_name().lower()
                if name_a < name_b:
                    return -1
                if name_a > name_b:
                    return 1
                return 0
            elif sort_method == "last_modified":
                time_a = note_a.get_last_modified()
                time_b = note_b.get_last_modified()
                if time_a > time_b:
                    return -1
                if time_a < time_b:
                    return 1
                return 0
        return 0

    def _on_sorting_method_changed(self, *args):
        print("Sorting Method Changed - Forcing Re-Sort")  # Debug
        if hasattr(self, "sort_model") and self.sort_model:
            self.sort_model.changed()

    def _apply_editor_settings(self, *args):
        print("Applying editor settings (TextView)")  # Debug

        font_size = self.confman.conf.get("font_size", 12)
        css_provider = Gtk.CssProvider()
        css = f"""
        textview {{
            font-size: {font_size}pt;
        }}
        """
        css_provider.load_from_data(css.encode())
        self.text_editor.get_style_context().add_provider(
            css_provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

    def _find_position_by_item(self, item_to_find):
        for i in range(self.sort_model.get_n_items()):
            if self.sort_model.get_item(i) == item_to_find:
                return i
        return None


    def on_close_request(self, *args):
        print("Window closing - saving files")
        if self.file_manager.currently_open_path:
            current_content = self.source_buffer.get_text(
                self.source_buffer.get_start_iter(),
                self.source_buffer.get_end_iter(),
                True, 
            )
            print(f"Saving file before exit: {self.file_manager.currently_open_path}")
            self.file_manager.save_note_content(
                self.file_manager.currently_open_path, current_content
            )
        return False 
