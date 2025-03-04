use crate::client::net::types::shared::{BinaryPayload, MessagePayload};

pub enum InPayloadType {
    Action(ActionResponse),
    Message(MessagePayload),
    Binary(BinaryPayload),
}

pub struct ActionResponse {
    pub(crate) error: String,
    pub(crate) name: String,
    pub(crate) message: String,
}
