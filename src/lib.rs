pub mod config;

pub const APP_ID: &str = match option_env!("NOTY_APP_ID") {
    Some(id) => id,
    None => "com.dagimg.noty",
};
