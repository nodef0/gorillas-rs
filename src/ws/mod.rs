use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub type ClientImpl = wasm::Client;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(not(target_arch = "wasm32"))]
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

// Things that need to be sent over the wire
//
// pub struct SharedData {
//     parts: Vec<Vec<Rectangle>>,
// }
//
// struct Building {
//     bound_box: Rectangle,
//     tiles: Vec<usize>, // indicies into the tilemap
// }
//
// struct Explosion {
//     pos: Vector,
//     frame: u32,
// }
//
// struct RoundState {
//     buildings: Vec<Building>,
//     gorilla_left: Rectangle,
//     gorilla_right: Rectangle,
//     wind: Vector,
// }
//
// pub struct GameState {
//     round: Round,
//     counting: bool,
//     counter: i32,
//     explosion_pos: Option<Explosion>,
//     explosion_masks: Vec<Circle>,
//     shot: Option<(Circle, Vector, f32)>,
//     turn: Side,
//     points_left: u32,
//     points_right: u32,
//     new_game: Option<Side>,
//     mouse_pos: Vector,
// }
