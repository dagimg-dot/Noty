import logging
import sys
import os
from gi.repository import GLib  # type: ignore
from .. import APPLICATION_ID

DEFAULT_FORMAT = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"

logger = logging.getLogger(APPLICATION_ID)


def setup_logging(level=logging.WARNING, log_to_file=False, log_file=None):
    """Configure the logging system for the application

    Args:
        level: The logging level (default: WARNING for production)
        log_to_file: Whether to log to a file
        log_file: Path to the log file (default: ~/.local/share/noty/noty.log)
    """
    root_logger = logging.getLogger()
    root_logger.setLevel(level)

    for handler in root_logger.handlers[:]:
        root_logger.removeHandler(handler)

    logger.setLevel(level)

    formatter = logging.Formatter(DEFAULT_FORMAT)

    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setFormatter(formatter)
    root_logger.addHandler(console_handler)

    if log_to_file:
        if not log_file:
            data_dir = GLib.get_user_data_dir()
            app_data_dir = os.path.join(data_dir, "noty")
            os.makedirs(app_data_dir, exist_ok=True)
            log_file = os.path.join(app_data_dir, "noty.log")

        file_handler = logging.FileHandler(log_file)
        file_handler.setFormatter(formatter)
        root_logger.addHandler(file_handler)

    logger.debug(f"Logging initialized at level {logging.getLevelName(level)}")
    if log_to_file:
        logger.debug(f"Logging to file: {log_file}")


def debug(msg, *args, **kwargs):
    logger.debug(msg, *args, **kwargs)


def info(msg, *args, **kwargs):
    logger.info(msg, *args, **kwargs)


def warning(msg, *args, **kwargs):
    logger.warning(msg, *args, **kwargs)


def error(msg, *args, **kwargs):
    logger.error(msg, *args, **kwargs)


def critical(msg, *args, **kwargs):
    logger.critical(msg, *args, **kwargs)


setup_logging()
