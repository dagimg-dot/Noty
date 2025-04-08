import gi
import os
from gettext import gettext as _

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")

from gi.repository import Gtk, Adw, Gio  # noqa: E402 # type: ignore
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
    switch_recurse_subfolders: Gtk.Switch = Gtk.Template.Child()
    switch_use_file_extension: Gtk.Switch = Gtk.Template.Child()
    dropdown_sorting_method: Gtk.DropDown = Gtk.Template.Child()
    switch_activate_row_on_select: Gtk.Switch = Gtk.Template.Child()

    # Appearance page widgets
    dropdown_theme: Gtk.DropDown = Gtk.Template.Child()
    dropdown_color_scheme: Gtk.DropDown = Gtk.Template.Child()
    switch_markdown_syntax: Gtk.Switch = Gtk.Template.Child()
    spin_button_font_size: Gtk.SpinButton = Gtk.Template.Child()

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.confman = ConfManager()
        self.style_manager = Adw.StyleManager.get_default()

        self.btn_notes_dir.connect("clicked", self.on_notes_dir_clicked)
        self.btn_notes_dir_reset.connect("clicked", self.on_notes_dir_reset)
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
        self.spin_button_font_size.connect("value-changed", self.on_font_size_changed)

        self.load_settings()

    def load_settings(self):
        self.label_notes_dir.set_label(os.path.basename(self.confman.conf["notes_dir"]))
        self.notes_dir_row.set_subtitle(self.confman.conf["notes_dir"])
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
        self.dropdown_theme.set_selected(THEME[current_theme])

        current_scheme = self.confman.conf["editor_color_scheme"]
        color_schemes = {k: v for v, k in enumerate(COLOR_SCHEMES)}

        if current_scheme in color_schemes:
            self.dropdown_color_scheme.set_selected(color_schemes[current_scheme])

        self.switch_markdown_syntax.set_active(
            self.confman.conf["show_markdown_syntax_highlighting"]
        )
        self.spin_button_font_size.set_value(self.confman.conf["font_size"])

    def on_notes_dir_clicked(self, button):
        dialog = Gtk.FileChooserDialog(
            title=_("Select Notes Directory"),
            action=Gtk.FileChooserAction.SELECT_FOLDER,
            transient_for=self,
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
        self.confman.conf["notes_dir"] = default_dir
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
        methods = list(SORTING_METHODS.keys())
        if 0 <= selected < len(methods):
            self.confman.conf["sorting_method"] = methods[selected]
            self.confman.save_conf()
            self.confman.emit("sorting_method_changed", methods[selected])

    def on_activate_row_on_select_changed(self, switch, param):
        active = switch.get_active()
        self.confman.conf["activate_row_on_select"] = active
        self.confman.save_conf()

    def on_theme_changed(self, dropdown, param):
        selected = dropdown.get_selected()
        themes = list(THEME.keys())
        if 0 <= selected < len(themes):
            self.confman.conf["theme"] = themes[selected]
            self.confman.save_conf()

            is_dark = self.style_manager.get_color_scheme() in [
                Adw.ColorScheme.PREFER_DARK,
                Adw.ColorScheme.FORCE_DARK,
            ]

            theme_dict = {
                "light": Adw.ColorScheme.FORCE_LIGHT,
                "dark": Adw.ColorScheme.FORCE_DARK,
                "system": Adw.ColorScheme.PREFER_DARK
                if is_dark
                else Adw.ColorScheme.PREFER_LIGHT,
            }

            self.style_manager.set_color_scheme(theme_dict[themes[selected]])
            self.confman.emit("theme_changed", themes[selected])

    def on_color_scheme_changed(self, dropdown, param):
        selected = dropdown.get_selected()
        schemes = ["default", "solarized-light", "solarized-dark", "monokai", "cobalt"]
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

    def on_font_size_changed(self, spin_button):
        value = int(spin_button.get_value())
        self.confman.conf["font_size"] = value
        self.confman.save_conf()
        self.confman.emit("font_size_changed", value)
