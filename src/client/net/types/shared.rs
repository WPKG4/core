use std::fmt;

#[derive(Clone)]
pub struct MessagePayload {
    pub message: String
}

impl fmt::Display for MessagePayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "m {}\n{}", self.message.len(), self.message)
    }
}
