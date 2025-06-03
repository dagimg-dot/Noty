import gi
import os
from gettext import gettext as _
from ..services.conf_manager import ConfManager
from ..services.style_scheme_manager import StyleSchemeManager
from ..utils.constants import SORTING_METHODS, COLOR_SCHEMES, THEME
from ..utils import logger

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
gi.require_version("GtkSource", "5")

from gi.repository import Gtk, Adw, Gio, Pango, Gdk  # noqa: E402 # type: ignore


@Gtk.Template(resource_path="/com/dagimg/noty/ui/preferences.ui")
class PreferencesDialog(Adw.PreferencesDialog):
    __gtype_name__ = "PreferencesDialog"

    # General page widgets
    notes_dir_row: Adw.ActionRow = Gtk.Template.Child()
    btn_notes_dir: Gtk.Button = Gtk.Template.Child()
    btn_notes_dir_reset: Gtk.Button = Gtk.Template.Child()
    label_notes_dir: Gtk.Label = Gtk.Template.Child()
    switch_persist_window_size: Gtk.Switch = Gtk.Template.Child()
    switch_recurse_subfolders: Gtk.Switch = Gtk.Template.Child()
    switch_use_file_extension: Gtk.Switch = Gtk.Template.Child()
    dropdown_sorting_method: Gtk.DropDown = Gtk.Template.Child()
    sorting_method_model: Gtk.StringList = Gtk.Template.Child()
    switch_activate_row_on_select: Gtk.Switch = Gtk.Template.Child()

    # Appearance page widgets
    dropdown_theme: Gtk.DropDown = Gtk.Template.Child()
    theme_model: Gtk.StringList = Gtk.Template.Child()
    dropdown_color_scheme: Gtk.DropDown = Gtk.Template.Child()
    color_scheme_model: Gtk.StringList = Gtk.Template.Child()
    switch_markdown_syntax: Gtk.Switch = Gtk.Template.Child()
    switch_show_line_numbers: Gtk.Switch = Gtk.Template.Child()
    switch_highlight_current_line: Gtk.Switch = Gtk.Template.Child()
    switch_custom_font: Gtk.Switch = Gtk.Template.Child()
    font_dialog_btn: Gtk.FontDialogButton = Gtk.Template.Child()
    spin_button_font_size: Gtk.SpinButton = Gtk.Template.Child()
    switch_vim_mode: Gtk.Switch = Gtk.Template.Child()
    btn_import_scheme: Gtk.Button = Gtk.Template.Child()

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.confman = ConfManager()
        self.style_manager = Adw.StyleManager.get_default()
        self.scheme_manager = StyleSchemeManager()

        # Setup toast overlay
        self._setup_toast_overlay()

        # Connect to system color scheme changes
        self.style_manager.connect(
            "notify::system-supports-color-schemes",
            self._on_system_theme_support_changed,
        )
        self.style_manager.connect(
            "notify::color-scheme", self._on_system_theme_changed
        )

        # Connect to style scheme manager signals
        self.scheme_manager.connect("schemes_updated", self._on_schemes_updated)

        # Populate dropdown models from constants
        self._populate_dropdown_models()

        self.btn_notes_dir.connect("clicked", self.on_notes_dir_clicked)
        self.btn_notes_dir_reset.connect("clicked", self.on_notes_dir_reset)
        self.switch_persist_window_size.connect(
            "notify::active", self.on_persist_window_size_changed
        )
        self.switch_recurse_subfolders.connect(
            "notify::active", self.on_recurse_subfolders_changed
        )
        self.switch_use_file_extension.connect(
            "notify::active", self.on_use_file_extension_changed
        )
        self.dropdown_sorting_method.connect(
            "notify::selected", self.on_sorting_method_changed
        )
        self.switch_activate_row_on_select.connect(
            "notify::active", self.on_activate_row_on_select_changed
        )

        self.dropdown_theme.connect("notify::selected", self.on_theme_changed)
        self.dropdown_color_scheme.connect(
            "notify::selected", self.on_color_scheme_changed
        )
        self.btn_import_scheme.connect("clicked", self.on_import_scheme_clicked)
        self.switch_markdown_syntax.connect(
            "notify::active", self.on_markdown_syntax_changed
        )
        self.switch_show_line_numbers.connect(
            "notify::active", self.on_show_line_numbers_changed
        )
        self.switch_highlight_current_line.connect(
            "notify::active", self.on_highlight_current_line_changed
        )
        self.switch_custom_font.connect("notify::active", self.on_custom_font_changed)
        self.font_dialog_btn.connect("notify::font-desc", self.on_font_desc_changed)
        self.spin_button_font_size.connect("value-changed", self.on_font_size_changed)

        self.load_settings()
        self.switch_vim_mode.connect("notify::active", self.on_vim_mode_changed)

    def _setup_toast_overlay(self):
        inner = self.get_child()
        if inner:
            self.set_child(None)
            overlay = Adw.ToastOverlay()
            overlay.set_child(inner)
            self.set_child(overlay)
            self.toast_overlay = overlay

    def _populate_dropdown_models(self):
        for method_name in SORTING_METHODS.keys():
            self.sorting_method_model.append(_(method_name.replace("_", " ").title()))

        for theme_name in THEME.keys():
            self.theme_model.append(_(theme_name.title()))

        self.color_scheme_model.splice(0, self.color_scheme_model.get_n_items())

        for scheme_name in COLOR_SCHEMES.keys():
            display_name = scheme_name.replace("_", " ").title()
            self.color_scheme_model.append(_(display_name))

        self._load_custom_schemes()

    def _load_custom_schemes(self):
        """Load custom schemes from the service and add to dropdown"""
        try:
            custom_schemes = self.scheme_manager.get_custom_scheme_mapping()

            for scheme_id, scheme_name in custom_schemes.items():
                display_name = f"[Custom] {scheme_name}"
                self.color_scheme_model.append(display_name)

        except Exception as e:
            logger.error(f"Error loading custom schemes: {e}")

    def _on_schemes_updated(self, scheme_manager):
        """Handle when schemes are updated in the service"""
        self._populate_dropdown_models()

        self._reload_scheme_selection()

    def _reload_scheme_selection(self):
        """Reload the scheme selection in the dropdown"""
        current_scheme = self.confman.conf["editor_color_scheme"]

        if current_scheme in COLOR_SCHEMES:
            self.dropdown_color_scheme.set_selected(
                list(COLOR_SCHEMES.keys()).index(current_scheme)
            )
        else:
            custom_schemes = self.scheme_manager.get_custom_scheme_mapping()
            if current_scheme in custom_schemes:
                builtin_count = len(COLOR_SCHEMES)
                custom_scheme_ids = list(custom_schemes.keys())
                try:
                    custom_index = custom_scheme_ids.index(current_scheme)
                    self.dropdown_color_scheme.set_selected(
                        builtin_count + custom_index
                    )
                except ValueError:
                    self.dropdown_color_scheme.set_selected(0)
            else:
                self.dropdown_color_scheme.set_selected(0)

    def load_settings(self):
        self.label_notes_dir.set_label(os.path.basename(self.confman.conf["notes_dir"]))
        self.notes_dir_row.set_subtitle(self.confman.conf["notes_dir"])
        self.switch_persist_window_size.set_active(
            self.confman.conf["persist_window_size"]
        )
        self.switch_recurse_subfolders.set_active(
            self.confman.conf["recurse_subfolders"]
        )
        self.switch_use_file_extension.set_active(
            self.confman.conf["use_file_extension"]
        )

        current_sorting = self.confman.conf["sorting_method"]
        if current_sorting in SORTING_METHODS:
            self.dropdown_sorting_method.set_selected(SORTING_METHODS[current_sorting])

        self.switch_activate_row_on_select.set_active(
            self.confman.conf["activate_row_on_select"]
        )

        current_theme = self.confman.conf["theme"]
        if current_theme in THEME:
            self.dropdown_theme.set_selected(THEME[current_theme])

        current_scheme = self.confman.conf["editor_color_scheme"]

        if current_scheme in COLOR_SCHEMES:
            self.dropdown_color_scheme.set_selected(
                list(COLOR_SCHEMES.keys()).index(current_scheme)
            )
        else:
            custom_schemes = self.scheme_manager.get_custom_scheme_mapping()
            if current_scheme in custom_schemes:
                builtin_count = len(COLOR_SCHEMES)
                custom_scheme_ids = list(custom_schemes.keys())
                try:
                    custom_index = custom_scheme_ids.index(current_scheme)
                    self.dropdown_color_scheme.set_selected(
                        builtin_count + custom_index
                    )
                except ValueError:
                    logger.warning(
                        f"Custom scheme '{current_scheme}' not found in mapping"
                    )
                    self.dropdown_color_scheme.set_selected(0)
            else:
                self.dropdown_color_scheme.set_selected(0)

        self.switch_markdown_syntax.set_active(
            self.confman.conf["show_markdown_syntax_highlighting"]
        )

        self.switch_show_line_numbers.set_active(
            self.confman.conf.get("editor_show_line_numbers", True)
        )
        self.switch_highlight_current_line.set_active(
            self.confman.conf.get("editor_highlight_current_line", True)
        )

        self.switch_custom_font.set_active(
            self.confman.conf.get("use_custom_font", False)
        )

        if "custom_font" in self.confman.conf:
            font_desc = Pango.FontDescription.from_string(
                self.confman.conf["custom_font"]
            )
            self.font_dialog_btn.set_font_desc(font_desc)

        self.spin_button_font_size.set_value(self.confman.conf["font_size"])
        # Load Vim mode setting
        self.switch_vim_mode.set_active(self.confman.conf.get("editor_vim_mode", False))

    def on_notes_dir_clicked(self, button):
        dialog = Gtk.FileChooserDialog(
            title=_("Select Notes Directory"),
            action=Gtk.FileChooserAction.SELECT_FOLDER,
            transient_for=self.get_parent(),
            modal=True,
        )
        dialog.add_buttons(
            _("Cancel"),
            Gtk.ResponseType.CANCEL,
            _("Select"),
            Gtk.ResponseType.ACCEPT,
        )

        dialog.set_current_folder(Gio.File.new_for_path(self.confman.conf["notes_dir"]))
        dialog.connect("response", self.on_folder_dialog_response)
        dialog.show()

    def on_folder_dialog_response(self, dialog, response_id):
        if response_id == Gtk.ResponseType.ACCEPT:
            folder_path = dialog.get_file().get_path()
            self.confman.conf["notes_dir"] = folder_path
            self.label_notes_dir.set_label(os.path.basename(folder_path))
            self.notes_dir_row.set_subtitle(folder_path)
            self.confman.save_conf()
            self.confman.emit("notes_dir_changed", folder_path)
        dialog.destroy()

    def on_notes_dir_reset(self, button):
        default_dir = ConfManager.BASE_SCHEMA["notes_dir"]
        self.coknfman.conf["notes_dir"] = default_dir
        self.label_notes_dir.set_label(os.path.basename(default_dir))
        self.notes_dir_row.set_subtitle(default_dir)
        self.confman.save_conf()
        self.confman.emit("notes_dir_changed", default_dir)

    def on_recurse_subfolders_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["recurse_subfolders"] = active
        self.confman.save_conf()
        self.confman.emit("recurse_subfolders_changed", active)

    def on_use_file_extension_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["use_file_extension"] = active
        self.confman.save_conf()

    def on_sorting_method_changed(self, dropdown, param):
        selected = dropdown.get_selected()
        sorting_methods = list(SORTING_METHODS.keys())
        if 0 <= selected < len(sorting_methods):
            self.confman.conf["sorting_method"] = sorting_methods[selected]
            self.confman.save_conf()
            self.confman.emit("sorting_method_changed", sorting_methods[selected])

    def on_activate_row_on_select_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["activate_row_on_select"] = active
        self.confman.save_conf()

    def on_theme_changed(self, dropdown, param):
        selected = dropdown.get_selected()
        themes = list(THEME.keys())

        if 0 <= selected < len(themes):
            selected_theme = themes[selected]
            self.confman.conf["theme"] = selected_theme
            self.confman.save_conf()

            theme_dict = {
                "light": Adw.ColorScheme.FORCE_LIGHT,
                "dark": Adw.ColorScheme.FORCE_DARK,
                "system": Adw.ColorScheme.DEFAULT,
            }

            self.style_manager.set_color_scheme(theme_dict[selected_theme])
            self.confman.emit("theme_changed", selected_theme)

    def _on_system_theme_changed(self, style_manager, pspec):
        if self.confman.conf["theme"] == "system":
            self.confman.emit("theme_changed", "system")

    def _on_system_theme_support_changed(self, style_manager, pspec):
        if self.confman.conf["theme"] == "system":
            self.confman.emit("theme_changed", "system")

    def on_color_scheme_changed(self, dropdown, param):
        selected = dropdown.get_selected()

        builtin_count = len(COLOR_SCHEMES)

        if selected < builtin_count:
            schemes = list(COLOR_SCHEMES.keys())
            selected_scheme = schemes[selected]
        else:
            custom_schemes = self.scheme_manager.get_custom_scheme_mapping()
            custom_scheme_ids = list(custom_schemes.keys())
            custom_index = selected - builtin_count
            if 0 <= custom_index < len(custom_scheme_ids):
                selected_scheme = custom_scheme_ids[custom_index]
            else:
                logger.warning(f"Invalid custom scheme index: {custom_index}")
                return

        self.confman.conf["editor_color_scheme"] = selected_scheme
        self.confman.save_conf()
        self.confman.emit("editor_color_scheme_changed", selected_scheme)

    def on_markdown_syntax_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["show_markdown_syntax_highlighting"] = active
        self.confman.save_conf()
        self.confman.emit(
            "markdown_syntax_highlighting_changed", "enabled" if active else "disabled"
        )

    def on_show_line_numbers_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["editor_show_line_numbers"] = active
        self.confman.save_conf()
        self.confman.emit("editor_show_line_numbers_changed", active)

    def on_highlight_current_line_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["editor_highlight_current_line"] = active
        self.confman.save_conf()
        self.confman.emit("editor_highlight_current_line_changed", active)

    def on_custom_font_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["use_custom_font"] = active
        self.confman.save_conf()
        self.confman.emit("custom_font_changed", "enabled" if active else "disabled")

    def on_font_desc_changed(self, font_button, param):
        font_desc = font_button.get_font_desc()
        if font_desc:
            font_string = font_desc.to_string()
            self.confman.conf["custom_font"] = font_string
            self.confman.save_conf()
            self.confman.emit("custom_font_changed", font_string)

    def on_font_size_changed(self, spin_button):
        value = int(spin_button.get_value())
        self.confman.conf["font_size"] = value
        self.confman.save_conf()
        self.confman.emit("font_size_changed", value)

    def on_vim_mode_changed(self, switch, param):
        active = switch.get_active()
        if active:
            dialog = Adw.MessageDialog.new(
                self.get_parent(),
                _("Enable Vim Mode?"),
                _(
                    "This enables Vim keybindings in the editor."
                    "To ensure you know what you are doing, please type the main command to quit Vim below:"
                ),
            )
            dialog.set_default_size(400, -1)
            dialog.set_body_use_markup(False)

            entry = Gtk.Entry()
            entry.set_placeholder_text(_("Enter Vim exit command..."))
            dialog.set_extra_child(entry)

            dialog.add_response("cancel", _("Cancel"))
            dialog.add_response("submit", _("Submit"))
            dialog.set_response_appearance("submit", Adw.ResponseAppearance.SUGGESTED)
            dialog.set_default_response("cancel")
            dialog.set_close_response("cancel")

            entry.connect("activate", lambda _: dialog.response("submit"))

            key_controller = Gtk.EventControllerKey()
            key_controller.connect(
                "key-pressed", self._on_vim_dialog_key_pressed, dialog
            )
            dialog.add_controller(key_controller)

            dialog.connect("response", self._on_vim_mode_dialog_response, switch, entry)
            dialog.present()
        else:
            self.confman.conf["editor_vim_mode"] = False
            self.confman.save_conf()
            self.confman.emit("vim_mode_changed", False, "user_disabled")

    def _on_vim_dialog_key_pressed(self, controller, keyval, keycode, state, dialog):
        if keyval == Gdk.KEY_Escape:
            dialog.response("cancel")
            return True
        return False

    def _on_vim_mode_dialog_response(self, dialog, response_id, switch, entry):
        if response_id == "submit":
            # Common exit commands: :q :q! :wq :x ZZ ZQ
            # Check for starts with :q as a basic check.
            # Or specific commands like "ZZ" or "ZQ" which don't start with a colon
            command_text = entry.get_text().strip()
            allowed_responses = [":x", "ZZ", "ZQ"]
            is_valid_exit_command = (
                command_text in allowed_responses or command_text.startswith(":q")
            )
            if is_valid_exit_command:
                self.confman.conf["editor_vim_mode"] = True
                self.confman.save_conf()
                self.confman.emit("vim_mode_changed", True, "user_enabled")
                dialog.destroy()

                self._show_toast(_("Vim mode enabled successfully."))
            else:
                self._show_toast(_("Incorrect Vim exit command. Vim mode not enabled."))

                # Block the signal handler to prevent unwanted toast in window
                switch.handler_block_by_func(self.on_vim_mode_changed)
                switch.set_active(False)
                switch.handler_unblock_by_func(self.on_vim_mode_changed)
                dialog.destroy()
        else:
            switch.set_active(False)
            dialog.destroy()

    def _show_toast(self, message):
        toast = Adw.Toast.new(message)
        toast.set_timeout(3)
        if hasattr(self, "toast_overlay"):
            self.toast_overlay.add_toast(toast)
        else:
            logger.info(f"Toast overlay not found, message: {message}")

    def on_persist_window_size_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["persist_window_size"] = active
        self.confman.save_conf()
        self.confman.emit("persist_window_size_changed", active)

    def on_import_scheme_clicked(self, button):
        file_dialog = Gtk.FileDialog()
        file_dialog.set_title(_("Import Style Scheme"))

        # Create file filter for XML files
        xml_filter = Gtk.FileFilter()
        xml_filter.set_name(_("GtkSourceView Style Schemes (*.xml)"))
        xml_filter.add_pattern("*.xml")
        xml_filter.add_mime_type("text/xml")
        xml_filter.add_mime_type("application/xml")

        filter_list = Gio.ListStore.new(Gtk.FileFilter)
        filter_list.append(xml_filter)
        file_dialog.set_filters(filter_list)
        file_dialog.set_default_filter(xml_filter)

        # Open file dialog
        file_dialog.open(self.get_parent(), None, self._on_import_scheme_file_selected)

    def _on_import_scheme_file_selected(self, dialog, result):
        """Handle the selected style scheme file"""
        try:
            file = dialog.open_finish(result)
            if file:
                file_path = file.get_path()
                self._import_style_scheme_file(file_path)
        except Exception as e:
            logger.error(f"Error selecting style scheme file: {e}")
            self._show_toast(_("Failed to open file dialog"))

    def _import_style_scheme_file(self, file_path):
        """Import and validate a style scheme file using the service"""
        try:
            # Use the service to import the scheme
            success, message, scheme_name = self.scheme_manager.import_scheme_file(
                file_path
            )

            if success:
                self._show_toast(_(message))
            else:
                if "already exists" in message:
                    self._show_replace_confirmation(file_path, scheme_name)
                else:
                    self._show_toast(_(message))

        except Exception as e:
            logger.error(f"Error importing style scheme: {e}")
            self._show_toast(_("Failed to import style scheme"))

    def _show_replace_confirmation(self, source_path, scheme_name):
        """Show confirmation dialog for replacing existing scheme"""
        file_name = os.path.basename(source_path)
        dialog = Adw.MessageDialog.new(
            self.get_parent(),
            _("Replace Style Scheme?"),
            _(
                f"A style scheme named '{file_name}' already exists. Do you want to replace it?"
            ),
        )

        dialog.add_response("cancel", _("Cancel"))
        dialog.add_response("replace", _("Replace"))
        dialog.set_response_appearance("replace", Adw.ResponseAppearance.DESTRUCTIVE)
        dialog.set_default_response("cancel")
        dialog.set_close_response("cancel")

        dialog.connect("response", self._on_replace_confirmation_response, source_path)
        dialog.present()

    def _on_replace_confirmation_response(self, dialog, response_id, source_path):
        """Handle replace confirmation response using the service"""
        if response_id == "replace":
            try:
                success, message, scheme_name = self.scheme_manager.replace_scheme_file(
                    source_path
                )

                if success:
                    self._show_toast(_(message))
                else:
                    self._show_toast(_(message))

            except Exception as e:
                logger.error(f"Error replacing style scheme: {e}")
                self._show_toast(_("Failed to replace style scheme"))

        dialog.destroy()
