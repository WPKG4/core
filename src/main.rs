use std::env;
use std::error::Error;
use std::time::Duration;

use figlet_rs::FIGfont;
use tokio::time;
use tracing::debug;

use crate::client::masterclient::MasterClient;

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

    debug!("Executable: {}, Install path: {}", env::current_exe()?.display(), config::INSTALL_PATH.display());
    if config::load_config().await.is_ok() && config::get_config("update-mode").await.unwrap_or("false".to_string()).eq("true") {
        install::update_mode().await?;
    } else if env::current_exe()?.parent().ok_or("Could not get current executable path!")?
        != config::INSTALL_PATH.as_path() {
        install::install(env::current_exe()?).await?;
    }

    debug!("Starting updater");
    tokio::spawn(async move { updater::start_updater().await });

    loop {
        time::sleep(Duration::from_secs(5)).await;
    }

    // let mut client = MasterClient::new(IP).await?;
    // client.register().await?;
    // client.handle().await?;

    // Ok(())
}
