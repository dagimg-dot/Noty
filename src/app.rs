use std::path::PathBuf;

use gpui_kit::component::TitleBar;
use gpui_kit::*;

use crate::actions;
use crate::appearance::{self, Fonts};
use crate::config::Config;
use crate::notes::NoteStore;
use crate::theme::ThemeLibrary;
use crate::workspace::Workspace;

const USAGE: &str = "Usage: noty [--notes-dir PATH]

A minimal keyboard driven note taking application.

Options:
  --notes-dir PATH  Open PATH instead of the notes folder from the settings
  -V, --version     Print the version
  -h, --help        Print this help";

struct CommandLine {
    notes_dir: Option<PathBuf>,
}

impl CommandLine {
    fn parse(mut args: impl Iterator<Item = String>) -> Self {
        let mut notes_dir = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--notes-dir" => notes_dir = args.next().map(PathBuf::from),
                "-V" | "--version" => {
                    println!("noty {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                "-h" | "--help" => {
                    println!("{USAGE}");
                    std::process::exit(0);
                }
                other => eprintln!("noty: ignoring unknown argument {other}"),
            }
        }
        Self { notes_dir }
    }
}

pub fn run() {
    let command_line = CommandLine::parse(std::env::args().skip(1));
    let config = Config::load(Config::default_path());
    let store = match command_line.notes_dir {
        Some(dir) => NoteStore::new(
            dir,
            config.recurse_subfolders(),
            config.use_file_extension(),
            config.sort_method(),
        ),
        None => NoteStore::from_config(&config),
    };
    if let Some(dir) = ThemeLibrary::user_dir() {
        let _ = std::fs::create_dir_all(dir);
    }
    let themes = ThemeLibrary::load(&ThemeLibrary::search_dirs());
    for problem in themes.problems() {
        eprintln!("noty: {problem}");
    }

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            actions::bind_keys(cx);
            appearance::register(&themes, cx);
            cx.set_global(Fonts::pick(&config, cx));
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let (width, height) = config.window_size();
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(width), px(height)),
                    cx,
                ))),
                window_min_size: Some(size(px(360.), px(420.))),
                app_id: Some(crate::APP_ID.into()),
                ..TitleBar::window_options()
            };
            let opened = gpui_kit::open_window(options, cx, move |window, cx| {
                appearance::apply(&themes, &config, Some(window), cx);
                cx.new(|cx| Workspace::new(config, themes, store, window, cx))
            });
            match opened {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    eprintln!("noty: could not open a window: {error:#}");
                    cx.quit();
                }
            }
        });
}
