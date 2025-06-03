import gi
import os
from gettext import gettext as _

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")

from gi.repository import Gtk, Adw, Gio, Pango  # noqa: E402 # type: ignore
from ..services.conf_manager import ConfManager  # noqa: E402
from ..utils.constants import SORTING_METHODS, COLOR_SCHEMES, THEME  # noqa: E402


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

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.confman = ConfManager()
        self.style_manager = Adw.StyleManager.get_default()

        # Connect to system color scheme changes
        self.style_manager.connect(
            "notify::system-supports-color-schemes",
            self._on_system_theme_support_changed,
        )
        self.style_manager.connect(
            "notify::color-scheme", self._on_system_theme_changed
        )

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

    def _populate_dropdown_models(self):
        for method_name in SORTING_METHODS.keys():
            self.sorting_method_model.append(_(method_name.replace("_", " ").title()))

        for theme_name in THEME.keys():
            self.theme_model.append(_(theme_name.title()))

        for scheme_name in COLOR_SCHEMES.keys():
            display_name = scheme_name.replace("_", " ").title()
            self.color_scheme_model.append(_(display_name))

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

        # Set dropdown selections from config
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
        schemes = list(COLOR_SCHEMES.keys())
        if 0 <= selected < len(schemes):
            self.confman.conf["editor_color_scheme"] = schemes[selected]
            self.confman.save_conf()
            self.confman.emit("editor_color_scheme_changed", schemes[selected])

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

    def on_persist_window_size_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["persist_window_size"] = active
        self.confman.save_conf()
        self.confman.emit("persist_window_size_changed", active)
