use std::error::Error;
use tokio::net::TcpStream;

mod client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:5000";
    let stream = TcpStream::connect(addr).await?;
    println!("Connected to server at {}", addr);

    let mut client = crate::client::wtp::TcpClient::new(stream);
    client.process().await?;

    Ok(())
}