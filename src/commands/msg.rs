use std::collections::HashMap;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::client::net::types::out::payloads::OutPayloadType;
use crate::client::net::types::shared::MessagePayload;
use crate::client::wpkgclient::coreclient::CoreClient;
use crate::commands::Command;

pub struct Msg;

pub(crate) const NAME: &str = "msg";

#[async_trait]
impl<R> Command<R> for Msg
where
    R: AsyncRead + AsyncWrite + Unpin + Send,
{
    #[allow(unused_variables)]
    async fn execute(
        &self,
        client: &mut CoreClient<R>,
        args: HashMap<String, String>,
    ) -> anyhow::Result<()> {
        client // TODO: wyświetl wiadomość w notatniku
            .wtp_client
            .send_packet(OutPayloadType::Message(MessagePayload::from_str(
                format!("world!, debug_params: {:?}", args).as_str(),
            )))
            .await?;
        Ok(())
    }
}
