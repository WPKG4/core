use std::collections::HashMap;
use std::fmt;

use crate::client::net::types::shared::{BinaryPayload, MessagePayload};

pub enum OutPayloadType {
    Action(ActionPayload),
    Message(MessagePayload),
    Binary(BinaryPayload),
}

// ActionPayload {
#[derive(Clone)]
pub struct ActionPayload {
    pub(crate) name: String,
    pub(crate) parameters: HashMap<String, String>,
}

impl fmt::Display for ActionPayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "a {}", self.name)?;
        for (key, value) in &self.parameters {
            writeln!(f, "{}: {}", key, value)?;
        }
        writeln!(f)
    }
}
// }