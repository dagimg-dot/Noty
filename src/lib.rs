mod actions;
pub mod app;
mod appearance;
pub mod config;
pub mod fuzzy;
pub mod notes;
pub mod theme;
mod ui;
mod workspace;

pub const APP_ID: &str = match option_env!("NOTY_APP_ID") {
    Some(id) => id,
    None => "com.dagimg.noty",
};
