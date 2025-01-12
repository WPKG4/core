use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::str::from_utf8;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use tokio::time::Instant;
use tracing::{debug, info};
use tracing_subscriber::fmt::format;
use whoami::{fallible, hostname, username};
use crate::client::types::headers::HeaderType;
use crate::client::types::payloads::{InPayloadType, MessagePayload, OutActionPayload, OutPayloadType};
use crate::config::UUID;
enum HandlerState {
    ReadingHeader,
    ReadingPayload(HeaderType),
    Logic(InPayloadType),
}

pub(crate) struct WtpClient {
    stream: TcpStream,
    state: HandlerState,
    last_action: Instant,
    buffer: Vec<u8>,
}

impl WtpClient {
    pub(crate) async fn new(stream: TcpStream) -> Self {
        let mut client = WtpClient {
            stream,
            state: HandlerState::ReadingHeader,
            last_action: Instant::now(),
            buffer: Vec::new(),
        };
        match client.send_packet(OutPayloadType::Action(OutActionPayload {
            name: "core-init".to_string(),
            parameters: HashMap::from([
                ("uuid".to_string(), UUID.to_string()),
                ("user".to_string(), username()),
                ("hostname". to_string(), fallible::hostname().unwrap_or("UNKNOWN".to_string()))
            ]),
        })).await {
            Ok(msg) => {
                info!("Client successfully registered {}", msg);
            }
            Err(e) => {
                info!("Client error: {}", e);
            }
        }
        client.state = HandlerState::ReadingHeader;
        client
    }

    pub(crate) async fn process(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match &self.state {
                HandlerState::ReadingHeader => {
                    self.read_header().await?;
                }
                HandlerState::ReadingPayload(HeaderType::Message(len)) => {
                    self.read_exact_bytes(*len).await?;
                }
                HandlerState::Logic(InPayloadType::Message(payload)) => {
                    self.handle_message(payload.clone()).await?
                }
            }
        }
    }

    async fn send_packet(&mut self, payload_type: OutPayloadType) -> Result<String, Box<dyn Error>> {
        match payload_type {
            OutPayloadType::Action(payload) => {
                self.stream.write_all(payload.to_string().as_bytes()).await?;
                
                let header = self.read_header().await?;
                let header_values: Vec<&str> = header.split_whitespace().collect();
                
                let len = match header_values.get(3) {
                    Some(&len) => len,
                    None => return Err("Invalid header! Length not found.".into()),
                };
                
                let len: usize = len
                    .parse()
                    .map_err(|_| "Invalid header! Length is not a valid number.")?;
                
                let message = self.read_exact_bytes(len).await?;
                
                let error_code = match header_values.get(2) {
                    Some(&code) => code,
                    None => return Err("Invalid header! Error code not found.".into()),
                };
                
                match error_code {
                    "ERR" => Err(message.into()),
                    "OK" => Ok(message),
                    _ => Err("Invalid error code.".into()),
                }
            }
            OutPayloadType::Message(payload) => {
                self.stream.write_all(payload.to_string().as_bytes()).await?;
                Ok("Message sent.".to_string())
            }
        }
    }


    async fn read_header(&mut self) -> Result<String, Box<dyn Error>> {
        let mut buf = vec![0; 1];
        loop {
            let timeout = Duration::from_secs(5);
            let result = time::timeout(timeout, self.stream.read(&mut buf)).await;

            match result {
                Ok(Ok(n)) if n > 0 => {
                    self.last_action = Instant::now();

                    if buf[0] == b'\n' {
                        break;
                    }

                    self.buffer.push(buf[0]);
                }
                Ok(Ok(0)) => {
                    return Err("Connection closed".into());
                }
                Ok(Err(e)) => {
                    return Err(Box::new(e));
                }
                Err(_) => {
                    if self.last_action.elapsed() >= Duration::from_secs(5*60) {
                        self.stream.write_all(b"p\n").await?;
                        debug!("Sent 'p' to the server after 5 minutes of inactivity");
                        self.last_action = Instant::now();
                    }
                }
                _ => {}
            }
        }

        let cloned_buffer = self.buffer.to_vec();
        let header = from_utf8(&cloned_buffer)?;
        debug!("Received header: {}", header);

        self.buffer.clear();
        self.state = match header.chars().next() {
            None => return Err("Header is empty".into()),
            Some(char) => {
                match char {
                    'a' => {
                        let len = match header.split(" ").nth(3) {
                            Some(len) => len,
                            None => return Err("Invaild header! Length not found!".into()),
                        };
                        HandlerState::ReadingPayload(HeaderType::Message(len.parse()?))
                    },
                    _ => { return Err(format!("Invalid header: {}", header).into()); }
                }
            }
        };

        Ok(header.to_string())
    }

    async fn read_exact_bytes(&mut self, len: usize) -> Result<String, Box<dyn Error>> {
        let mut buf = vec![0; len];

        self.stream.read_exact(&mut buf).await?;

        let cloned_buffer = self.buffer.to_vec();
        let message = from_utf8(&cloned_buffer)?;

        self.buffer.clear();
        self.state = HandlerState::Logic(InPayloadType::Message(MessagePayload { message: message.to_string() }));

        Ok(message.to_string())
    }
    
    async fn handle_message(&mut self, message: MessagePayload) -> Result<(), Box<dyn Error>> {
        info!("Received message: {}", message.message);
        Ok(())
    }
}

fn parse_parameters(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    
    for line in input.lines() {
        if let Some((key, value)) = line.split_once(':') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}
