use anyhow::Result;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

lazy_static! {
    pub static ref INSTALL_PATH: PathBuf = {
        match cfg!(debug_assertions) {
            false => match option_env!("INSTALL_PATH") {
                Some(x) => PathBuf::from(x),
                None => match dirs::data_local_dir() {
                    Some(x) => x.join("WPKG4"),
                    None => PathBuf::from(""),
                },
            },
            true => PathBuf::from("./workdir"),
        }
    };
    pub static ref UUID: Uuid = Uuid::new_v4();
    pub static ref PING_INTERVAL: Duration = Duration::from_secs(5 * 60);
    static ref CONFIG: RwLock<HashMap<String, String>> = RwLock::new(load_default_config());
}

pub async fn set_config(key: &str, value: &str) {
    let mut config = CONFIG.write().await;
    config.insert(key.to_string(), value.to_string());
}

pub async fn rm_config(key: &str) {
    let mut config = CONFIG.write().await;
    config.remove(&key.to_string());
}

pub async fn get_config(key: &str) -> Result<String> {
    CONFIG
        .read()
        .await
        .get(key)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Could not find value of key: {}", key))
}

pub async fn load_config() -> Result<()> {
    let toml_string = fs::read_to_string(INSTALL_PATH.join("config.toml"))?;
    let parsed_config = toml::from_str(&toml_string)?;
    let mut config = CONFIG.write().await;
    *config = parsed_config;
    Ok(())
}

pub async fn save_config() -> Result<()> {
    if !INSTALL_PATH.exists() {
        fs::create_dir_all(&*INSTALL_PATH)?;
    }
    let config = CONFIG.read().await.clone();
    let toml_string = toml::to_string(&config)?;
    fs::write(INSTALL_PATH.join("config.toml"), toml_string)?;

    Ok(())
}

pub fn load_default_config() -> HashMap<String, String> {
    HashMap::from([
        ("IP".to_string(), "127.0.0.1:5000".to_string()),
        ("UUID".to_string(), Uuid::new_v4().to_string()),
        ("GROUP".to_string(), "MASTER".to_string()),
    ])
}

pub static IP: &str = "127.0.0.1:5000";
