use quicksilver::geom::Vector;
use serde::Deserialize;

#[derive(Copy, Clone, Deserialize, Debug)]
pub enum Side {
    Left,
    Right,
}

#[derive(Deserialize, Debug)]
pub struct Explosion {
    pub pos: Vector,
    pub frame: u32,
}
