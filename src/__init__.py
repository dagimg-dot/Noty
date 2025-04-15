# Application ID set by Meson during build
APPLICATION_ID = "@application_id@"
VERSION = "@VERSION@"

# Export the application ID for use in other modules
__all__ = ["APPLICATION_ID", "VERSION"]
