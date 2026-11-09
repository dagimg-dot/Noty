#![cfg_attr(windows, windows_subsystem = "windows")]

mod actions;
mod app;
mod appearance;
mod config;
#[cfg(debug_assertions)]
mod debug_replay;
mod format;
mod fuzzy;
mod notes;
#[cfg(unix)]
mod platform;
mod search;
mod theme;
mod ui;
mod workspace;

pub const APP_ID: &str = match option_env!("NOTY_APP_ID") {
    Some(id) => id,
    None => "com.dagimg.noty",
};

fn main() {
    app::run();
}
