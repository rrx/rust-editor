use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    TestResponse,
    TestRequest(String),
    ServerShutdownReq,
    ServerShutdownResp,
    ServerRestartReq,
    ServerRestartResp,
    ProcessStartReq(String, Vec<String>),
    ProcessStartResp(Result<String, String>),
    ProcessStopReq(Vec<String>),
    ProcessStopResp,
    ProcessListReq,
    ProcessListResp(Vec<String>),
    Ack,
    Invalid(String)
}

#[derive(Debug)]
pub enum ServerMessage {
    Message(Message),
    Data(bytes::Bytes),
    EOF
}

#[derive(Debug)]
pub enum ServerCommand {
    Shutdown,
    Restart,
    Message(Message, Sender<ServerMessage>),
}


