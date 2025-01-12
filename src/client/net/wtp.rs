use crate::client::net::types::out::payloads::OutPayloadType;
use crate::client::net::types::shared::MessagePayload;
use crate::config::PING_INTERVAL;
use anyhow::{Context, Result};
use std::str::from_utf8;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time;
use tokio::time::Instant;
use tracing::{debug, info};
use crate::client::net::types::r#in::payloads::{InActionPayload, InPayloadType};

pub(crate) struct WtpClient<R: AsyncRead + AsyncWrite + Unpin> {
    stream: R,
    last_action: Instant,
}

impl<R> WtpClient<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) async fn new(stream: R) -> Self {
        WtpClient {
            stream,
            last_action: Instant::now(),
        }
    }

    pub async fn send_packet(&mut self, payload_type: OutPayloadType) -> Result<InPayloadType> {
        match payload_type {
            OutPayloadType::Action(payload) => {
                self.stream
                    .write_all(payload.to_string().as_bytes())
                    .await
                    .context("Failed to send action payload")?;
                self.read_packet().await
            }
            OutPayloadType::Message(payload) => {
                self.stream
                    .write_all(payload.to_string().as_bytes())
                    .await
                    .context("Failed to send message payload")?;
                self.read_packet().await
            }
        }
    }

    pub async fn read_packet(&mut self) -> Result<InPayloadType> {
        let mut buf = vec![0; 1];
        let mut header = Vec::new();

        loop {
            let timeout = Duration::from_secs(5);
            let result = time::timeout(timeout, self.stream.read(&mut buf)).await;

            match result {
                Ok(Ok(n)) if n > 0 => {
                    self.last_action = Instant::now();

                    if buf[0] == b'\n' {
                        break;
                    }

                    header.push(buf[0]);
                }
                Ok(Ok(0)) => {
                    return Err(anyhow::anyhow!("Connection closed").into());
                }
                Ok(Err(e)) => {
                    return Err(anyhow::anyhow!("{}", e).into());
                }
                Err(_) => {
                    if self.last_action.elapsed() >= *PING_INTERVAL {
                        self.stream
                            .write_all(b"p\n")
                            .await
                            .context("Failed to send ping")?;
                        debug!("Sent 'p' to the server after 5 minutes of inactivity");
                        self.last_action = Instant::now();
                    }
                }
                _ => {}
            }
        }

        let header = from_utf8(&header).context("Failed to parse header from bytes")?;

        match header.chars().next() {
            None => Err(anyhow::anyhow!("Invalid header!").into()),
            Some(char) => match char {
                'a' => {
                    let header_values: Vec<&str> = header.split_whitespace().collect();

                    let error_code = header_values.get(2).ok_or_else(|| {
                        anyhow::anyhow!("Invalid header! Error code not found.")
                    })?;

                    let action_name = header_values.get(1).ok_or_else(|| {
                        anyhow::anyhow!("Invalid header! Action name not found.")
                    })?;

                    let len = header_values.get(3).ok_or_else(|| {
                        anyhow::anyhow!("Invalid header! Length not found.")
                    })?;

                    let len: usize = len.parse().map_err(|_| {
                        anyhow::anyhow!("Invalid header! Length is not a valid number.")
                    })?;

                    let message = self.read_exact_bytes(len).await?;

                    debug!("<ACTION PAYLOAD> {} \"{}\" len {}: {}", error_code, action_name, len, message);

                    Ok(InPayloadType::Action(InActionPayload {
                        error: error_code.to_string(),
                        message,
                        name: action_name.to_string(),
                        message_length: len,
                    }))
                }
                'm' => {
                    let header_values: Vec<&str> = header.split_whitespace().collect();

                    let len = header_values.get(2).ok_or_else(|| {
                        anyhow::anyhow!("Invalid header! Length not found.")
                    })?;

                    let len: usize = len.parse().map_err(|_| {
                        anyhow::anyhow!("Invalid header! Length is not a valid number.")
                    })?;

                    let message = self.read_exact_bytes(len).await?;

                    Ok(InPayloadType::Message(MessagePayload {
                        length: len,
                        message,
                    }))
                }
                _ => Err(anyhow::anyhow!("Unimplemented header type!").into()),
            },
        }
    }

    async fn read_exact_bytes(&mut self, len: usize) -> Result<String> {
        let mut buf = Vec::with_capacity(len);

        while buf.len() < len {
            let remaining_len = len - buf.len();
            let mut temp_buf = vec![0; remaining_len];

            self.stream.read_exact(&mut temp_buf).await
                .context("Failed to read exact bytes")?;

            buf.extend_from_slice(&temp_buf);
        }

        let message = from_utf8(&buf).context("Failed to parse message from bytes")?;

        Ok(message.to_string())
    }
}
