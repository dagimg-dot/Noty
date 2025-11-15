import gi
import os
import shutil
import xml.etree.ElementTree as ET
from gi.repository import GObject, Gio, GtkSource  # type: ignore
from ..utils import logger, singleton

gi.require_version("GtkSource", "5")


class StyleSchemeManagerSignaler(GObject.Object):
    __gsignals__ = {
        "schemes_updated": (GObject.SignalFlags.RUN_FIRST, None, ()),
        "scheme_imported": (GObject.SignalFlags.RUN_FIRST, None, (str,)),
        "import_error": (GObject.SignalFlags.RUN_FIRST, None, (str,)),
    }


class StyleSchemeManager(metaclass=singleton.Singleton):
    """
    Centralized service for managing GtkSourceView style schemes.

    Handles:
    - Setting up custom schemes directory
    - Copying bundled schemes from resources
    - Importing custom schemes
    - Validating scheme files
    - Managing GtkSourceView search paths
    - Providing schemes information for UI
    """

    BUNDLED_SCHEMES = [
        "noty-classic.xml",
        "noty-dark.xml",
        "coldark-dark.xml",
        "nord.xml",
        "tokyo-night.xml",
    ]

    def __init__(self):
        super().__init__()
        self.signaler = StyleSchemeManagerSignaler()
        self.emit = self.signaler.emit
        self.connect = self.signaler.connect

        self._custom_schemes_dir = None
        self._custom_scheme_mapping = {}
        self._is_initialized = False

    def _ensure_initialized(self):
        """Ensure custom schemes directory is set up (lazy initialization)"""
        if not self._is_initialized:
            self._setup_custom_schemes_directory()
            self._is_initialized = True

    def get_custom_schemes_directory(self):
        """Get the custom schemes directory path, creating it if necessary"""
        if self._custom_schemes_dir is None:
            # Use XDG_DATA_HOME or fallback to ~/.local/share
            data_home = os.environ.get(
                "XDG_DATA_HOME", os.path.expanduser("~/.local/share")
            )
            self._custom_schemes_dir = os.path.join(
                data_home, "noty", "gtksourceview-5", "styles"
            )

        # Create directory if it doesn't exist
        os.makedirs(self._custom_schemes_dir, exist_ok=True)
        return self._custom_schemes_dir

    def _setup_custom_schemes_directory(self):
        """Set up custom schemes directory and add to GtkSourceView search path"""
        try:
            custom_dir = self.get_custom_schemes_directory()

            self._copy_bundled_schemes()

            scheme_manager = GtkSource.StyleSchemeManager.get_default()
            search_paths = scheme_manager.get_search_path()

            if custom_dir not in search_paths:
                search_paths.append(custom_dir)
                scheme_manager.set_search_path(search_paths)
                scheme_manager.force_rescan()
                logger.info(f"Added custom schemes directory: {custom_dir}")

            self._update_custom_scheme_mapping()

        except Exception as e:
            logger.error(f"Error setting up custom schemes directory: {e}")

    def _copy_bundled_schemes(self):
        """Copy bundled style schemes to custom directory if they don't exist"""
        try:
            custom_dir = self.get_custom_schemes_directory()

            for scheme_file in self.BUNDLED_SCHEMES:
                dest_path = os.path.join(custom_dir, scheme_file)

                if not os.path.exists(dest_path):
                    resource_path = f"/com/dagimg/noty/style-schemes/{scheme_file}"

                    try:
                        resource_data = Gio.resources_lookup_data(
                            resource_path, Gio.ResourceLookupFlags.NONE
                        )

                        with open(dest_path, "wb") as f:
                            f.write(resource_data.get_data())

                        logger.info(f"Copied bundled scheme: {scheme_file}")

                    except Exception as e:
                        logger.warning(
                            f"Could not copy bundled scheme {scheme_file}: {e}"
                        )

        except Exception as e:
            logger.error(f"Error copying bundled schemes: {e}")

    def _update_custom_scheme_mapping(self):
        """Update the mapping of custom schemes"""
        try:
            scheme_manager = GtkSource.StyleSchemeManager.get_default()
            all_scheme_ids = scheme_manager.get_scheme_ids()

            from ..utils.constants import COLOR_SCHEMES

            builtin_scheme_names = set(COLOR_SCHEMES.keys())

            self._custom_scheme_mapping = {}

            for scheme_id in all_scheme_ids:
                scheme = scheme_manager.get_scheme(scheme_id)
                if scheme:
                    scheme_name = scheme.get_name()
                    normalized_name = (
                        scheme_name.lower().replace(" ", "_").replace("-", "_")
                    )

                    if (
                        scheme_id not in builtin_scheme_names
                        and normalized_name not in builtin_scheme_names
                    ):
                        self._custom_scheme_mapping[scheme_id] = scheme_name

        except Exception as e:
            logger.error(f"Error updating custom scheme mapping: {e}")

    def get_custom_scheme_mapping(self):
        """Get the mapping of custom scheme IDs to names"""
        self._ensure_initialized()
        return self._custom_scheme_mapping.copy()

    def import_scheme_file(self, file_path):
        """
        Import a style scheme file.

        Returns:
            tuple: (success: bool, message: str, scheme_name: str or None)
        """
        self._ensure_initialized()
        try:
            if not self.validate_style_scheme(file_path):
                return False, "Invalid style scheme file", None

            scheme_info = self.get_scheme_info(file_path)
            scheme_name = scheme_info.get("name", os.path.basename(file_path))

            file_name = os.path.basename(file_path)
            dest_path = os.path.join(self.get_custom_schemes_directory(), file_name)

            if os.path.exists(dest_path):
                return False, f"Scheme '{file_name}' already exists", scheme_name

            shutil.copy2(file_path, dest_path)

            self.refresh_schemes()

            self.emit("scheme_imported", scheme_name)
            return (
                True,
                f"Style scheme '{scheme_name}' imported successfully",
                scheme_name,
            )

        except Exception as e:
            error_msg = f"Failed to import style scheme: {e}"
            logger.error(error_msg)
            self.emit("import_error", error_msg)
            return False, error_msg, None

    def replace_scheme_file(self, file_path):
        """
        Replace an existing style scheme file.

        Returns:
            tuple: (success: bool, message: str, scheme_name: str or None)
        """
        self._ensure_initialized()
        try:
            if not self.validate_style_scheme(file_path):
                return False, "Invalid style scheme file", None

            scheme_info = self.get_scheme_info(file_path)
            scheme_name = scheme_info.get("name", os.path.basename(file_path))

            file_name = os.path.basename(file_path)
            dest_path = os.path.join(self.get_custom_schemes_directory(), file_name)

            shutil.copy2(file_path, dest_path)

            self.refresh_schemes()

            self.emit("scheme_imported", scheme_name)
            return (
                True,
                f"Style scheme '{scheme_name}' replaced successfully",
                scheme_name,
            )

        except Exception as e:
            error_msg = f"Failed to replace style scheme: {e}"
            logger.error(error_msg)
            self.emit("import_error", error_msg)
            return False, error_msg, None

    def validate_style_scheme(self, file_path):
        """Validate that the file is a valid GtkSourceView style scheme"""
        try:
            tree = ET.parse(file_path)
            root = tree.getroot()

            if root.tag != "style-scheme":
                return False

            if "id" not in root.attrib or "name" not in root.attrib:
                return False

            return True
        except ET.ParseError:
            return False
        except Exception:
            return False

    def get_scheme_info(self, file_path):
        """Extract scheme information from XML file"""
        try:
            tree = ET.parse(file_path)
            root = tree.getroot()

            return {
                "id": root.attrib.get("id", ""),
                "name": root.attrib.get("_name", root.attrib.get("name", "")),
                "description": root.attrib.get(
                    "_description", root.attrib.get("description", "")
                ),
            }
        except Exception:
            return {}

    def refresh_schemes(self):
        """Refresh the style scheme manager and update mappings"""
        self._ensure_initialized()
        try:
            scheme_manager = GtkSource.StyleSchemeManager.get_default()
            scheme_manager.force_rescan()

            self._update_custom_scheme_mapping()

            self.emit("schemes_updated")

        except Exception as e:
            logger.error(f"Error refreshing style schemes: {e}")

    def get_scheme_by_id(self, scheme_id):
        """Get a GtkSourceView style scheme by ID"""
        self._ensure_initialized()
        scheme_manager = GtkSource.StyleSchemeManager.get_default()
        return scheme_manager.get_scheme(scheme_id)

    def scheme_exists(self, scheme_id):
        """Check if a scheme exists"""
        return self.get_scheme_by_id(scheme_id) is not None
