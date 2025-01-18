use crate::client::coreclient::CoreClient;
use crate::client::net::types::out::payloads::OutPayloadType;
use crate::client::net::types::shared::MessagePayload;
use crate::commands::Command;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct Msg;

pub(crate) const NAME: &str = "hello";

#[async_trait]
impl<R> Command<R> for Msg
where
    R: AsyncRead + AsyncWrite + Unpin + Send,
{
    fn help(&self) -> String {
        "<message> - displays hello world".to_string()
    }
    #[allow(unused_variables)]
    async fn execute(&self, client: &mut CoreClient<R>, args: &str) -> anyhow::Result<()> {
        client
            .wtp_client
            .send_packet(OutPayloadType::Message(MessagePayload {
                length: 5,
                message: "world".to_string(),
            }))
            .await?;
        Ok(())
    }
}
