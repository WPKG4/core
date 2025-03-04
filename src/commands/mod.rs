use std::collections::HashMap;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::client::wpkgclient::coreclient::CoreClient;

pub mod command;
pub mod msg;
pub mod fetchscreen;
pub mod utils;

#[async_trait]
pub trait Command<R>: Send + Sync
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    fn help(&self) -> String;
    async fn execute(
        &self,
        client: &mut CoreClient<R>,
        args: HashMap<String, String>,
    ) -> anyhow::Result<()>;
}

#[derive(Default)]
pub struct CommandsManager<R>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    pub commands: HashMap<String, Box<dyn Command<R>>>,
}

impl<R> CommandsManager<R>
where
    R: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new() -> Self {
        let mut commands: HashMap<String, Box<dyn Command<R>>> = HashMap::new();

        // Command definition
        commands.insert(msg::NAME.to_string(), Box::new(msg::Msg));
        commands.insert(fetchscreen::NAME.to_string(), Box::new(fetchscreen::Fetchscreen));

        Self { commands }
    }
}
