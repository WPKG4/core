use std::str::from_utf8;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::{self, Instant};
use tracing::{debug, info};

use crate::client::net::types::r#in::payloads::{InActionPayload, InPayloadType};
use crate::client::net::types::out::payloads::OutPayloadType;
use crate::client::net::types::shared::{BinaryPayload, MessagePayload};
use crate::config::PING_INTERVAL;

pub(crate) struct WtpClient<R: AsyncRead + AsyncWrite + Unpin> {
    stream: R,
    last_action: Instant,
}

impl<R> WtpClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn new(stream: R) -> Self {
        WtpClient { stream, last_action: Instant::now() }
    }

    pub async fn send_packet(&mut self, payload_type: OutPayloadType) -> Result<()> {
        match payload_type {
            OutPayloadType::Action(payload) => {
                self.stream.write_all(payload.to_string().into_bytes().as_slice()).await?;
            }
            OutPayloadType::Message(payload) => {
                self.stream.write_all(payload.to_string().into_bytes().as_slice()).await?;
            }
            OutPayloadType::Binary(payload) => {
                self.stream
                    .write_all(format!("b {}\n", payload.bytes.len()).into_bytes().as_slice())
                    .await?;
                self.stream.write_all(&payload.bytes).await?;
            }
        };

        Ok(())
    }

    pub async fn read_packet(&mut self) -> Result<InPayloadType> {
        let mut header = Vec::new();

        loop {
            let mut buf = [0; 1];
            let result = time::timeout(Duration::from_secs(5), self.stream.read(&mut buf)).await;

            match result {
                Ok(Ok(n)) if n > 0 => {
                    self.last_action = Instant::now();

                    if buf[0] == b'\n' {
                        break;
                    }

                    header.push(buf[0]);
                }
                Ok(Ok(0)) => {
                    return Err(anyhow::anyhow!("Connection closed"));
                }
                Ok(Err(e)) => {
                    return Err(anyhow::anyhow!("{}", e));
                }
                Err(_) => {
                    if self.last_action.elapsed() >= *PING_INTERVAL {
                        self.stream.write_all(b"p\n").await.context("Failed to send ping")?;
                        debug!("Sent 'p' to the server after 5 minutes of inactivity");
                        self.last_action = Instant::now();
                    }
                }
                _ => {}
            }
        }

        let header = from_utf8(&header).context("Failed to parse header")?;
        debug!("Parsed header: {}", header);

        match header.chars().next() {
            Some('a') => self.parse_action_payload(header).await,
            Some('m') => self.parse_message_payload(header).await,
            Some('b') => self.parse_binary_payload(header).await,
            _ => Err(anyhow::anyhow!("Unimplemented header type")),
        }
    }

    async fn parse_action_payload(&mut self, header: &str) -> Result<InPayloadType> {
        let parts: Vec<&str> = header.split_whitespace().collect();
        let (error_code, action_name, len) = (
            parts.get(2).context("Missing error code")?,
            parts.get(1).context("Missing action name")?,
            parts.get(3).context("Missing length")?.parse::<usize>().context("Invalid length")?,
        );

        let bytes = self.read_exact_bytes(len).await?;
        let message = from_utf8(&bytes)?;
        info!("<ACTION PAYLOAD> {} \"{}\" len {}: {}", error_code, action_name, len, message);

        Ok(InPayloadType::Action(InActionPayload {
            error: error_code.to_string(),
            name: action_name.to_string(),
            message: message.to_string(),
        }))
    }

    async fn parse_message_payload(&mut self, header: &str) -> Result<InPayloadType> {
        let parts: Vec<&str> = header.split_whitespace().collect();
        let len =
            parts.get(1).context("Missing length")?.parse::<usize>().context("Invalid length")?;

        let bytes = self.read_exact_bytes(len).await?;
        let message = from_utf8(&bytes)?;
        info!("<MESSAGE PAYLOAD>: len={}, message={}", len, message);

        Ok(InPayloadType::Message(MessagePayload { message: message.to_string() }))
    }

    async fn parse_binary_payload(&mut self, header: &str) -> Result<InPayloadType> {
        let parts: Vec<&str> = header.split_whitespace().collect();
        let len =
            parts.get(1).context("Missing length")?.parse::<usize>().context("Invalid length")?;

        let bytes = self.read_exact_bytes(len).await?;
        let message = from_utf8(&bytes)?;
        info!("<BINARY PAYLOAD>: len={}, message={}", len, message);

        Ok(InPayloadType::Binary(BinaryPayload { bytes }))
    }

    async fn read_exact_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; len];
        self.stream.read_exact(&mut buf).await.context("Failed to read exact bytes")?;

        Ok(buf)
    }
}
