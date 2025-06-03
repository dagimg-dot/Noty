from gettext import gettext as _
from pathlib import Path
from os.path import isfile, isdir
from os import environ as Env, system, makedirs
import json
from gi.repository import GObject, GLib  # type: ignore
from .. import APPLICATION_ID
from ..utils import logger, singleton

documents_dir = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOCUMENTS)
if not documents_dir:
    logger.info("Documents directory not found, creating it")
    system("xdg-user-dirs-update")
    documents_dir = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOCUMENTS)
    if not documents_dir:
        logger.info("Documents directory still not found, using home directory")
        documents_dir = f"{Env.get('HOME')}/Documents"


class ConfManagerSignaler(GObject.Object):
    __gsignals__ = {
        "notes_dir_changed": (GObject.SignalFlags.RUN_FIRST, None, (str,)),
        "markdown_syntax_highlighting_changed": (
            GObject.SignalFlags.RUN_FIRST,
            None,
            (str,),
        ),
        "theme_changed": (GObject.SignalFlags.RUN_FIRST, None, (str,)),
        "sorting_method_changed": (GObject.SignalFlags.RUN_FIRST, None, (str,)),
        "editor_color_scheme_changed": (GObject.SignalFlags.RUN_FIRST, None, (str,)),
        "font_size_changed": (GObject.SignalFlags.RUN_FIRST, None, (int,)),
        "recurse_subfolders_changed": (GObject.SignalFlags.RUN_FIRST, None, (bool,)),
        "persist_window_size_changed": (GObject.SignalFlags.RUN_FIRST, None, (bool,)),
        "custom_font_changed": (GObject.SignalFlags.RUN_FIRST, None, (str,)),
        "editor_show_line_numbers_changed": (
            GObject.SignalFlags.RUN_FIRST,
            None,
            (bool,),
        ),
        "editor_highlight_current_line_changed": (
            GObject.SignalFlags.RUN_FIRST,
            None,
            (bool,),
        ),
        "vim_mode_changed": (GObject.SignalFlags.RUN_FIRST, None, (bool, str)),
    }


class ConfManager(metaclass=singleton.Singleton):
    BASE_SCHEMA = {
        "windowsize": {"width": 500, "height": 800},
        "persist_window_size": True,
        "notes_dir": "{0}/{1}".format(documents_dir, _("Noties")),
        "show_markdown_syntax_highlighting": False,
        "theme": "system",
        "sorting_method": "name",
        "activate_row_on_select": False,
        "use_file_extension": False,
        "editor_color_scheme": "default",
        "recurse_subfolders": False,
        "font_size": 12,
        "use_custom_font": False,
        "custom_font": "Monospace 12",
        "last_opened_file": None,
        "editor_show_line_numbers": True,
        "editor_highlight_current_line": True,
        "editor_vim_mode": False,
    }

    def __init__(self):
        self.window = None
        self.signaler = ConfManagerSignaler()
        self.emit = self.signaler.emit
        self.connect = self.signaler.connect

        # check if inside flatpak sandbox
        self.is_flatpak = "XDG_RUNTIME_DIR" in Env.keys() and isfile(
            f"{Env['XDG_RUNTIME_DIR']}/flatpak-info"
        )

        if self.is_flatpak:
            logger.info("Running inside flatpak sandbox")
            self.path = Path(f"{Env.get('XDG_CONFIG_HOME')}/{APPLICATION_ID}.json")
        else:
            logger.info("Running outside flatpak sandbox")
            self.path = Path(f"{Env.get('HOME')}/.config/{APPLICATION_ID}.json")

        self.conf = None

        if isfile(str(self.path)):
            logger.info("Config file found, loading it")
            self.load_conf()
        else:
            logger.info("Config file not found, using default schema")
            self.conf = ConfManager.BASE_SCHEMA.copy()
            self.save_conf()

        if not isdir(self.conf["notes_dir"]):
            try:
                makedirs(self.conf["notes_dir"])
            except PermissionError:
                logger.info("Permission error creating notes directory, using default")
                self.conf["notes_dir"] = self.BASE_SCHEMA["notes_dir"]
                self.save_conf()
                if not isdir(self.conf["notes_dir"]):
                    makedirs(self.conf["notes_dir"])

    def load_conf(self):
        try:
            with open(str(self.path)) as fd:
                self.conf = json.loads(fd.read())

            # verify that the file has all of the schema keys
            for k in ConfManager.BASE_SCHEMA:
                if k not in self.conf.keys():
                    if isinstance(ConfManager.BASE_SCHEMA[k], (list, dict)):
                        self.conf[k] = ConfManager.BASE_SCHEMA[k].copy()
                    else:
                        self.conf[k] = ConfManager.BASE_SCHEMA[k]

            # Handle transition from dark_mode to theme
            if "dark_mode" in self.conf and "theme" not in self.conf:
                self.conf["theme"] = "dark" if self.conf["dark_mode"] else "light"
                del self.conf["dark_mode"]
                self.save_conf()

        except Exception:
            logger.info("Error loading config file, using default schema")
            self.conf = ConfManager.BASE_SCHEMA.copy()
            self.save_conf()

    def save_conf(self, *args):
        with open(str(self.path), "w") as fd:
            fd.write(json.dumps(self.conf))
