use std::{env, fs};
use crate::config::INSTALL_PATH;
use anyhow::Result;
use tokio::process::Command;
use tracing::debug;
use crate::config;

pub async fn install() -> Result<()> {
    if !INSTALL_PATH.exists() {
        debug!("Working directory not found, creating: {}", INSTALL_PATH.display());
        fs::create_dir(INSTALL_PATH.as_path())?;
    }
    debug!("Installing: {}", INSTALL_PATH.display());
    debug!("Saving configuration");
    config::save_config().await?;
    debug!("Copying binary into working directory");
    let executable_path = INSTALL_PATH.join(if cfg!(windows) { "core-rs.exe" } else { "core-rs" });
    fs::copy(env::current_exe()?, executable_path.clone())?;
    debug!("Restarting: {}", executable_path.display());
    Command::new(executable_path).spawn()?;
    std::process::exit(0);
}