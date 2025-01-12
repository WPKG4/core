use std::error::Error;
use tokio::net::TcpStream;
use figlet_rs::FIGfont;
use tracing::info;

mod client;
mod logger;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logger::init();
    let standard_font = FIGfont::standard()?;
    let figure = standard_font.convert("WPKG4 - szybkie i zajebiste");
    println!("{}", figure.unwrap());
    
    let addr = "127.0.0.1:5000";
    let stream = TcpStream::connect(addr).await?;
    info!("Connected to server at {}", addr);

    let mut client = crate::client::wtp::WtpClient::new(stream).await;
    client.process().await?;

    Ok(())
}