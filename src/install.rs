use std::{env, fs::Permissions, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use anyhow::Result;
use tokio::{fs, process::Command, time};
use tracing::debug;

use crate::config::{self, INSTALL_PATH};

pub async fn install(binary_path: PathBuf) -> Result<()> {
    if !INSTALL_PATH.exists() {
        debug!("Working directory not found, creating: {}", INSTALL_PATH.display());
        fs::create_dir(INSTALL_PATH.as_path()).await?;
    }

    debug!("Installing to {}", INSTALL_PATH.display());

    debug!("Enabling update mode and saving configuration");
    config::set_config("update-mode", "true").await;
    config::save_config().await?;

    debug!("Running in update mode");
    Command::new(binary_path).spawn()?;

    std::process::exit(0);
}

pub async fn update_mode() -> Result<()> {
    debug!("Running in update mode");
    time::sleep(Duration::from_millis(100)).await;

    debug!("Moving/Replacing executable with updated one");
    let current_executable = env::current_exe()?;
    let _ = fs::remove_file(config::BINARY_FILE.as_path()).await;
    fs::copy(current_executable, config::BINARY_FILE.as_path()).await?;
    fs::set_permissions(config::BINARY_FILE.as_path(), Permissions::from_mode(0o755)).await?;

    debug!("Disabling update mode");
    config::set_config("update-mode", "false").await;
    config::save_config().await?;

    debug!("Running updated executable in regular mode");
    Command::new(config::BINARY_FILE.as_path()).spawn()?;

    std::process::exit(0);
}