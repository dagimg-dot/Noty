def get_numbered_dict(d):
    return {k: v for v, k in enumerate(d)}


COLOR_SCHEMES = {
    "default": {
        "text": "#000000",
        "background": "#ffffff",
        "selection": "rgba(180, 180, 180, 0.4)",
    },
    "solarized_light": {
        "text": "#657b83",
        "background": "#fdf6e3",
        "selection": "rgba(180, 180, 180, 0.4)",
    },
    "solarized_dark": {
        "text": "#839496",
        "background": "#002b36",
        "selection": "rgba(120, 120, 120, 0.4)",
    },
    "monokai": {
        "text": "#f8f8f2",
        "background": "#272822",
        "selection": "rgba(120, 120, 120, 0.4)",
    },
    "cobalt": {
        "text": "#ffffff",
        "background": "#002240",
        "selection": "rgba(120, 120, 120, 0.4)",
    },
    "nord": {
        "text": "#d8dee9",
        "background": "#2e3440",
        "selection": "rgba(120, 120, 120, 0.4)",
    },
}


COLOR_SCHEMES_DICT = get_numbered_dict(COLOR_SCHEMES)

SORTING_METHODS = {
    "name": 0,
    "date_modified": 1,
}

THEME = {
    "light": 0,
    "dark": 1,
    "system": 2,
}
