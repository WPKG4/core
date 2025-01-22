use std::env;

use anyhow::Result;
use figlet_rs::FIGfont;
use tracing::{debug, info};

use crate::client::masterclient::MasterClient;

mod client;
mod commands;
mod config;
mod install;
mod logger;
mod updater;

#[tokio::main]
async fn main() -> Result<()> {
    logger::init();
    display_banner();

    debug!(
        "Executable: {}, Version: {}, Install path: {}",
        env::current_exe()?.display(),
        env!("CARGO_PKG_VERSION"),
        config::INSTALL_PATH.display()
    );

    if should_update().await? {
        install::update_mode().await?;
    } else if should_install()? {
        install::install(env::current_exe()?).await?;
    }

    
    #[cfg(not(debug_assertions))]
    {
        debug!("Starting updater");
        tokio::spawn(async move { updater::start_updater().await });
    }


    start_client().await?;

    Ok(())
}

fn display_banner() {
    let standard_font = FIGfont::standard();
    match standard_font {
        Ok(font) => {
            if let Some(figure) = font.convert("WPKG4 - Szybkie i Zajebiste") {
                println!("{}", figure);
            }
        }
        Err(_) => {
            info!("WPKG4 - Szybkie i Zajebiste")
        }
    }
}

async fn should_update() -> Result<bool> {
    Ok(config::load_config().await.is_ok()
        && config::get_config("update-mode").await.unwrap_or_else(|_| "false".to_string())
            == "true")
}

fn should_install() -> Result<bool> {
    Ok(env::current_exe()?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not get current executable path!"))?
        != config::INSTALL_PATH.as_path()
        && !cfg!(debug_assertions))
}

async fn start_client() -> Result<()> {
    let mut client = MasterClient::new(&config::get_config("ip").await?).await?;
    client.register().await?;
    client.handle().await?;
    Ok(())
}
