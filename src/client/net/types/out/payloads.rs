use crate::client::net::types::shared::MessagePayload;
use std::collections::HashMap;
use std::fmt;

pub enum OutPayloadType {
    Action(OutActionPayload),
    Message(MessagePayload),
}

#[derive(Clone)]
pub struct OutActionPayload {
    pub(crate) name: String,
    pub(crate) parameters: HashMap<String, String>,
}

impl fmt::Display for OutActionPayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "a {}", self.name)?;
        for (key, value) in &self.parameters {
            writeln!(f, "{}: {}", key, value)?;
        }
        writeln!(f)
    }
}
