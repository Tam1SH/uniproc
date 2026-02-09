use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub enum Request {
    Ping,
    GetIdentity,
    GetProcesses { limit: u32 },
    KillProcess { pid: u32 },
    StreamStats { enable: bool },
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub enum Response {
    Pong,
    Identity(String),
    ProcessList(Vec<u32>),
    Success,
    Error(String),
}

#[derive(Archive, Deserialize, Serialize, Debug)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub enum Message {
    Request { id: u64, payload: Request },
    Response { id: u64, payload: Response },
}
