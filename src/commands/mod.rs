use crate::client::coreclient::CoreClient;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncWrite};

pub mod hello;

#[async_trait]
pub trait Command<R>: Send + Sync
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    fn help(&self) -> String;
    async fn execute(&self, client: &mut CoreClient<R>, args: &str) -> anyhow::Result<()>;
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
        commands.insert(hello::NAME.to_string(), Box::new(hello::Msg));
        
        Self { commands }
    }
}
