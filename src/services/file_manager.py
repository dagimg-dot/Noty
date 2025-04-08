from gi.repository import GObject, Gio  # type: ignore
from os import listdir, path, remove, rename
from datetime import datetime
from .conf_manager import ConfManager
from ..models.note import Note
import time


class FileManager(GObject.Object):
    __gsignals__ = {
        "note_changed": (GObject.SignalFlags.RUN_FIRST, None, (str,)),
        "note_reloaded": (GObject.SignalFlags.RUN_FIRST, None, (str,)),
    }

    def __init__(self):
        super().__init__()
        self.confman = ConfManager()

        self.currently_open_path = None
        self.last_save_time = None
        self.notes_dir = self.confman.conf["notes_dir"]
        self.notes_model = Gio.ListStore.new(Note)

        # Add debounce for note_changed signals
        self._last_note_changed_time = 0
        self._last_note_changed_path = None

        self.confman.connect("notes_dir_changed", self._handle_notes_dir_change)
        self.confman.connect("markdown_syntax_highlighting_changed", self.reload_notes)
        self.confman.connect("recurse_subfolders_changed", self.reload_notes)

        self.reload_notes()

    def get_notes_model(self):
        return self.notes_model

    def load_note_content(self, note_path):
        self.currently_open_path = note_path

        if note_path and path.isfile(note_path):
            try:
                with open(note_path, "r") as file:
                    content = file.read()
                    self.last_save_time = datetime.fromtimestamp(
                        path.getmtime(note_path)
                    )
                    return content
            except Exception as e:
                print(f"Error loading file {note_path}: {e}")
                self.currently_open_path = None
                return ""
        else:
            self.currently_open_path = None
            return ""

    def save_note_content(self, note_path, content, overwrite_external=False):
        """
        Saves the given content to the specified note_path.
        Returns True on success, False otherwise.
        Emits 'note_changed' if an external modification is detected and not overwritten.
        """
        if note_path and path.isfile(note_path):
            if not overwrite_external and self.last_save_time:
                # Check for external modifications first
                if self._detect_external_modification(note_path):
                    return False

            try:
                with open(note_path, "r") as f_read:
                    if f_read.read() == content:
                        self.last_save_time = datetime.now()
                        return True
            except Exception:
                pass

            try:
                with open(note_path, "w") as fd:
                    fd.write(content)
                self.last_save_time = datetime.now()
                note_object = self._find_note_by_path(note_path)
                if note_object:
                    note_object.update_last_modified()
                return True
            except Exception as e:
                print(f"Error saving file {note_path}: {e}")
                return False
        return False

    def create_note(self, name):
        # TODO: Handle potential name collisions, invalid chars
        # TODO: Get extension from ConfManager
        use_md = self.confman.conf.get("use_file_extension", False)
        extension = ".md" if use_md else ""
        file_path = path.join(self.notes_dir, f"{name}{extension}")

        if path.exists(file_path):
            print(f"Note already exists: {file_path}")
            # TODO: Raise error
            return None

        try:
            with open(file_path, "w") as fd:
                fd.write("")
            new_note = Note(file_path)
            self.notes_model.append(new_note)
            # TODO: Emit 'notes_reloaded' or a specific 'note_added' signal
            # self.emit("notes_reloaded")
            return new_note
        except Exception as e:
            print(f"Error creating note {file_path}: {e}")
            return None

    def delete_note_by_path(self, note_path):
        note_to_delete = self._find_note_by_path(note_path)
        if note_to_delete:
            try:
                remove(note_path)
                position = self.notes_model.find(note_to_delete)
                if position[0]:
                    self.notes_model.remove(position[1])
                if self.currently_open_path == note_path:
                    self.currently_open_path = None
                    self.last_save_time = None
                # self.emit("notes_reloaded")  # Simple way for now
                return True
            except Exception as e:
                print(f"Error deleting note {note_path}: {e}")
                return False
        return False

    def rename_note_by_path(self, note_path, new_name):
        note_to_rename = self._find_note_by_path(note_path)
        if note_to_rename:
            # TODO: Handle potential name collisions, invalid chars
            # TODO: Get extension from ConfManager
            use_md = self.confman.conf.get("use_file_extension", False)
            extension = ".md" if use_md else ""
            new_file_path = path.join(path.dirname(note_path), f"{new_name}{extension}")

            if path.exists(new_file_path):
                print(f"Target name already exists: {new_file_path}")
                # TODO: Raise error
                return False

            try:
                rename(note_path, new_file_path)
                note_to_rename.update_after_rename(new_file_path)
                if self.currently_open_path == note_path:
                    self.currently_open_path = new_file_path
                    self.last_save_time = datetime.fromtimestamp(
                        path.getmtime(new_file_path)
                    )
                # self.emit("notes_reloaded")
                return True
            except Exception as e:
                print(f"Error renaming note {note_path} to {new_file_path}: {e}")
                return False
        return False

    def reload_notes(self, *args):
        print("Reloading notes...")  # Debugging
        self.notes_model.remove_all()
        self.notes_dir = self.confman.conf["notes_dir"]
        print(f"Notes directory: {self.notes_dir}")

        try:
            listdir_func = (
                self._listdir_recursive
                if self.confman.conf["recurse_subfolders"]
                else self._listdir_flat
            )

            file_paths = list(listdir_func(self.notes_dir))
            print(f"Found {len(file_paths)} total files")

            count = 0
            for f_path in file_paths:
                use_md = self.confman.conf.get("use_file_extension", False)
                is_valid_extension = (not use_md) or (
                    use_md and f_path.lower().endswith(".md")
                )

                # More detailed debugging
                print(f"Checking file: {f_path}")
                print(f"  Is file: {path.isfile(f_path)}")
                print(f"  Valid extension: {is_valid_extension}")
                print(f"  Hidden: {path.basename(f_path).startswith('.')}")

                if path.isfile(f_path) and is_valid_extension:
                    if not path.basename(f_path).startswith("."):
                        try:
                            note = Note(f_path)
                            self.notes_model.append(note)
                            count += 1
                            print(f"Added note: {note.get_name()}")
                        except Exception as e:
                            print(f"Error creating Note object for {f_path}: {e}")

            print(f"Added {count} notes to the model")

        except FileNotFoundError:
            print(f"Notes directory not found: {self.notes_dir}")
        except Exception as e:
            print(f"Error scanning notes directory {self.notes_dir}: {e}")

        # self.emit("notes_reloaded")
        return count > 0

    def check_external_changes(self, note_path):
        """
        Checks if a file has been externally modified without saving.
        Returns True if external modifications are detected, False otherwise.
        Emits 'note_changed' signal if external modifications detected.
        """
        if note_path and path.isfile(note_path) and self.last_save_time:
            return self._detect_external_modification(note_path)
        return False

    def _detect_external_modification(self, note_path):
        """
        Internal helper to detect if a file has been modified externally.
        Returns True if modification detected, False otherwise.
        Also emits the note_changed signal if needed.
        """
        try:
            print(f"Checking for external modifications to {note_path}")
            current_mtime = datetime.fromtimestamp(path.getmtime(note_path))
            print(
                f"Current mtime: {current_mtime}, Last save time: {self.last_save_time}"
            )

            if current_mtime > self.last_save_time:
                print(f"External modification detected for {note_path}")

                # Debounce the note_changed signal - only emit if
                # at least 1 second has passed since last emission for the same file
                current_time = time.time()
                should_emit = (
                    note_path != self._last_note_changed_path
                    or current_time - self._last_note_changed_time > 1.0
                )

                if should_emit:
                    print("===> EMITTING note_changed signal")
                    self._last_note_changed_time = current_time
                    self._last_note_changed_path = note_path
                    self.emit("note_changed", note_path)

                return True
        except FileNotFoundError:
            print(f"File not found during external check: {note_path}")
            return False

        return False

    # --- Private Helper Methods ---

    def _find_note_by_path(self, note_path):
        for i in range(self.notes_model.get_n_items()):
            note = self.notes_model.get_item(i)
            if note.get_file_path() == note_path:
                return note
        return None

    def _handle_notes_dir_change(self, new_dir):
        print(f"Notes directory changed to: {new_dir}")  # Debug
        self.notes_dir = new_dir
        self.save_note_content(
            self.currently_open_path,
            self.load_note_content(self.currently_open_path),
            overwrite_external=False,
        )
        self.currently_open_path = None
        self.last_save_time = None
        self.reload_notes()

    def _listdir_flat(self, base_path):
        try:
            for fname in listdir(base_path):
                yield path.join(base_path, fname)
        except FileNotFoundError:
            return

    def _listdir_recursive(self, base_path):
        try:
            for entry in listdir(base_path):
                full_path = path.join(base_path, entry)
                if path.isdir(full_path):
                    yield from self._listdir_recursive(full_path)
                else:
                    yield full_path
        except FileNotFoundError:
            return
