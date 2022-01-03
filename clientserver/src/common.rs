use serde::{Deserialize, Serialize};

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



