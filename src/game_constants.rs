use quicksilver::geom::Vector;

pub const BUILD_PALETTE: [[u8; 3]; 4] = [
    [0x22, 0xa0, 0xb6],
    [0xcb, 0x0c, 0x59],
    [0xbb, 0xbb, 0xbb],
    [0x24, 0x24, 0x24],
];

pub const SKY_BLUE_DARK: [u8; 4] = [0x09, 0x18, 0x33, 0xFF];
pub const SKY_BLUE_LIGHT: [u8; 4] = [0x13, 0x3e, 0x7c, 0xFF];
pub const SKY_CUTS: u32 = 4;
pub const WINDOW_X: f32 = 800.0;
pub const WINDOW_Y: f32 = 600.0;
pub const CENTER: (f32, f32) = (WINDOW_X / 2.0, WINDOW_Y / 2.0);
pub const GRAVITY: f32 = 0.012;
pub const DELTAT_MS: f32 = 16.667;
pub const GORILLA_SIZE: (f32, f32) = (40.0, 60.0);
pub const SHOT_RADIUS: f32 = 10.0;
pub const EXPLOSION_FRAMES: u32 = 12;
pub const EXPLOSION_SIZE: (u32, u32) = (96, 96);
pub const EXPLOSION_HALF_VEC: Vector = Vector { x: 48.0, y: 48.0 };
pub const POWER_MAX: i32 = 200;
pub const JUICE_COUNTER: f32 = 0.5;
pub const START_OFFSET: f32 = 60.0;
pub const END_OFFSET: f32 = 80.0;
pub const DISTANCE_MIN: usize = 2;
pub const PARTS_GRID_MIN: f32 = 4.0;
pub const TITLE: &str = "gorillas.rs";
pub const PLAY: &str = "Play [Enter]";
pub const BOT_DELAY_MAX: i32 = 60;
