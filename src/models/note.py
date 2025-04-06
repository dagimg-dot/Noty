from gi.repository import GObject  # type: ignore
from os import path


class Note(GObject.Object):
    """
    Represents a single not file
    """

    __gtype_name__ = "Note"

    name = GObject.Property(
        type=str,
        nick="Name",
        blurb="Filename without extension (but might have one if '.md' is added)",
    )

    file_path = GObject.Property(
        type=str,
        nick="File Path",
        blurb="Full path to the note file",
        flags=GObject.ParamFlags.READABLE,
    )

    last_modified = GObject.Property(
        type=float,
        nick="Last Modified",
        blurb="Unix timestamp of when the note was last modified",
        flags=GObject.ParamFlags.READABLE,
    )

    def __init__(self, file_path):
        super().__init__()
        self._file_path = file_path
        self._last_modified = path.getmtime(file_path)
        base_name = path.basename(self._file_path)
        self._name = path.splitext(base_name)[0] or base_name

        self.notify("name")
        self.notify("file_path")
        self.notify("last_modified")

    def do_get_property(self, prop):
        if prop.name == "name":
            return self._name
        elif prop.name == "file_path":
            return self._file_path
        elif prop.name == "last_modified":
            return self._last_modified
        else:
            raise AttributeError(f"unknown property {prop.name}")

    def get_name(self):
        return self._name

    def get_file_path(self):
        return self._file_path

    def get_last_modified(self):
        return self._last_modified

    def update_last_modified(self):
        new_time = path.getmtime(self._file_path)
        if new_time != self._last_modified:
            self._last_modified = new_time
            self.notify("last_modified")

    def update_after_rename(self, new_file_path):
        self._file_path = new_file_path
        # TODO: Re-calculate name based on new path (will need ConfManager later)
        base_name = path.basename(self._file_path)
        self._name = path.splitext(base_name)[0]
        self._last_modified = path.getmtime(self._file_path)

        self.notify("name")
        self.notify("file_path")
        self.notify("last_modified")
