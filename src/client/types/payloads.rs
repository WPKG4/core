use std::collections::HashMap;

pub enum OutPayloadType {
    Action(OutActionPayload),
    Message(MessagePayload)
}

pub enum InPayloadType {
    Message(MessagePayload)
}

pub struct InActionPayload {
    error: String,
    name: String,
    message_length: usize,
    message: String
}

#[derive(Clone)]
pub struct OutActionPayload {
    pub(crate) name: String,
    pub(crate) parameters: HashMap<String, String>,
}

impl OutActionPayload {
    pub fn to_string(&self) -> String {
        let mut result = format!("a {}\n", self.name);
        for (key, value) in &self.parameters {
            result.push_str(&format!("{}: {}\n", key, value));
        }
        result.push('\n');
        result
    }
}

#[derive(Clone)]
pub struct MessagePayload {
    pub(crate) message: String,
}

impl MessagePayload {
    pub fn to_string(&self) -> String {
        let result = format!("m {}\n{}", self.message.len(), self.message);
        result
    }
}