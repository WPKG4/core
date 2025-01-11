use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::str::from_utf8;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use tokio::time::Instant;

enum HandlerState {
    ReadingHeader,
    ReadingPayload(HeaderType),
    Logic(PayloadType),
}

enum HeaderType {
    Action,
    Message
}

enum PayloadType {
    Action(ActionPayload),
    Message(MessagePayload)
}

#[derive(Clone)]
struct ActionPayload {
    parameters: HashMap<String, String>,
}

struct MessagePayload {
    message: String,
}

pub(crate) struct TcpClient {
    stream: TcpStream,
    state: HandlerState,
    last_action: Instant,
    buffer: Vec<u8>,
}

impl TcpClient {
    pub(crate) fn new(stream: TcpStream) -> Self {
        TcpClient {
            stream,
            state: HandlerState::ReadingHeader,
            last_action: Instant::now(),
            buffer: Vec::new(),
        }
    }

    pub(crate) async fn process(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match &self.state {
                HandlerState::ReadingHeader => {
                    self.read_header().await?;
                }
                HandlerState::ReadingPayload(HeaderType::Action) => {
                    self.read_action().await?;
                }
                HandlerState::ReadingPayload(HeaderType::Message) => {
                    todo!()
                }
                HandlerState::Logic(PayloadType::Action(payload)) => {
                    self.handle_action(payload.clone()).await?;
                }
                HandlerState::Logic(PayloadType::Message(payload)) => {
                    todo!()
                }
            }
        }
    }

    async fn read_header(&mut self) -> Result<(), Box<dyn Error>> {
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
                        println!("Sent 'p' to the server after 5 seconds of inactivity");
                        self.last_action = Instant::now();
                    }
                }
                _ => {}
            }
        }

        let cloned_buffer = self.buffer.to_vec();
        let header = from_utf8(&cloned_buffer)?;
        println!("Received header: {}", header);

        self.buffer.clear();
        self.state = match header.chars().next() {
            None => return Err("Header is empty".into()),
            Some(char) => {
                match char {
                    'm' => HandlerState::ReadingPayload(HeaderType::Message),
                    'a' => HandlerState::ReadingPayload(HeaderType::Action),
                    _ => { return Err(format!("Invalid header: {}", header).into()); }
                }
            }
        };

        Ok(())
    }

    async fn read_action(&mut self) -> Result<(), Box<dyn Error>> {
        let mut buf = vec![0; 1];

        while let Ok(n) = self.stream.read(&mut buf).await {
            if n == 0 {
                return Err("Connection closed".into());
            }

            self.buffer.push(buf[0]);

            if self.buffer.windows(2).position(|w| w == b"\n\n").is_some() {
                break;
            }
        }

        let cloned_buffer = self.buffer.to_vec();
        let action = from_utf8(&cloned_buffer)?;

        self.buffer.clear();
        self.state = HandlerState::Logic(PayloadType::Action(ActionPayload { parameters: parse_parameters(action) }));

        Ok(())
    }
    
    async fn handle_action(&mut self, payload: ActionPayload) -> Result<(), Box<dyn Error>> {
        println!("handler: {:?}", payload.parameters);

        let response = "Action processed\n\n";

        self.stream.write_all(response.as_bytes()).await?;
        
        self.state = HandlerState::ReadingHeader;
        
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
