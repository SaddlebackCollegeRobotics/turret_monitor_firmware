use serde::{Deserialize, Serialize};

#[repr(u32)]
#[derive(Deserialize, Serialize, Debug)]
pub enum RequestKind {
    Default = 0,
    Telemetry = 1,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    kind: RequestKind,
}
