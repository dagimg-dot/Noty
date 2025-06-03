def get_numbered_dict(d):
    return {k: v for v, k in enumerate(d)}


COLOR_SCHEMES = {
    "default": 0,
    "solarized_light": 1,
    "solarized_dark": 2,
    "cobalt": 3,
    "kate": 4,
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
