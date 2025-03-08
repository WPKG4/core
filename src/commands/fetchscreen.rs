use std::collections::HashMap;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use super::utils::screen::fetch_screenshot;
use crate::client::net::types::out::payloads::OutPayloadType;
use crate::client::net::types::shared::{BinaryPayload, MessagePayload};
use crate::client::wpkgclient::coreclient::CoreClient;
use crate::commands::Command;
use crate::config::BINARY_SPLIT_SIZE;

pub struct FetchScreen;

pub(crate) const NAME: &str = "fetchscreen";

#[async_trait]
impl<R> Command<R> for FetchScreen
where
    R: AsyncRead + AsyncWrite + Unpin + Send,
{
    #[allow(unused_variables)]
    async fn execute(
        &self,
        client: &mut CoreClient<R>,
        args: HashMap<String, String>,
    ) -> anyhow::Result<()> {
        match fetch_screenshot() {
            Ok(jpeg) => {
                let metadata = HashMap::from([
                    ("name", "image.jpg".to_string()),
                    ("type", "image/jpeg".to_string()),
                    ("len", jpeg.len().to_string()),
                ]);
                client
                    .wtp_client
                    .send_packet(OutPayloadType::Message(MessagePayload::from_str(
                        serde_json::to_string(&metadata)?.as_str(),
                    )))
                    .await?;

                for chunk in jpeg.chunks(*BINARY_SPLIT_SIZE) {
                    client
                        .wtp_client
                        .send_packet(OutPayloadType::Binary(BinaryPayload {
                            bytes: chunk.to_vec(),
                        }))
                        .await?;
                }
            }
            Err(err) => {
                client
                    .wtp_client
                    .send_packet(OutPayloadType::Message(MessagePayload::from_str(
                        format!("ERR {} {}", err.to_string().len(), err.to_string()).as_str(),
                    )))
                    .await?;
            }
        }
        Ok(())
    }
}
