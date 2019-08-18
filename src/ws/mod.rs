use serde::{Serialize, Deserialize};

#[cfg(target_arch="wasm32")]
pub mod wasm;

#[cfg(target_arch="wasm32")]
pub type ClientImpl = wasm::Client;

#[cfg(not(target_arch="wasm32"))]
pub mod native;

#[cfg(not(target_arch="wasm32"))]
pub type ClientImpl = native::Client;

pub enum SocketError {
    Create,
    // Read,
    // Write,
}


#[derive(Deserialize, Debug)]
pub struct WireState {
    s: String,
}

#[derive(Deserialize, Debug)]
pub struct WireStatePacket {
    pub seq: (Option<u32>, u32),
    pub data: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub enum WireClientEvent {
    Ack(u32),
}

