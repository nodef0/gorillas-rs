use crate::ws::SocketError;

pub struct Client {

}

impl Client {
    pub fn new(_url: &str) -> Result<Self, SocketError> {
        Err(SocketError::Create)
    }

    pub fn send_text(&self, _text: &str) {
    }
}
