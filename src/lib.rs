mod actions;
pub mod app;
mod appearance;
pub mod config;
pub mod fuzzy;
pub mod notes;
mod saver;
#[cfg(debug_assertions)]
mod script;
mod search;
pub mod theme;
mod ui;
mod workspace;

pub const APP_ID: &str = match option_env!("NOTY_APP_ID") {
    Some(id) => id,
    None => "com.dagimg.noty",
};
