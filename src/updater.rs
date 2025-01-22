use std::env::consts;
use std::fs::Permissions;
#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

use anyhow::Result;
use serde::Deserialize;
use tokio::{fs, time};
use tracing::{debug, error};

use crate::config::{self, INSTALL_PATH};

#[derive(Debug, Deserialize)]
struct UpdateInfo {
    version: String,
    checksum: String,
    path: String,
}

async fn get_update() -> Result<UpdateInfo> {
    let endpoint = format!(
        "{}/api/core/stable/{}/{}/json",
        config::UPDATE_URL.to_string(),
        consts::OS,
        consts::ARCH
    );
    debug!("Update info URL for this system: {}", endpoint);
    let response = reqwest::get(endpoint).await?;
    let update_info: UpdateInfo = response.json().await?;
    Ok(update_info)
}

pub async fn start_updater() {
    if !INSTALL_PATH.exists() {
        debug!("Working directory not found, creating: {}", INSTALL_PATH.display());
        let _ = fs::create_dir(INSTALL_PATH.as_path()).await;
    }
    loop {
        debug!("Checking for an update");

        match get_update().await {
            Ok(update_info) => {
                if !env!("CARGO_PKG_VERSION").eq(&update_info.version) {
                    debug!("Found an update; version {}", &update_info.version);

                    let mut tried = 0;
                    while tried < 3 {
                        debug!("Try {}/3: Downloading update", tried + 1);
                        let Ok(response) =
                            reqwest::get(config::UPDATE_URL.clone() + "/files" + &update_info.path)
                                .await
                        else {
                            error!("Failed to fetch the update");
                            break;
                        };

                        let Ok(binary) = response.bytes().await else {
                            error!("Failed to fetch the update");
                            break;
                        };

                        let digest = sha256::digest(binary.to_vec());
                        debug!(
                            "File checksum: {}, Required checksum: {}",
                            &digest, &update_info.checksum
                        );

                        if digest.eq(&update_info.checksum) {
                            debug!("Update downloaded");

                            let save_path = config::UPDATER_BINARY_FILE.clone();

                            let _ = fs::remove_file(save_path.as_path()).await;
                            if let Err(err) = fs::write(save_path.as_path(), binary).await {
                                error!("Failed to save binary: {}", err);
                                break;
                            }

                            #[cfg(target_os = "linux")]
                            if let Err(err) = fs::set_permissions(
                                save_path.as_path(),
                                Permissions::from_mode(0o755),
                            )
                            .await
                            {
                                error!("Failed to set file permissions: {}", err);
                                break;
                            }

                            if let Err(err) = crate::install::install(save_path).await {
                                error!("Update failed: {}", err);
                                break;
                            };
                        }

                        tried += 1
                    }

                    if tried == 3 {
                        error!("Tried 3 times of 3. Checksum mismatched.");
                    }
                }
            }
            Err(err) => error!("Updater failed to check for an update: {}", err),
        }

        time::sleep(config::PING_INTERVAL.clone()).await;
    }
}
