use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use super::utils::streamer::{ScreenStreamer, StreamConfig};
use crate::client::net::types::out::payloads::OutPayloadType;
use crate::client::net::types::shared::MessagePayload;
use crate::client::wpkgclient::coreclient::CoreClient;
use crate::commands::Command;

pub struct StreamScreen;

pub(crate) const NAME: &str = "streamscreen";

lazy_static::lazy_static! {
    pub static ref SCREEN_STREAMER: Arc<Mutex<ScreenStreamer>> =
        Arc::new(Mutex::new(ScreenStreamer::new()));
}

async fn send_response<R: AsyncRead + AsyncWrite + Unpin + Send>(
    client: &mut CoreClient<R>,
    error_code: &str,
    message: &str,
) -> anyhow::Result<()> {
    let formatted = format!("{} {} {}", error_code, message.len(), message);
    client
        .wtp_client
        .send_packet(OutPayloadType::Message(MessagePayload::from_str(&formatted)))
        .await?;
    Ok(())
}

#[async_trait]
impl<R> Command<R> for StreamScreen
where
    R: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    async fn execute(
        &self,
        client: &mut CoreClient<R>,
        args: HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let request = match args.get("request") {
            Some(value) => value,
            None => {
                send_response(client, "ERR", "Missing 'request' parameter").await?;
                return Ok(());
            }
        };

        match request.as_str() {
            "start" => {
                let config = match StreamConfig::from_args(&args) {
                    Ok(config) => config,
                    Err(e) => {
                        send_response(client, "ERR", &format!("Config error: {}", e)).await?;
                        return Ok(());
                    }
                };

                let start_result = {
                    let mut streamer =
                        SCREEN_STREAMER.lock().map_err(|_| anyhow::anyhow!("Mutex poisoned"))?;
                    streamer.configure(config);
                    streamer.start()
                };

                if let Err(e) = start_result {
                    send_response(client, "ERR", &format!("Failed to start stream: {}", e)).await?;
                }
            }
            "stop" => {
                let streamer =
                    SCREEN_STREAMER.lock().map_err(|_| anyhow::anyhow!("Mutex poisoned"))?;
                let _ = streamer.stop();
            }
            "state" => {
                let state = {
                    let streamer =
                        SCREEN_STREAMER.lock().map_err(|_| anyhow::anyhow!("Mutex poisoned"))?;
                    streamer.state()
                };

                send_response(client, "OK", &format!("{:?}", state)).await?;
            }
            _ => {
                send_response(client, "ERR", &format!("Unknown request type: {}", request)).await?;
            }
        }

        Ok(())
    }
}
