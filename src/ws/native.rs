use crate::ws::{GameEvents, SocketError};
use std::sync::mpsc::Sender;

pub struct Client {}

impl Client {
    pub fn new(_url: &str, _tx: Sender<Vec<u8>>) -> Result<Self, SocketError> {
        Err(SocketError::Create)
    }

    pub fn on_console(&self, _s: &str) {}
    pub fn send_ack(&self, _seq: u32) {}
    pub fn send_game_event(&self, event: GameEvents) {}
}