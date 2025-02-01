use crate::client::net::types::shared::{BinaryPayload, MessagePayload};

pub enum InPayloadType {
    Action(InActionPayload),
    Message(MessagePayload),
    Binary(BinaryPayload),
}

pub struct InActionPayload {
    pub(crate) error: String,
    pub(crate) name: String,
    pub(crate) message: String,
}
