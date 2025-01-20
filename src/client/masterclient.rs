use std::collections::HashMap;

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tracing::{debug, error, info};
use whoami::fallible::{hostname, username};

use crate::client::coreclient::CoreClient;
use crate::client::net::tls::tls_stream;
use crate::client::net::types::r#in::payloads::InPayloadType;
use crate::client::net::types::out::payloads::{OutActionPayload, OutPayloadType};
use crate::client::net::wtp::WtpClient;
use crate::config;

pub(crate) struct MasterClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) wtp_client: WtpClient<R>,
}

impl MasterClient<TcpStream> {
    pub async fn new(ip: &str) -> Result<Self> {
        Ok(MasterClient { wtp_client: WtpClient::new(TcpStream::connect(ip).await?) })
    }
}
impl MasterClient<TlsStream<TcpStream>> {
    pub async fn new_tls(ip: &str) -> Result<Self> {
        Ok(MasterClient { wtp_client: WtpClient::new(tls_stream(ip).await?) })
    }
}
impl<R> MasterClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub async fn register(&mut self) -> Result<()> {
        self.wtp_client
            .send_packet(OutPayloadType::Action(OutActionPayload {
                name: "core-init".to_string(),
                parameters: HashMap::from([
                    ("uuid".to_string(), config::get_config("UUID").await?),
                    ("user".to_string(), username().unwrap_or("UNKNOWN".to_string())),
                    ("hostname".to_string(), hostname().unwrap_or("UNKNOWN".to_string())),
                ]),
            }))
            .await?;
        match self.wtp_client.read_packet().await? {
            InPayloadType::Action(action) => {
                if action.error == "OK" {
                    info!("Master Client registered successfully");
                } else {
                    error!("Master Client could not register successfully: {}", action.message);
                    return Err(anyhow::anyhow!(
                        "Master Client could not register successfully: {}",
                        action.message
                    ));
                }
            }
            InPayloadType::Message(_) => {
                error!("Master Client received unexpected message");
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
                    if message.message.starts_with("NEW") {
                        tokio::spawn(async move {
                            let mut core_client =
                                CoreClient::new(config::IP).await.expect("Client crashed!");
                            core_client.register().await.expect("Could not register client!");
                            core_client.handle().await.expect("Handler crashed!");
                        });
                    }
                }
            }
        }
    }
}
