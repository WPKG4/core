use figlet_rs::FIGfont;
use std::error::Error;
use tokio::net::TcpStream;
use tracing::info;
use crate::client::coreclient::CoreClient;

mod client;
mod config;
mod logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logger::init();
    let standard_font = FIGfont::standard()?;
    let figure = standard_font.convert("WPKG4 - szybkie i zajebiste");
    println!("{}", figure.unwrap());
    

    let mut client = CoreClient::new().await?;
    client.register().await?;

    Ok(())
}
