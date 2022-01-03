use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    TestResponse,
    TestRequest(String),
    RequestServerShutdown,
    ResponseServerShutdown,
    RequestServerRestart,
    ResponseServerRestart,
    Ack
}



