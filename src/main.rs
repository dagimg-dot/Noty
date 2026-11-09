#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod config;
#[cfg(debug_assertions)]
mod debug_replay;
mod format;
mod notes;
#[cfg(unix)]
mod platform;
mod theme;
mod ui;

pub const APP_ID: &str = match option_env!("NOTY_APP_ID") {
    Some(id) => id,
    None => "com.dagimg.noty",
};

fn main() {
    app::run();
}
