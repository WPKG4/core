use crate::client::coreclient::CoreClient;
use crate::config::{INSTALL_PATH, IP};
use figlet_rs::FIGfont;
use std::env;
use std::error::Error;
use tracing::debug;

mod client;
mod commands;
mod config;
mod install;
mod logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logger::init();
    let standard_font = FIGfont::standard()?;
    let figure = standard_font.convert("WPKG4 - szybkie i zajebiste");
    println!("{}", figure.unwrap());

    if env::current_exe()?
        .parent()
        .ok_or("Could not get current executable path!")?
        != INSTALL_PATH.as_path()
        && !cfg!(debug_assertions)
    {
        install::install().await?;
    }

    config::set_config("test", "test_config123").await;
    config::save_config().await?;
    config::load_config().await?;
    debug!("Config test value: {}", config::get_config("test").await?);

    let mut client = CoreClient::new(IP).await?;
    client.register().await?;
    client.handle().await?;

    Ok(())
}
