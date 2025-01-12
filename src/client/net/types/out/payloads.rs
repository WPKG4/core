use crate::client::net::types::shared::MessagePayload;
use std::collections::HashMap;

pub enum OutPayloadType {
    Action(OutActionPayload),
    Message(MessagePayload),
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
