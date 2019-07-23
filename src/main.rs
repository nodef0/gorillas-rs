mod game_constants;
use game_constants::*;

use quicksilver::{
    geom::{Circle, Line, Rectangle, Shape, Transform, Vector},
    graphics::{
        Background::{Col, Img},
        Color, Font, FontStyle, Image, PixelFormat, View,
    },
    input::{ButtonState, MouseButton},
    lifecycle::{run, Asset, Event, Settings, State, Window},
    Result,
};

use rand::prelude::*;
use std::cell::RefCell;
use std::cmp;

struct Round {
    buildings: Vec<(Rectangle, Color, Option<Vec<Rectangle>>)>,
    gorilla_left: Rectangle,
    gorilla_right: Rectangle,
}

struct Explosion {
    pos: Vector,
    frame: u32,
}

struct Game {
    round: Round,
    counting: bool,
    counter: i32,
    sky: Image,
    explosion: RefCell<Asset<Image>>,
    explosion_state: Option<Explosion>,
    font: RefCell<Asset<Font>>,
    font_style: FontStyle,
    shot: Option<(Circle, Vector)>,
    turn: Side,
    points_left: u32,
    points_right: u32,
    new_game: bool,
    juice: Option<f32>,
}

enum Side {
    Left,
    Right,
}

enum Collision {
    None,
    Sky,
    Buildings(Vec<usize>),
    Player(Side),
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

fn buildings() -> Vec<(Rectangle, Color, Option<Vec<Rectangle>>)> {
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
            None,
        ));
        last_x += width;
        if last_x > WINDOW_X {
            break;
        }
    }
    b
}

fn update_shot(pos: Vector, speed: Vector) -> (Vector, Vector) {
    (
        Vector::new(pos.x + DELTAT_MS * speed.x, pos.y + DELTAT_MS * speed.y),
        Vector::new(speed.x, speed.y + GRAVITY),
    )
}

fn next_side(side: &Side) -> Side {
    match side {
        Side::Left => Side::Right,
        Side::Right => Side::Left,
    }
}

fn place_gorilla(
    side: Side,
    buildings: &[(Rectangle, Color, Option<Vec<Rectangle>>)],
) -> Rectangle {
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

fn collide_field(pos: Vector) -> bool {
    pos.x > WINDOW_X as f32 || pos.y > WINDOW_Y as f32 || pos.x < 0.0
}

fn remove_parts(circle: Circle, parts: &mut Vec<Rectangle>) {
    let overlap = parts
        .iter()
        .cloned()
        .filter(|x| circle.overlaps(x))
        .collect::<Vec<_>>();
    parts.retain(|x| !circle.overlaps(x));
    for i in overlap {
        parts.extend(create_parts(&circle, &i));
    }
}

fn collide_shot(
    circle: Circle,
    buildings: &[(Rectangle, Color, Option<Vec<Rectangle>>)],
) -> Vec<usize> {
    let mut v = vec![];
    for (i, (rect, _, parts)) in buildings.iter().enumerate() {
        match parts {
            None => {
                if circle.overlaps(rect) {
                    v.push(i);
                }
            }
            Some(xs) => {
                for x in xs {
                    if circle.overlaps(x) {
                        v.push(i);
                        break;
                    }
                }
            }
        }
    }
    v
}

fn collide_player(circle: Circle, player: &Rectangle) -> bool {
    circle.overlaps(player)
}

fn create_parts(circle: &Circle, source: &Rectangle) -> Vec<Rectangle> {
    let size = source.size / 2.0;
    let rects = [
        Rectangle::new((source.pos.x, source.pos.y), size),
        Rectangle::new((source.pos.x + size.x, source.pos.y), size),
        Rectangle::new((source.pos.x, source.pos.y + size.y), size),
        Rectangle::new((source.pos.x + size.x, source.pos.y + size.y), size),
    ];
    if source.size.x < PARTS_GRID_MIN && source.size.y < PARTS_GRID_MIN {
        rects
            .iter()
            .cloned()
            .filter(|x| !circle.overlaps(x))
            .collect::<Vec<_>>()
    } else {
        let mut v = vec![];
        for r in rects.iter() {
            if circle.overlaps(r) {
                v.extend(create_parts(circle, r));
            } else {
                v.push(*r);
            }
        }
        v
    }
}

impl Round {
    fn new() -> Self {
        let buildings = buildings();
        let gorilla_left = place_gorilla(Side::Left, &buildings);
        let gorilla_right = place_gorilla(Side::Right, &buildings);
        Round {
            buildings,
            gorilla_left,
            gorilla_right,
        }
    }
}

impl Game {
    fn gorilla_from_side(&self, side: &Side) -> &Rectangle {
        match side {
            Side::Left => &self.round.gorilla_left,
            Side::Right => &self.round.gorilla_right,
        }
    }

    fn collision(&self, circle: Circle) -> Collision {
        let hits = collide_shot(circle, &self.round.buildings);
        if !hits.is_empty() {
            let explosion = Circle::new(circle.pos, circle.radius * 4.0);
            Collision::Buildings(collide_shot(explosion, &self.round.buildings))
        } else if collide_field(circle.pos) {
            Collision::Sky
        } else if collide_player(circle, &self.round.gorilla_left) {
            Collision::Player(Side::Left)
        } else if collide_player(circle, &self.round.gorilla_right) {
            Collision::Player(Side::Right)
        } else {
            Collision::None
        }
    }

    fn on_explode(&mut self, pos: Vector) {
        self.shot = None;
        self.explosion_state = Some(Explosion {
            pos: pos - EXPLOSION_HALF_VEC,
            frame: 0,
        });
        self.juice = Some(0.0);
    }
}

impl State for Game {
    fn new() -> Result<Game> {
        let sky_vec = gen_cuts(SKY_CUTS, &SKY_BLUE_DARK, &SKY_BLUE_LIGHT);
        Ok(Game {
            round: Round::new(),
            counting: false,
            counter: 0i32,
            sky: Image::from_raw(&sky_vec, 1, (2 << (SKY_CUTS - 1)) - 1, PixelFormat::RGBA)?,
            explosion: RefCell::new(Asset::new(Image::load("Explosion.png"))),
            explosion_state: None,
            font: RefCell::new(Asset::new(Font::load("UI.ttf"))),
            font_style: FontStyle::new(64.0, Color::WHITE),
            shot: None,
            turn: Side::Left,
            points_left: 0,
            points_right: 0,
            new_game: false,
            juice: None,
        })
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;

        // draw sky
        window.draw_ex(
            &Rectangle::new((0, 0), window.screen_size()),
            Img(&self.sky),
            Transform::IDENTITY,
            0.0,
        );

        //draw buildings
        for b in self.round.buildings.iter() {
            if let Some(ref parts) = b.2 {
                for p in parts {
                    window.draw_ex(p, b.1, Transform::IDENTITY, 1.0);
                }
            } else {
                window.draw_ex(&b.0, b.1, Transform::IDENTITY, 1.0);
            }
        }

        // draw gorillas
        window.draw_ex(
            &self.round.gorilla_left,
            Col(Color::RED),
            Transform::IDENTITY,
            2.0,
        );
        window.draw_ex(
            &self.round.gorilla_right,
            Col(Color::RED),
            Transform::IDENTITY,
            2.0,
        );

        // draw shot
        if let Some((circle, _)) = self.shot {
            window.draw_ex(&circle, Col(Color::YELLOW), Transform::IDENTITY, 3.0);
        } else if !self.new_game {
            // draw aim
            let gorilla = self.gorilla_from_side(&self.turn);
            let center = gorilla.pos + (gorilla.size / 2);
            let dir = (window.mouse().pos() - center).normalize();
            window.draw_ex(
                &Line::new(center + (dir * START_OFFSET), center + (dir * END_OFFSET))
                    .with_thickness(4.0),
                Col(Color::YELLOW),
                Transform::IDENTITY,
                4.0,
            );
        }

        // draw power bar
        window.draw_ex(
            &Rectangle::new((100, 500), (self.counter, 50)),
            Col(Color::RED),
            Transform::IDENTITY,
            3.0,
        );

        // draw explosion frame
        self.explosion.borrow_mut().execute(|img| {
            if let Some(ref explosion) = self.explosion_state {
                window.draw_ex(
                    &Rectangle::new(explosion.pos, EXPLOSION_SIZE),
                    Img(&img.subimage(Rectangle::new(
                        (96 * (explosion.frame / 2), 0),
                        EXPLOSION_SIZE,
                    ))),
                    Transform::IDENTITY,
                    4.0,
                );
            }
            Ok(())
        })?;

        //draw score
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
                let center = gorilla.pos + (gorilla.size / 2);
                let dir = (window.mouse().pos() - center).normalize();
                self.shot = Some((
                    Circle::new(center + dir * 4 * SHOT_RADIUS, SHOT_RADIUS),
                    dir * 0.006 * self.counter as f32,
                ));
                self.turn = next_side(&self.turn);
            }
            _ => (),
        };
        Ok(())
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        if let Some(ref mut state) = self.explosion_state {
            if state.frame / 2 == EXPLOSION_FRAMES {
                self.explosion_state = None;
                if self.new_game {
                    self.round = Round::new();
                    self.new_game = false;
                    return Ok(());
                }
            } else {
                state.frame += 1;
            }
        }

        if let Some((mut circle, prev_speed)) = self.shot {
            let (pos, speed) = update_shot(circle.pos, prev_speed);
            circle.pos = pos;
            match self.collision(circle) {
                Collision::None => self.shot = Some((circle, speed)),
                Collision::Player(side) => {
                    match side {
                        Side::Left => self.points_right += 1,
                        Side::Right => self.points_left += 1,
                    }
                    self.on_explode(pos);
                    self.new_game = true;
                }
                Collision::Buildings(xs) => {
                    let explosion = Circle::new(circle.pos, circle.radius * 4.0);
                    for i in xs {
                        let building = &mut self.round.buildings[i];
                        match building.2 {
                            None => building.2 = Some(create_parts(&explosion, &building.0)),
                            Some(ref mut parts) => remove_parts(explosion, parts),
                        }
                    }
                    self.on_explode(pos);
                }
                _ => self.shot = None,
            }
        }

        if let Some(ref mut juice) = &mut self.juice {
            *juice += JUICE_COUNTER;
            let y = 10.0 * (-2.0 * *juice).exp2() * juice.sin();
            let x = 0.2 * (-2.0 * *juice).exp2() * juice.sin();
            let view = if y.abs() < 0.01 {
                self.juice = None;
                Rectangle::new_sized((800, 600))
            } else {
                Rectangle::new((x, y), (800, 600))
            };
            window.set_view(View::new(view));
        }

        if self.counting {
            self.counter = cmp::min(self.counter + 3, POWER_MAX);
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
