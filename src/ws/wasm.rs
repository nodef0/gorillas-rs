use crate::ws::{SocketError, WireClientEvent};
use stdweb::console;
use stdweb::web::{
    event::{IMessageEvent, SocketMessageData, SocketMessageEvent},
    IEventTarget, SocketBinaryType, WebSocket,
};

use std::sync::mpsc::Sender;

pub struct Client {
    socket: WebSocket,
}

impl Client {
    pub fn new(url: &str, tx: Sender<Vec<u8>>) -> Result<Self, SocketError> {
        match WebSocket::new_with_protocols(url, &[&"rust-websocket"]) {
            Ok(socket) => {
                socket.set_binary_type(SocketBinaryType::ArrayBuffer);
                socket.add_event_listener(move |event: SocketMessageEvent| match event.data() {
                    SocketMessageData::Text(msg) => {
                        console!(log, msg);
                    }
                    SocketMessageData::Blob(_) => {
                        console!(log, "Blob");
                    }
                    SocketMessageData::ArrayBuffer(msg) => {
                        if let Err(_) = tx.send(msg.into()) {
                            console!(log, "Channel Error");
                        }
                    }
                });
                Ok(Client { socket })
            }
            Err(error) => {
                let err = format!("Error creating socket {:?}", error);
                console!(error, err);
                Err(SocketError::Create)
            }
        }
    }

    pub fn send_text(&self, text: &str) {
        if self.socket.send_text(text).is_err() {
            let err = format!("Error sending {}", text);
            console!(log, err)
        }
    }

    pub fn on_console(&self, s: &str) {
        console!(log, String::from(s));
    }

    pub fn send_ack(&self, seq: u32) {
        let event = WireClientEvent::Ack(seq);
        let bytes = bincode::serialize(&event).unwrap();
        if self.socket.send_bytes(&bytes[..]).is_err() {
            let err = format!("Error acking {}", seq);
            console!(log, err)
        }
    }
}
