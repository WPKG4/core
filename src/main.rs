use std::env;
use std::error::Error;

use figlet_rs::FIGfont;
use tracing::debug;

use crate::client::masterclient::MasterClient;
use crate::config::INSTALL_PATH;

mod client;
mod commands;
mod config;
mod install;
mod logger;
mod updater;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logger::init();
    let standard_font = FIGfont::standard()?;
    let figure = standard_font.convert("WPKG4 - szybkie i zajebiste");
    println!("{}", figure.unwrap());

    if env::current_exe()?.parent().ok_or("Could not get current executable path!")?
        != INSTALL_PATH.as_path()
        && !cfg!(debug_assertions)
    {
        install::install().await?;
    }

    config::set_config("test", "test_config123").await;
    config::save_config().await?;
    config::load_config().await?;
    debug!("{}", updater::get_update().await?.version);
    debug!("Config test value: {}", config::get_config("test").await?);

    let mut client = MasterClient::new(config::get_config("IP").await?.as_str()).await?;
    client.register().await?;
    client.handle().await?;

    Ok(())
}
