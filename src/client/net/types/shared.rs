#[derive(Clone)]
pub struct MessagePayload {
    pub length: usize,
    pub message: String,
}

impl MessagePayload {
    pub fn to_string(&self) -> String {
        let result = format!("m {}\n{}", self.message.len(), self.message);
        result
    }
}
