use quicksilver::{
    geom::{Circle, Rectangle, Shape, Transform, Vector},
    graphics::{
        Background::{Col, Img},
        Color, Font, FontStyle, Image, PixelFormat,
    },
    input::{ButtonState, MouseButton},
    lifecycle::{run, Asset, Event, Settings, State, Window},
    Result,
};

use rand::prelude::*;
use std::cell::RefCell;

const BUILD_PALETTE: [[u8; 3]; 4] = [
    [0x22, 0xa0, 0xb6],
    [0xcb, 0x0c, 0x59],
    [0xbb, 0xbb, 0xbb],
    [0x24, 0x24, 0x24],
];

const SKY_BLUE_DARK: [u8; 4] = [0x09, 0x18, 0x33, 0xFF];
const SKY_BLUE_LIGHT: [u8; 4] = [0x13, 0x3e, 0x7c, 0xFF];
const SKY_CUTS: u32 = 4;

const WINDOW_X: u32 = 800;
const WINDOW_Y: u32 = 600;

const DELTAT_MS: f32 = 16.667;
// 9.81 m/s^2 * DELTA_MS
const GRAVITY: f32 = 0.00835;

const GORILLA_SIZE: (f32, f32) = (40.0, 60.0);
const SHOT_RADIUS: f32 = 10.0;

struct Game {
    counting: bool,
    counter: i32,
    sky: Image,
    font: RefCell<Asset<Font>>,
    font_style: FontStyle,
    buildings: Vec<(Rectangle, Color)>,
    gorilla_left: Rectangle,
    gorilla_right: Rectangle,
    shot: Option<(Circle, Vector)>, // pos, speed
    turn: Side,
    points_left: u32,
    points_right: u32,
}

fn cut(start: &[u8], end: &[u8], out: &mut [u8]) {
    for (i, e) in out.iter_mut().enumerate() {
        *e = (start[i] / 2 + end[i] / 2) + ((start[i] & 1) ^ (end[i] & 1));
    }
}

fn gen_cuts(times: u32, start: &[u8], end: &[u8]) -> Vec<u8> {
    if times == 0 {
        Vec::new()
    } else {
        let mut t = [0; 4];
        cut(start, end, &mut t);
        let mut p = gen_cuts(times - 1, start, &t);
        let mut n = gen_cuts(times - 1, &t, end);
        p.extend_from_slice(&t);
        p.append(&mut n);
        p
    }
}

fn buildings() -> Vec<(Rectangle, Color)> {
    let mut b = vec![];
    let mut rng = rand::thread_rng();
    let mut last_x = 0;
    loop {
        let height = rng.gen_range(300, 500);
        let width = 60 + 5 * rng.gen_range(0, 5);
        let color = BUILD_PALETTE[rng.gen_range(0, 4)];
        b.push((
            Rectangle::new((last_x, height), (width - 2, WINDOW_Y - height)),
            Color::from_rgba(color[0], color[1], color[2], 1.0),
        ));
        last_x += width;
        if last_x > WINDOW_X {
            break;
        }
    }
    b
}

// take the current position and speed, and updates them

fn update_shot(pos: Vector, speed: Vector) -> (Vector, Vector) {
    (
        Vector::new(pos.x + DELTAT_MS * speed.x, pos.y + DELTAT_MS * speed.y),
        Vector::new(speed.x, speed.y + GRAVITY),
    )
}

enum Side {
    Left,
    Right,
}

fn next_side(side: &Side) -> Side {
    match side {
        Side::Left => Side::Right,
        Side::Right => Side::Left,
    }
}

const DISTANCE_MIN: usize = 2;

fn place_gorilla(side: Side, buildings: &[(Rectangle, Color)]) -> Rectangle {
    let field_length = buildings.len() - 1;
    let mut rng = rand::thread_rng();
    let i = match side {
        Side::Left => rng.gen_range(1, (field_length / 2) - DISTANCE_MIN),
        Side::Right => rng.gen_range((field_length / 2) + DISTANCE_MIN, field_length),
    };
    let b = buildings[i].0;
    Rectangle::new(
        (
            b.pos.x + b.size.x / 2.0 - GORILLA_SIZE.0 / 2.0,
            b.pos.y - GORILLA_SIZE.1,
        ),
        GORILLA_SIZE,
    )
}

enum Collision {
    None,
    Sky,
    Building(usize),
    Player(Side),
}

fn collide_field(pos: Vector) -> bool {
    pos.x > WINDOW_X as f32 || pos.y > WINDOW_Y as f32 || pos.x < 0.0 // || pos.y < 0.0 => shot can go upwards the screen and come back
}

fn collide_buildings(circle: Circle, buildings: &[(Rectangle, Color)]) -> Option<usize> {
    for (i, (rect, _)) in buildings.iter().enumerate() {
        if circle.overlaps(rect) {
            return Some(i);
        }
    }
    None
}

fn collide_player(circle: Circle, player: &Rectangle) -> bool {
    circle.overlaps(player)
}

impl Game {
    fn gorilla_from_side(&self, side: &Side) -> &Rectangle {
        match side {
            Side::Left => &self.gorilla_left,
            Side::Right => &self.gorilla_right,
        }
    }

    fn collision(&self, circle: Circle) -> Collision {
        if collide_field(circle.pos) {
            Collision::Sky
        } else if let Some(i) = collide_buildings(circle, &self.buildings) {
            Collision::Building(i)
        } else if collide_player(circle, &self.gorilla_left) {
            Collision::Player(Side::Left)
        } else if collide_player(circle, &self.gorilla_right) {
            Collision::Player(Side::Right)
        } else {
            Collision::None
        }
    }
}

impl State for Game {
    fn new() -> Result<Game> {
        let sky_vec = gen_cuts(SKY_CUTS, &SKY_BLUE_DARK, &SKY_BLUE_LIGHT);
        let buildings = buildings();
        let gorilla_left = place_gorilla(Side::Left, &buildings);
        let gorilla_right = place_gorilla(Side::Right, &buildings);
        let font = RefCell::new(Asset::new(Font::load("UI.ttf")));
        Ok(Game {
            counting: false,
            counter: 0i32,
            sky: Image::from_raw(&sky_vec, 1, (2 << (SKY_CUTS - 1)) - 1, PixelFormat::RGBA)?,
            font,
            font_style: FontStyle::new(64.0, Color::WHITE),
            buildings,
            gorilla_left,
            gorilla_right,
            shot: None,
            turn: Side::Left,
            points_left: 0,
            points_right: 0,
        })
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::WHITE)?;
        window.draw_ex(
            &Rectangle::new((0, 0), window.screen_size()),
            Img(&self.sky),
            Transform::IDENTITY,
            0.0,
        );
        for b in self.buildings.iter() {
            window.draw_ex(&b.0, b.1, Transform::IDENTITY, 1.0);
        }
        // draw gorillas
        window.draw_ex(
            &self.gorilla_left,
            Col(Color::RED),
            Transform::IDENTITY,
            2.0,
        );
        window.draw_ex(
            &self.gorilla_right,
            Col(Color::RED),
            Transform::IDENTITY,
            2.0,
        );

        // draw shot
        if let Some((circle, _)) = self.shot {
            window.draw_ex(&circle, Col(Color::YELLOW), Transform::IDENTITY, 3.0);
        }

        // draw power bar
        window.draw_ex(
            &Rectangle::new((100, 500), (self.counter, 50)),
            Col(Color::RED),
            Transform::IDENTITY,
            3.0,
        );

        self.font.borrow_mut().execute(|f| {
            if let Ok(ref text) = f.render(
                &format!("{:02} {:02}", self.points_left, self.points_right),
                &self.font_style,
            ) {
                window.draw_ex(
                    &text.area().with_center((WINDOW_X / 2, 100)),
                    Img(text),
                    Transform::IDENTITY,
                    4.0,
                );
            } else {
                eprintln!("Failed to render score")
            }
            Ok(())
        })?;
        Ok(())
    }

    fn event(&mut self, event: &Event, window: &mut Window) -> Result<()> {
        match (event, self.counting, self.shot) {
            (Event::MouseButton(MouseButton::Left, ButtonState::Pressed), false, None) => {
                self.counting = true;
            }
            (Event::MouseButton(MouseButton::Left, ButtonState::Released), true, None) => {
                self.counting = false;
                let gorilla = self.gorilla_from_side(&self.turn);
                let dir = window.mouse().pos() - gorilla.pos;
                self.shot = Some((
                    Circle::new(
                        gorilla.pos + Vector::new(gorilla.size.x / 2.0, -SHOT_RADIUS),
                        SHOT_RADIUS,
                    ),
                    dir.normalize() * 0.006 * self.counter as f32,
                ));
                self.turn = next_side(&self.turn);
            }
            _ => (),
        };
        Ok(())
    }

    fn update(&mut self, _window: &mut Window) -> Result<()> {
        if let Some((mut circle, prev_speed)) = self.shot {
            let (pos, speed) = update_shot(circle.pos, prev_speed);
            circle.pos = pos;
            match self.collision(circle) {
                Collision::None => self.shot = Some((circle, speed)),
                Collision::Player(Side::Left) => { self.points_right += 1; self.shot = None; },
                Collision::Player(Side::Right) => { self.points_left += 1; self.shot = None; },
                _ => self.shot = None,
            }
        }
        if self.counting {
            self.counter += 3;
        } else if self.counter > 0 {
            self.counter -= 4;
            if self.counter < 0 {
                self.counter = 0;
            }
        }
        Ok(())
    }
}

fn main() {
    run::<Game>("Gorillas-rs", Vector::new(800, 600), Settings::default());
}
