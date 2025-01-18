use std::collections::HashMap;

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tracing::{debug, error, info};

use crate::client::net::tls::tls_stream;
use crate::client::net::types::r#in::payloads::InPayloadType;
use crate::client::net::types::out::payloads::{OutActionPayload, OutPayloadType};
use crate::client::net::wtp::WtpClient;
use crate::commands::CommandsManager;
use crate::config::UUID;

pub(crate) struct CoreClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) wtp_client: WtpClient<R>,
}

impl CoreClient<TcpStream> {
    pub async fn new(ip: &str) -> Result<Self> {
        Ok(CoreClient { wtp_client: WtpClient::new(TcpStream::connect(ip).await?) })
    }
}
impl CoreClient<TlsStream<TcpStream>> {
    pub async fn new_tls(ip: &str) -> Result<Self> {
        Ok(CoreClient { wtp_client: WtpClient::new(tls_stream(ip).await?) })
    }
}
impl<R> CoreClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub async fn register(&mut self) -> Result<()> {
        self.wtp_client
            .send_packet(OutPayloadType::Action(OutActionPayload {
                name: "core-init".to_string(),
                parameters: HashMap::from([("uuid".to_string(), UUID.to_string())]),
            }))
            .await?;
        match self.wtp_client.read_packet().await? {
            InPayloadType::Action(action) => {
                if action.error == "OK" {
                    info!("Core client registered successfully");
                } else {
                    error!("Core client could not register successfully: {}", action.message);
                }
            }
            InPayloadType::Message(_) => {
                error!("Core client received unexpected message");
            }
        }
        Ok(())
    }

    pub async fn handle(&mut self) -> Result<()> {
        let commands: CommandsManager<R> = CommandsManager::new();
        loop {
            match self.wtp_client.read_packet().await? {
                InPayloadType::Action(action) => {
                    debug!("Client received action: {}", action.name);
                }
                InPayloadType::Message(message) => {
                    debug!("Client received message: {}", message.message);
                    commands
                        .commands
                        .get(&message.message)
                        .ok_or_else(|| {
                            anyhow::anyhow!("Could not find command! {}", message.message)
                        })?
                        .execute(self, "test")
                        .await?;
                }
            }
        }
    }
}
