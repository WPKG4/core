use lazy_static::lazy_static;
use std::time::Duration;
use uuid::Uuid;

lazy_static! {
    pub static ref UUID: Uuid = Uuid::new_v4();
    pub static ref PING_INTERVAL: Duration = Duration::from_secs(5 * 60);
}
pub static IP: &str = "127.0.0.1:5000";