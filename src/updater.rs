use anyhow::Result;
use serde::Deserialize;
use tracing::debug;

use crate::config::UPDATE_URL;

#[derive(Debug, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub checksum: String,
    pub path: String,
}
pub async fn get_update() -> Result<UpdateInfo> {
    let endpoint = format!("{}/files/core/stable/{}/{}/version.json", *UPDATE_URL, std::env::consts::OS, std::env::consts::ARCH);
    let response = reqwest::get(endpoint).await?;
    let update_info: UpdateInfo = response.json().await?;
    Ok(update_info)
}