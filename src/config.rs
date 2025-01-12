use lazy_static::lazy_static;
use uuid::{uuid, Uuid};

lazy_static! {
    pub static ref UUID: Uuid = Uuid::new_v4();
}