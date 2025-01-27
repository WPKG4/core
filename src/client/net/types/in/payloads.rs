use crate::client::net::types::shared::MessagePayload;

pub enum InPayloadType {
    Action(InActionPayload),
    Message(MessagePayload),
}

pub struct InActionPayload {
    pub(crate) error: String,
    pub(crate) name: String,
    pub(crate) message: String,
}
