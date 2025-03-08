use std::collections::HashMap;
use std::str::from_utf8;

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tracing::{debug, error, info};
use whoami::fallible::{hostname, username};

use crate::client::wpkgclient::coreclient::CoreClient;
use crate::client::net::tls::tls_stream;
use crate::client::net::types::r#in::payloads::InPayloadType;
use crate::client::net::types::out::payloads::{ActionPayload, OutPayloadType};
use crate::client::net::types::shared::MessagePayload;
use crate::client::net::wtp::WtpClient;
use crate::commands::command::CommandPayload;
use crate::config;

pub(crate) struct MasterClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) wtp_client: WtpClient<R>,
}

impl MasterClient<TcpStream> {
    pub async fn from_tcp(ip: &str) -> Result<Self> {
        Ok(MasterClient { wtp_client: WtpClient::new(TcpStream::connect(ip).await?) })
    }
}
impl MasterClient<TlsStream<TcpStream>> {
    pub async fn from_tls(ip: &str) -> Result<Self> {
        Ok(MasterClient { wtp_client: WtpClient::new(tls_stream(ip).await?) })
    }
}
impl<R> MasterClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub async fn register(&mut self) -> Result<()> {
        self.wtp_client
            .send_packet(OutPayloadType::Action(ActionPayload {
                name: "core-init".to_string(),
                parameters: HashMap::from([
                    ("uuid".to_string(), config::get_config("uuid").await?),
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
            InPayloadType::Binary(_) => {
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
                    let command = CommandPayload::from(&message.message)?;
                    match command.name.as_str() {
                        "NEW" => {
                            tokio::spawn(async move {
                                let mut core_client = CoreClient::from_tcp(
                                    &config::get_config("ip")
                                        .await
                                        .expect("Could not get IP Addres!"),
                                )
                                .await
                                .expect("Client crashed!");
                                core_client.register().await.expect("Could not register client!");
                                core_client.handle().await.expect("Handler crashed!");
                            });
                        }
                        "DEL" => {
                            todo!("Delete client")
                        }
                        &_ => {
                            self.wtp_client
                                .send_packet(OutPayloadType::Message(MessagePayload::from_str(
                                    format!("Command {}, not found!", command.name).as_str(),
                                )))
                                .await?;
                        }
                    };
                }
                InPayloadType::Binary(binary_payload) => {
                    debug!("Received binary payload! {}", from_utf8(&binary_payload.bytes)?)
                }
            }
        }
    }
}
