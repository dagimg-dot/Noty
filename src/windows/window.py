import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
gi.require_version("GtkSource", "5")

from gi.repository import Adw, Gtk, Gdk, Gio, Pango, GtkSource  # type: ignore # noqa: E402
from gettext import gettext as _  # noqa: E402
from ..services.file_manager import FileManager  # noqa: E402
from ..services.conf_manager import ConfManager  # noqa: E402
from ..services.style_scheme_manager import StyleSchemeManager  # noqa: E402
from ..models.note import Note  # noqa: E402
from ..widgets.note_list_item import NoteListItem  # noqa: E402
from ..widgets.rename_popover import RenamePopover  # noqa: E402
from ..utils import logger  # noqa: E402


@Gtk.Template(resource_path="/com/dagimg/noty/ui/window.ui")
class NotyWindow(Adw.ApplicationWindow):
    __gtype_name__ = "NotyWindow"

    search_entry: Gtk.SearchEntry = Gtk.Template.Child()
    results_list_revealer: Gtk.Revealer = Gtk.Template.Child()
    notes_list_view: Gtk.ListView = Gtk.Template.Child()
    text_editor: GtkSource.View = Gtk.Template.Child()
    notes_list_container: Gtk.Box = Gtk.Template.Child()
    toast_overlay: Adw.ToastOverlay = Gtk.Template.Child()

    def __init__(self, **kwargs):
        super().__init__(**kwargs)

        self.confman = ConfManager()
        self.file_manager = FileManager()
        self.scheme_manager = StyleSchemeManager()

        # Set initial window size from config if persistence is enabled
        if self.confman.conf["persist_window_size"]:
            width = self.confman.conf["windowsize"]["width"]
            height = self.confman.conf["windowsize"]["height"]
            self.set_default_size(width, height)

        # Create shared RenamePopover
        self.rename_popover = RenamePopover()
        self._rename_note_object = None

        # Flag to track selection method (keyboard vs mouse)
        self._selection_from_keyboard = False

        self._current_query = ""
        self._note_filter = Gtk.CustomFilter.new(self._filter_notes, None)

        logger.info("Setting up list view...")
        self._setup_list_view()
        self._setup_key_controllers()
        self._setup_text_view()
        self._connect_signals()

        self.text_editor.set_sensitive(False)  # No file open initially

        self.file_manager.reload_notes()

        # Debug output to see if we have notes
        model = self.file_manager.get_notes_model()
        logger.debug(f"Notes model has {model.get_n_items()} items")

        # Initially show the notes list
        self.results_list_revealer.set_reveal_child(True)

        self._apply_editor_settings()

        self.connect("close-request", self.on_close_request)

    def _setup_list_view(self):
        model = self.file_manager.get_notes_model()

        self.filter_model = Gtk.FilterListModel.new(model, self._note_filter)

        # Create a sorter function using CustomSorter
        self.sorter = Gtk.CustomSorter.new(self._sort_notes_func, None)
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

        # Add click controllers
        click_controller = Gtk.GestureClick.new()
        click_controller.connect("pressed", self._on_listview_click)
        self.notes_list_view.add_controller(click_controller)

        # Add right-click controller for context menu
        right_click_controller = Gtk.GestureClick.new()
        right_click_controller.set_button(Gdk.BUTTON_SECONDARY)
        right_click_controller.connect("pressed", self._on_list_right_click)
        self.notes_list_view.add_controller(right_click_controller)

        # Add actions for context menu
        self._setup_list_actions()

    def _setup_key_controllers(self):
        list_key_controller = Gtk.EventControllerKey.new()
        list_key_controller.connect("key-pressed", self._on_list_key_pressed)
        self.notes_list_view.add_controller(list_key_controller)

        search_key_controller = Gtk.EventControllerKey.new()
        search_key_controller.connect("key-pressed", self._on_search_key_pressed)
        self.search_entry.add_controller(search_key_controller)

        # Standard editor key controller (will be disabled if Vim mode is on)
        self._standard_editor_key_controller = Gtk.EventControllerKey.new()
        self._standard_editor_key_controller.connect(
            "key-pressed", self._on_editor_key_pressed
        )
        self.text_editor.add_controller(self._standard_editor_key_controller)

        # Variables to hold Vim specific controllers
        self._vim_im_context = None
        self._vim_key_controller = None

    def _setup_text_view(self):
        self.source_buffer = GtkSource.Buffer()
        self.text_editor.set_buffer(self.source_buffer)

        # Initial Vim mode setup based on config
        if self.confman.conf.get("editor_vim_mode", False):
            self._enable_vim_bindings()
        else:
            self._disable_vim_bindings()  # Ensures standard controller is active if Vim is off initially

        show_markdown_syntax = self.confman.conf.get(
            "show_markdown_syntax_highlighting", True
        )
        if show_markdown_syntax:
            language_manager = GtkSource.LanguageManager.get_default()
            markdown_language = language_manager.get_language("markdown")
            if markdown_language:
                self.source_buffer.set_language(markdown_language)
            else:
                logger.warning(
                    "Markdown language not found during setup. Syntax highlighting disabled."
                )

        # Read line number and current line highlight settings from config
        self.text_editor.set_show_line_numbers(
            self.confman.conf.get("editor_show_line_numbers", True)
        )
        self.text_editor.set_highlight_current_line(
            self.confman.conf.get("editor_highlight_current_line", True)
        )
        self.text_editor.set_tab_width(4)
        self.text_editor.set_insert_spaces_instead_of_tabs(True)
        self.text_editor.set_auto_indent(True)

        self.source_buffer.connect("changed", self._on_buffer_changed)
        self.text_editor.connect("notify::has-focus", self._on_editor_focus_changed)

    def _connect_signals(self):
        # Search Entry
        self.search_entry.connect("search-changed", self._on_search_changed)
        self.search_entry.connect("activate", self._on_search_activate)
        self.search_entry.connect("notify::has-focus", self._on_search_focus_changed)

        # Text Editor
        self.text_editor.connect("notify::has-focus", self._on_editor_focus_changed)

        # File Manager
        self.file_manager.connect("note_changed", self._on_external_note_change)
        self.file_manager.connect("note_reloaded", self._on_notes_reloaded)

        # Conf Manager
        self.confman.connect("theme_changed", self._apply_editor_settings)
        self.confman.connect("editor_color_scheme_changed", self._apply_editor_settings)
        self.confman.connect("font_size_changed", self._apply_editor_settings)
        self.confman.connect(
            "markdown_syntax_highlighting_changed", self._apply_editor_settings
        )
        self.confman.connect("custom_font_changed", self._apply_editor_settings)
        self.confman.connect("sorting_method_changed", self._on_sorting_method_changed)
        self.confman.connect(
            "persist_window_size_changed", self._on_persist_window_size_changed
        )
        self.confman.connect(
            "editor_show_line_numbers_changed", self._apply_editor_settings
        )
        self.confman.connect(
            "editor_highlight_current_line_changed", self._apply_editor_settings
        )
        self.confman.connect("vim_mode_changed", self._on_vim_mode_setting_changed)

        # Rename Popover
        self.rename_popover.connect("rename-success", self._on_rename_success)

    def get_content(self):
        return self.source_buffer.get_text(
            self.source_buffer.get_start_iter(),
            self.source_buffer.get_end_iter(),
            True,
        )

    def get_list_item(self):
        return self.notes_list_view.get_focus_child().get_first_child()

    def save_content(self):
        if self.file_manager.currently_open_path:
            self.file_manager.save_note_content(
                self.file_manager.currently_open_path, self.get_content()
            )

    def save_window_size(self, enabled=False):
        if self.confman.conf["persist_window_size"] or enabled:
            width, height = self.get_default_size()
            self.confman.conf["windowsize"]["width"] = width
            self.confman.conf["windowsize"]["height"] = height
            self.confman.save_conf()
            logger.info(f"Saving window size: {width}x{height}")

    def show_toast(self, message, timeout=3):
        toast = Adw.Toast.new(message)
        toast.set_timeout(timeout)
        self.toast_overlay.add_toast(toast)

    def _on_search_focus_changed(self, widget, pspec):
        if widget.has_focus():
            self.results_list_revealer.set_reveal_child(True)
        else:
            self.results_list_revealer.set_reveal_child(False)

    def _search_entry_focus(self):
        self.search_entry.grab_focus()
        self.search_entry.select_region(0, -1)

    def _on_listview_click(self, gesture, n_press, x, y):
        self._selection_from_keyboard = False

    def _select_first_child(self):
        self.selection_model.set_selected(0)
        first_item = self.notes_list_view.get_first_child()
        if first_item:
            first_item.grab_focus()

    def _select_last_child(self):
        n_items = self.sort_model.get_n_items()
        self.selection_model.set_selected(n_items - 1)
        last_item = self.notes_list_view.get_last_child()
        if last_item:
            last_item.grab_focus()

    def _on_list_right_click(self, gesture, n_press, x, y):
        selected_item = self.selection_model.get_selected_item()
        if not selected_item or not isinstance(selected_item, Note):
            return

        menu = Gio.Menu.new()
        menu.append(_("Rename"), "item.rename")
        menu.append(_("Delete"), "item.delete")

        popover = Gtk.PopoverMenu.new_from_model(menu)
        popover.set_parent(self.notes_list_view)

        rect = Gdk.Rectangle()
        rect.x = x
        rect.y = y
        rect.width = 1
        rect.height = 1
        popover.set_pointing_to(rect)

        popover.popup()

    def _on_list_key_pressed(self, controller, keyval, keycode, state):
        if keyval in (
            Gdk.KEY_Up,
            Gdk.KEY_Down,
            Gdk.KEY_Home,
            Gdk.KEY_End,
            Gdk.KEY_Page_Up,
            Gdk.KEY_Page_Down,
        ):
            self._selection_from_keyboard = True

        # Check if there are items in the list first
        n_items = self.sort_model.get_n_items()
        if n_items == 0:
            return False

        if keyval == Gdk.KEY_Delete or keyval == Gdk.KEY_BackSpace:
            selected_item = self.selection_model.get_selected_item()
            if selected_item and isinstance(selected_item, Note):
                self._show_delete_confirmation(selected_item)
                return True

        # Handle Ctrl+J/K/R for list navigation and rename
        ctrl_pressed = state & Gdk.ModifierType.CONTROL_MASK

        if ctrl_pressed and (
            keyval == Gdk.KEY_j or keyval == Gdk.KEY_k or keyval == Gdk.KEY_r
        ):
            self._selection_from_keyboard = True
            selected_pos = self.selection_model.get_selected()

            if keyval == Gdk.KEY_j:
                if selected_pos < n_items - 1:
                    self.selection_model.set_selected(selected_pos + 1)
                    self.notes_list_view.scroll_to(
                        selected_pos + 1, Gtk.ListScrollFlags.FOCUS, None
                    )
                else:
                    self._select_first_child()
                return True
            elif keyval == Gdk.KEY_k:
                if selected_pos > 0:
                    self.selection_model.set_selected(selected_pos - 1)
                    self.notes_list_view.scroll_to(
                        selected_pos - 1, Gtk.ListScrollFlags.FOCUS, None
                    )
                else:
                    self._select_last_child()
                return True
            elif keyval == Gdk.KEY_r:
                selected_item = self.selection_model.get_selected_item()
                if selected_item and isinstance(selected_item, Note):
                    note_list_item = self.get_list_item()
                    self._show_rename_popover(selected_item, note_list_item)
                return True

        if keyval == Gdk.KEY_Escape:
            self._search_entry_focus()
            return True
        elif keyval == Gdk.KEY_Up:
            selected_pos = self.selection_model.get_selected()
            if selected_pos == 0:
                self._select_last_child()
                return True
        elif keyval == Gdk.KEY_Down:
            selected_pos = self.selection_model.get_selected()
            if selected_pos == n_items - 1:
                self._select_first_child()
                return True

        if keyval == Gdk.KEY_F2:
            selected_item = self.selection_model.get_selected_item()
            if selected_item and isinstance(selected_item, Note):
                note_list_item = self.get_list_item()
                self._show_rename_popover(selected_item, note_list_item)
                return True

        return False

    def _on_search_key_pressed(self, controller, keyval, keycode, state):
        if keyval in (Gdk.KEY_Up, Gdk.KEY_Down):
            self._selection_from_keyboard = True

        n_items = self.sort_model.get_n_items()
        if n_items == 0:
            return False

        ctrl_pressed = state & Gdk.ModifierType.CONTROL_MASK

        if ctrl_pressed and (keyval == Gdk.KEY_j or keyval == Gdk.KEY_k):
            if keyval == Gdk.KEY_j:
                self._select_first_child()
            elif keyval == Gdk.KEY_k:
                self._select_last_child()
            return True

        if keyval == Gdk.KEY_Down:
            self._select_first_child()
            return True
        elif keyval == Gdk.KEY_Up:
            self._select_last_child()
            return True
        return False

    def _on_editor_key_pressed(self, controller, keyval, keycode, state):
        vim_mode_enabled = self.confman.conf.get("editor_vim_mode", False)
        ctrl_pressed = state & Gdk.ModifierType.CONTROL_MASK
        alt_pressed = state & Gdk.ModifierType.ALT_MASK

        # Toggle Vim mode with Alt+V
        if alt_pressed and keyval == Gdk.KEY_v:
            new_vim_state = not vim_mode_enabled
            self.confman.conf["editor_vim_mode"] = new_vim_state
            self.confman.save_conf()
            reason = "user_enabled" if new_vim_state else "user_disabled"
            self.confman.emit("vim_mode_changed", new_vim_state, reason)
            return True

        # Focus search with Alt+S (works regardless of Vim mode)
        if alt_pressed and keyval == Gdk.KEY_s:
            self._search_entry_focus()
            return True

        # Handle Escape key only when Vim mode is disabled
        if not vim_mode_enabled and keyval == Gdk.KEY_Escape:
            self._search_entry_focus()
            return True

        # Font size shortcuts work regardless of Vim mode
        if ctrl_pressed:
            if keyval == Gdk.KEY_plus or keyval == Gdk.KEY_equal:
                current_size = self.confman.conf.get("font_size", 12)
                if current_size < 32:
                    self.confman.conf["font_size"] = current_size + 1
                    self.confman.save_conf()
                    self.confman.emit("font_size_changed", current_size + 1)
                return True

            elif keyval == Gdk.KEY_minus:
                current_size = self.confman.conf.get("font_size", 12)
                if current_size > 8:
                    self.confman.conf["font_size"] = current_size - 1
                    self.confman.save_conf()
                    self.confman.emit("font_size_changed", current_size - 1)
                return True

            elif keyval == Gdk.KEY_0:
                default_size = 12
                self.confman.conf["font_size"] = default_size
                self.confman.save_conf()
                self.confman.emit("font_size_changed", default_size)
                return True

        return False

    def _enable_vim_bindings(self):
        if not self._vim_key_controller:
            logger.info("Enabling Vim bindings for text editor")

            self._vim_im_context = GtkSource.VimIMContext.new()
            self._vim_im_context.set_client_widget(self.text_editor)

            self._vim_key_controller = Gtk.EventControllerKey.new()
            self._vim_key_controller.set_im_context(self._vim_im_context)
            self._vim_key_controller.set_propagation_phase(Gtk.PropagationPhase.CAPTURE)
            self.text_editor.add_controller(self._vim_key_controller)
        else:
            logger.info("Vim bindings already enabled or controller exists")

    def _disable_vim_bindings(self):
        if self._vim_key_controller and self._vim_key_controller.get_widget():
            logger.info("Disabling Vim bindings for text editor")
            self.text_editor.remove_controller(self._vim_key_controller)
            self._vim_key_controller = None
            self._vim_im_context = None

    def _on_vim_mode_setting_changed(self, confman, is_enabled, reason="unknown"):
        logger.info(
            f"Vim mode setting changed: {'Enabled' if is_enabled else 'Disabled'} (reason: {reason})"
        )
        if is_enabled:
            self._enable_vim_bindings()
            # Only show toast for legitimate user actions
            if reason == "user_enabled":
                self.show_toast("Vim mode enabled. Use Alt+S to focus search.", 5)
        else:
            self._disable_vim_bindings()
            # Only show toast for legitimate user actions
            if reason == "user_disabled":
                self.show_toast(
                    "Vim mode disabled. Use Escape or Alt+S to focus search.", 3
                )

    # --- ListView Factory Callbacks ---

    def _on_factory_setup(self, factory, list_item):
        note_list_item = NoteListItem()
        list_item.set_child(note_list_item)
        logger.debug("Factory Setup (Widget created)")

    def _on_rename_success(self, popover, new_name):
        toast = Adw.Toast.new(f"Note renamed to '{new_name}'")
        toast.set_timeout(3)
        self.toast_overlay.add_toast(toast)

    def _on_note_delete_requested(self, note_list_item, note_object):
        self._show_delete_confirmation(note_object)

    def _on_factory_bind(self, factory, list_item):
        note_object = list_item.get_item()
        if isinstance(note_object, Note):
            list_item.get_child().bind_to_note(note_object)
            logger.debug(f"Factory Bind: {note_object.get_name()}")
        else:
            logger.warning("Factory Bind: Item/Widget type mismatch or missing")

    def _on_factory_unbind(self, factory, list_item):
        note_object = list_item.get_item()
        if isinstance(note_object, Note):
            list_item.get_child().unbind(note_object)
            logger.debug(f"Factory Unbind: {note_object.get_name()}")
        else:
            logger.warning("Factory Unbind: Item/Widget type mismatch or missing")

    # --- Signal Handlers ---

    def _on_note_selected(self, selection_model, *args):
        selected_item = selection_model.get_selected_item()
        if selected_item is None:
            logger.debug("Selection Cleared or Invalid")
            return

        activate_on_select = self.confman.conf.get("activate_row_on_select", False)

        # Only apply activate_on_select for mouse selection, not keyboard navigation and not on active search
        if (
            activate_on_select
            and not self._selection_from_keyboard
            and not self.search_entry.get_text().strip()
        ):
            self._load_note_into_editor(selected_item)
            self.text_editor.grab_focus()

        self._selection_from_keyboard = False

    def _on_note_activated(self, list_view, position):
        model = list_view.get_model()
        note_object = model.get_item(position)
        if isinstance(note_object, Note):
            self._load_note_into_editor(note_object)
            self.text_editor.grab_focus()

    def _load_note_into_editor(self, note_object):
        if not note_object:
            self.source_buffer.set_text("")
            self.text_editor.set_sensitive(False)
            self.file_manager.currently_open_path = None
            return

        self.save_content()

        content = self.file_manager.load_note_content(note_object.get_file_path())
        self.source_buffer.handler_block_by_func(self._on_buffer_changed)
        self.source_buffer.set_text(content if content is not None else "")
        self.source_buffer.handler_unblock_by_func(self._on_buffer_changed)
        self.text_editor.set_sensitive(True)
        start_iter = self.source_buffer.get_start_iter()
        self.source_buffer.place_cursor(start_iter)

        self.search_entry.set_text(note_object.get_name())

    def _on_search_changed(self, search_entry):
        query = search_entry.get_text().strip().lower()
        logger.debug(f"Search Query: {query}")
        if not hasattr(self, "filter_model"):
            return

        self._current_query = query
        self._note_filter.changed(Gtk.FilterChange.DIFFERENT)

    def _filter_notes(self, note_object, _):
        if isinstance(note_object, Note) and self._current_query:
            return self._current_query in note_object.get_name().lower()
        return True

    def _on_search_activate(self, search_entry):
        name_to_open_or_create = search_entry.get_text().strip()
        if not name_to_open_or_create:
            return

        target_note = None
        model = self.file_manager.get_notes_model()
        for i in range(model.get_n_items()):
            note = model.get_item(i)
            if note.get_name().lower() == name_to_open_or_create.lower():
                target_note = note
                break

        if target_note:
            self._load_note_into_editor(target_note)
            self.text_editor.grab_focus()
        else:
            new_note = self.file_manager.create_note(name_to_open_or_create)
            if new_note:
                self._load_note_into_editor(new_note)
                self.text_editor.grab_focus()

    def _on_buffer_changed(self, text_buffer):
        pass  # For now, rely on focus-out

    def _on_editor_focus_changed(self, widget, param):
        if not widget.has_focus():
            logger.debug("Editor Focus Lost - Saving")
            self.save_content()
            self.results_list_revealer.set_reveal_child(True)
        else:
            logger.debug("Editor Focus Gained - Hiding Revealer")
            self.results_list_revealer.set_reveal_child(False)
            self.source_buffer.place_cursor(self.source_buffer.get_end_iter())

            # Check for external changes when editor gains focus
            if self.file_manager.currently_open_path:
                self.file_manager.check_external_changes(
                    self.file_manager.currently_open_path
                )

        return False  # Allow event propagation

    def _on_notes_reloaded(self, file_manager):
        logger.debug("Window notified: Notes Reloaded")
        if self.filter_model.get_filter():
            self.filter_model.get_filter().changed(Gtk.FilterChange.DIFFERENT)
        if self.sorter:
            self.sort_model.items_changed(0, 0, 0)

    def _on_external_note_change(self, file_manager, note_path):
        if note_path != self.file_manager.currently_open_path:
            return

        note_name = "Unknown"
        for i in range(self.file_manager.get_notes_model().get_n_items()):
            note = self.file_manager.get_notes_model().get_item(i)
            if note.get_file_path() == note_path:
                note_name = note.get_name()
                break

        # Create a toast notification
        toast = Adw.Toast.new(f"{note_name} changed externally")
        toast.set_button_label("Reload")
        toast.set_timeout(0)  # Don't auto-hide
        toast.connect("button-clicked", self._on_reload_toast)
        toast.connect("dismissed", self._on_dismiss_toast)

        self._externally_changed_path = note_path

        if (
            hasattr(self, "toast_overlay")
            and self.toast_overlay is not None
            and self.toast_overlay
        ):
            self.toast_overlay.add_toast(toast)

    def _on_reload_toast(self, toast):
        if hasattr(self, "_externally_changed_path") and self._externally_changed_path:
            content = self.file_manager.load_note_content(self._externally_changed_path)

            if content is not None:
                self.source_buffer.handler_block_by_func(self._on_buffer_changed)
                self.source_buffer.set_text(content)
                self.source_buffer.handler_unblock_by_func(self._on_buffer_changed)

            self._externally_changed_path = None

    def _on_dismiss_toast(self, toast):
        if hasattr(self, "_externally_changed_path") and self._externally_changed_path:
            current_content = self.get_content()

            self.file_manager.save_note_content(
                self._externally_changed_path, current_content, overwrite_external=True
            )

            self._externally_changed_path = None

    def _sort_notes_func(self, note_a, note_b, user_data):
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
            elif sort_method == "date_modified":
                time_a = note_a.get_last_modified()
                time_b = note_b.get_last_modified()

                if time_a > time_b:
                    return -1
                if time_a < time_b:
                    return 1
                return 0
        return 0

    def _on_sorting_method_changed(self, *args):
        logger.debug("Sorting Method Changed - Forcing Re-Sort")
        self.sorter.changed(Gtk.SorterChange.DIFFERENT)
        logger.debug("Sort order updated")

    def _apply_editor_settings(self, *args):
        logger.debug(
            "Applying editor settings (using GtkSourceView style schemes + CSS for font)"
        )

        font_size = self.confman.conf.get("font_size", 12)
        color_scheme_key = self.confman.conf.get("editor_color_scheme", "default")
        use_custom_font = self.confman.conf.get("use_custom_font", False)
        custom_font_name = self.confman.conf.get("custom_font", "Monospace 12")
        show_markdown_syntax = self.confman.conf.get(
            "show_markdown_syntax_highlighting", True
        )
        show_line_numbers = self.confman.conf.get("editor_show_line_numbers", True)
        highlight_current_line = self.confman.conf.get(
            "editor_highlight_current_line", True
        )

        # Apply line numbers and current line highlight
        self.text_editor.set_show_line_numbers(show_line_numbers)
        self.text_editor.set_highlight_current_line(highlight_current_line)

        # 1. Apply Font via CSS (as this was working well)
        font_family_name = "Monospace"
        if use_custom_font:
            try:
                font_desc = Pango.FontDescription.from_string(custom_font_name)
                parsed_family_name = font_desc.get_family()
                if parsed_family_name:
                    font_family_name = parsed_family_name
                else:
                    logger.warning(
                        f"Could not determine font family from '{custom_font_name}'. Using Monospace."
                    )
            except Exception as e:
                logger.warning(
                    f"Could not parse custom font string '{custom_font_name}': {e}. Using Monospace."
                )

        css = f"""
        textview {{
            font-family: '{font_family_name}';
            font-size: {font_size}pt;
        }}
        """
        css_provider = Gtk.CssProvider()
        css_provider.load_from_data(css.encode())

        style_context = self.text_editor.get_style_context()
        if (
            hasattr(self, "_custom_font_style_provider")
            and self._custom_font_style_provider
        ):
            style_context.remove_provider(self._custom_font_style_provider)
        style_context.add_provider(
            css_provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )
        self._custom_font_style_provider = css_provider  # Store for removal/update

        if show_markdown_syntax:
            language_manager = GtkSource.LanguageManager.get_default()
            markdown_language = language_manager.get_language("markdown")
            if markdown_language:
                self.source_buffer.set_language(markdown_language)
                self._apply_source_style_scheme(color_scheme_key)
            else:
                logger.warning(
                    "Markdown language not found. Syntax highlighting disabled."
                )
                self.source_buffer.set_language(None)
                self.source_buffer.set_style_scheme(None)
        else:
            self.source_buffer.set_language(None)
            self.source_buffer.set_style_scheme(None)

    def _apply_source_style_scheme(self, app_scheme_key):
        """Apply GtkSourceView style scheme using the StyleSchemeManager service."""
        style_manager_adw = Adw.StyleManager.get_default()
        is_dark_system = style_manager_adw.get_color_scheme() in [
            Adw.ColorScheme.PREFER_DARK,
            Adw.ColorScheme.FORCE_DARK,
        ]

        # Try to get the scheme directly by ID (works for both custom and bundled schemes)
        style_scheme = self.scheme_manager.get_scheme_by_id(app_scheme_key)

        if style_scheme:
            self.source_buffer.set_style_scheme(style_scheme)
            logger.info(f"Applied GtkSourceView style scheme: {style_scheme.get_id()}")
            return

        # If scheme not found, apply appropriate default based on system theme
        fallback_scheme_id = "Adwaita-dark" if is_dark_system else "Adwaita"
        fallback_scheme = self.scheme_manager.get_scheme_by_id(fallback_scheme_id)

        if fallback_scheme:
            self.source_buffer.set_style_scheme(fallback_scheme)
            logger.info(
                f"Applied fallback GtkSourceView style scheme: {fallback_scheme.get_id()}"
            )
        else:
            logger.error(
                "Could not apply any GtkSourceView style scheme. Editor might look plain."
            )

    def _find_position_by_item(self, item_to_find):
        for i in range(self.sort_model.get_n_items()):
            if self.sort_model.get_item(i) == item_to_find:
                return i
        return None

    def _find_position_by_index(self, index):
        for i in range(self.sort_model.get_n_items()):
            if i == index:
                return self.sort_model.get_item(i)
        return None

    def on_close_request(self, *args):
        logger.info("Window closing - saving files")
        if self.file_manager.currently_open_path:
            logger.info(
                f"Saving file before exit: {self.file_manager.currently_open_path}"
            )
            self.save_content()

        self.save_window_size()

        self.hide()
        return True

    def _setup_list_actions(self):
        action_group = Gio.SimpleActionGroup.new()

        rename_action = Gio.SimpleAction.new("rename", None)
        rename_action.connect("activate", self._on_rename_action)
        action_group.add_action(rename_action)

        delete_action = Gio.SimpleAction.new("delete", None)
        delete_action.connect("activate", self._on_delete_action)
        action_group.add_action(delete_action)

        self.notes_list_view.insert_action_group("item", action_group)

    def _on_rename_action(self, action, param):
        selected_item = self.selection_model.get_selected_item()
        if selected_item and isinstance(selected_item, Note):
            note_list_item = self.get_list_item()
            self._show_rename_popover(selected_item, note_list_item)

    def _on_delete_action(self, action, param):
        selected_item = self.selection_model.get_selected_item()
        if selected_item and isinstance(selected_item, Note):
            self._show_delete_confirmation(selected_item)

    def _show_delete_confirmation(self, note_object):
        dialog = Adw.MessageDialog.new(
            self,
            _("Delete Note?"),
            _(
                "Are you sure you want to delete '{}'? This action cannot be undone."
            ).format(note_object.get_name()),
        )

        dialog.add_response("cancel", _("Cancel"))
        dialog.add_response("delete", _("Delete"))
        dialog.set_response_appearance("delete", Adw.ResponseAppearance.DESTRUCTIVE)
        dialog.set_default_response("cancel")
        dialog.set_close_response("cancel")

        dialog.connect("response", self._on_delete_dialog_response, note_object)
        dialog.present()

    def _on_delete_dialog_response(self, dialog, response, note_object):
        if response == "delete":
            was_open_note = (
                self.file_manager.currently_open_path == note_object.get_file_path()
            )

            success = self.file_manager.delete_note_by_path(note_object.get_file_path())
            if success:
                toast = Adw.Toast.new(f"Note '{note_object.get_name()}' deleted")
                toast.set_timeout(3)
                self.toast_overlay.add_toast(toast)

                if was_open_note:
                    self.source_buffer.set_text("")
                    self.text_editor.set_sensitive(False)
                    self._search_entry_focus()

        dialog.destroy()

    def _show_rename_popover(self, note_object, widget):
        """Show the shared rename popover for a note"""
        self._rename_note_object = note_object
        self.rename_popover.set_note(note_object)

        current_parent = self.rename_popover.get_parent()
        if current_parent:
            self.rename_popover.unparent()

        self.rename_popover.set_parent(widget)
        rect = widget.get_allocation()
        self.rename_popover.set_pointing_to(rect)
        self.rename_popover.popup()

    def _on_persist_window_size_changed(self, confman, enabled):
        self.save_window_size(enabled)
