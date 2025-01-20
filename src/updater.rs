use anyhow::Result;
use serde::Deserialize;

use crate::config::UPDATE_URL;

#[derive(Debug, Deserialize)]
struct UpdateInfo {
    version: String,
    checksum: String,
    url: String,
}

async fn get_update() -> Result<UpdateInfo> {
    let endpoint = format!("{}/api/core/stable/windows/x64/json", UPDATE_URL.to_string());
    let response = reqwest::get(endpoint).await?;
    let update_info: UpdateInfo = response.json().await?;
    Ok(update_info)
}