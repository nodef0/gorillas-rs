use quicksilver::geom::Vector;

pub const WINDOW_X: f32 = 800.0;
pub const WINDOW_Y: f32 = 600.0;
pub const CENTER: (f32, f32) = (WINDOW_X / 2.0, WINDOW_Y / 2.0);
pub const GRAVITY: f32 = 0.012;
pub const DELTAT_MS: f32 = 16.667;
pub const GORILLA_SIZE: (u32, u32) = (64, 96);
pub const SHOT_RADIUS: f32 = 10.0;
pub const EXPLOSION_FRAMES: u32 = 12;
pub const EXPLOSION_SIZE: (u32, u32) = (96, 96);
pub const EXPLOSION_HALF_VEC: Vector = Vector { x: 48.0, y: 48.0 };
pub const EXPLOSION_DESTROY_SCALE: f32 = 4.0;
pub const POWER_MAX: i32 = 200;
pub const JUICE_COUNTER: f32 = 0.5;
pub const START_OFFSET: f32 = 60.0;
pub const END_OFFSET: f32 = 80.0;
pub const DISTANCE_MIN: usize = 2;
pub const PARTS_GRID_MIN: f32 = 4.0;
pub const TITLE: &str = "gorillas.rs";
pub const PLAY: &str = "Play [Enter]";
pub const BOT_COUNTER_MAX: i32 = 200;
pub const BOT_AIM_INC_Y: f32 = 0.1;

pub const TILE_SIZE: (u32, u32) = (16, 16);

pub const PLAYER_PALETTE: [&str; 10] = [
    "4a5462", "4a5462", "4a5462", "4a5462", "b3b9d1", "b3b9d1", "b3b9d1", "b3b9d1", "fad6b8",
    "141013",
];

pub const PARTICLE_COUNT: usize = 1024;
pub const PARTICLE_MIN_VEL: f32 = 0.02;
pub const PARTICLE_MAX_VEL: f32 = 0.3;
pub const PARTICLE_SIZE: (u32, u32) = (2, 2);

//pub const WIND_ORIGIN: Vector  = Vector { x: 50.0, y: 50.0 };
pub const WIND_SHOW_RATIO: f32 = 5.0;
pub const WIND_PLAY_RATIO: f32 = 0.003;

pub const BANANA_SIZE: (u32, u32) = (20, 20); 
pub const BANANA_LOC: (u32, u32) = (0, 128);
pub const BANANA_ANG_SPEED: f32 = 5.0;
