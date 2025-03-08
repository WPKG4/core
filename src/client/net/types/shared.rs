use std::fmt;

#[derive(Clone)]
pub struct MessagePayload {
    pub message: String,
}

impl MessagePayload {
    pub fn from_str(message: &str) -> MessagePayload {
        MessagePayload { message: message.to_string() }
    }
}

impl fmt::Display for MessagePayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "m {}\n{}", self.message.len(), self.message)
    }
}

pub struct BinaryPayload {
    pub bytes: Vec<u8>,
}
