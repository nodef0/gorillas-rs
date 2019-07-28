use crate::game_constants::*;
use crate::{GameConfig, SharedAssets};
use crate::circle::CircleF;

use quicksilver::{
    geom::{Circle, Line, Rectangle, Shape, Transform, Vector},
    graphics::{
        Background::{Col, Img},
        Color, View,
    },
    input::{ButtonState, MouseButton},
    lifecycle::{Event, Window},
    Result,
};

use rand::prelude::*;
use std::cmp;

#[derive(Copy, Clone)]
enum Side {
    Left,
    Right,
}

struct Bot {
    counter: i32,
    dir: Vector,
    pos: Vector,
    target: Vector,
}

impl Bot {
    fn new(side: Side, round: &Round) -> Self {
        let (dir, target, pos) = match side {
            Side::Left => {
                let target = round.gorilla_right.pos + round.gorilla_right.size / 2.0;
                let pos = round.gorilla_left.pos + round.gorilla_left.size / 2.0;
                (target - pos, target, pos)
            }
            Side::Right => {
                let target = round.gorilla_left.pos + round.gorilla_left.size / 2.0;
                let pos = round.gorilla_right.pos + round.gorilla_right.size / 2.0;
                (target - pos, target, pos)
            }
        };
        let dir = dir.normalize();
        Bot {
            counter: 0,
            dir,
            target,
            pos,
        }
    }

    fn aim(&mut self, collision: Vector) {
        let mut inc = if collision.x < self.target.x {
            BOT_AIM_INC_Y
        } else {
            -BOT_AIM_INC_Y
        };
        if self.pos.x < self.target.x {
            inc = -inc;
        }
        self.dir.y += inc;
        self.dir = self.dir.normalize();
    }
}

enum Collision {
    None,
    Sky,
    Buildings(Vec<usize>),
    Player(Side, Vec<usize>),
}

struct Round {
    buildings: Vec<Building>,
    gorilla_left: Rectangle,
    gorilla_right: Rectangle,
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

struct Explosion {
    pos: Vector,
    frame: u32,
}

pub struct Game {
    round: Round,
    counting: bool,
    counter: i32,
    explosion_state: Option<Explosion>,
    explosion_masks: Vec<Circle>,
    shot: Option<(Circle, Vector)>,
    turn: Side,
    points_left: u32,
    points_right: u32,
    new_game: bool,
    juice: Option<f32>,
    mouse_pos: Vector,
    bot_right: Option<Bot>,
    bot_left: Option<Bot>,
}

struct Building {
    bound_box: Rectangle,
    tiles: Vec<usize>, // indicies into the tilemap
    parts: Option<Vec<Rectangle>>,
}

fn buildings() -> Vec<Building> {
    let mut b = vec![];
    let mut rng = rand::thread_rng();
    let mut last_x = 0;
    loop {
        let pos_y = 16 * rng.gen_range(16, 32);
        let height = WINDOW_Y as u32 - pos_y;
        let width = 64 + 16 * rng.gen_range(0, 3);
        let color_offset = 8 * rng.gen_range(0, 4); 
        let mut tiles = vec![color_offset];
        for _ in 1..width / 16 {
            tiles.push(1 + color_offset);
        }
        tiles.push(2 + color_offset);
        for _ in 1..=height / 16 {
            tiles.push(3 + color_offset);
            for _ in 1..width / 16 {
                let mid_tile = rng.gen_range(5, 8);
                tiles.push(mid_tile + color_offset);
            }
            tiles.push(4 + color_offset);
        }

        b.push(Building {
            bound_box: Rectangle::new((last_x, pos_y), (width, height)),
            parts: None,
            tiles,
        });
        last_x += width + 8;
        if last_x > WINDOW_X as u32 {
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

fn next_side(side: Side) -> Side {
    match side {
        Side::Left => Side::Right,
        Side::Right => Side::Left,
    }
}

fn place_gorilla(side: Side, buildings: &[Building]) -> Rectangle {
    let field_length = buildings.len() - 1;
    let mut rng = rand::thread_rng();
    let i = match side {
        Side::Left => rng.gen_range(1, (field_length / 2) - DISTANCE_MIN),
        Side::Right => rng.gen_range((field_length / 2) + DISTANCE_MIN, field_length),
    };
    let b = buildings[i].bound_box;
    Rectangle::new(
        (
            b.pos.x + b.size.x / 2.0 - GORILLA_SIZE.0 / 2.0,
            b.pos.y - GORILLA_SIZE.1,
        ),
        GORILLA_SIZE,
    )
}

fn collide_field(pos: Vector) -> bool {
    pos.x > WINDOW_X || pos.y > WINDOW_Y || pos.x < 0.0
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

fn collide_shot(circle: Circle, buildings: &[Building]) -> Vec<usize> {
    let mut v = vec![];
    for (i, building) in buildings.iter().enumerate() {
        // (i, (rect, _, parts))
        match &building.parts {
            None => {
                if circle.overlaps(&building.bound_box) {
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


impl Game {
    pub fn new(config: GameConfig) -> Result<Self> {
        let round = Round::new();
        let bot_left = if config.bot_left {
            Some(Bot::new(Side::Left, &round))
        } else {
            None
        };
        let bot_right = if config.bot_right {
            Some(Bot::new(Side::Right, &round))
        } else {
            None
        };
        Ok(Game {
            round,
            counting: false,
            counter: 0i32,
            explosion_state: None,
            explosion_masks: vec![],
            shot: None,
            turn: Side::Left,
            points_left: 0,
            points_right: 0,
            new_game: false,
            juice: None,
            mouse_pos: Vector::ZERO,
            bot_left,
            bot_right,
        })
    }
    fn gorilla_from_side(&self, side: Side) -> &Rectangle {
        match side {
            Side::Left => &self.round.gorilla_left,
            Side::Right => &self.round.gorilla_right,
        }
    }

    fn collision(&self, circle: Circle) -> Collision {
        let hits = collide_shot(circle, &self.round.buildings);
        let explosion = Circle::new(circle.pos, circle.radius * 4.0);
        if !hits.is_empty() {
            Collision::Buildings(collide_shot(explosion, &self.round.buildings))
        } else if collide_field(circle.pos) {
            Collision::Sky
        } else if collide_player(circle, &self.round.gorilla_left) {
            let terrain_damage = collide_shot(explosion, &self.round.buildings);
            Collision::Player(Side::Left, terrain_damage)
        } else if collide_player(circle, &self.round.gorilla_right) {
            let terrain_damage = collide_shot(explosion, &self.round.buildings);
            Collision::Player(Side::Right, terrain_damage)
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

    fn destroy_terrain(&mut self, circle: &Circle, xs: Vec<usize>) {
        let explosion = Circle::new(circle.pos, circle.radius * 4.0);
        for i in xs {
            let building = &mut self.round.buildings[i];
            match building.parts {
                None => building.parts = Some(create_parts(&explosion, &building.bound_box)),
                Some(ref mut parts) => remove_parts(explosion, parts),
            }
        }
        self.explosion_masks.push(explosion);
    }

    pub fn draw(&mut self, shared: &SharedAssets, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;

        // draw sky
        shared.sky.borrow_mut().execute(|img| {

            for mask in &self.explosion_masks {
                window.draw_ex(
                    &CircleF {
                        pos: mask.pos,
                        radius: mask.radius,
                    },
                    Img(&img.subimage(mask.bounding_box())),
                    Transform::IDENTITY,
                    2.0,
                );
            }

            window.draw_ex(
                &Rectangle::new((0, 0), window.screen_size()),
                Img(&img),
                Transform::IDENTITY,
                0.0,
            );
            
            Ok(())
        })?;

        //draw buildings
        for b in self.round.buildings.iter() {
            //if let Some(ref parts) = b.parts {
            //    for p in parts {
            //        window.draw_ex(p, b.color, Transform::IDENTITY, 1.0);
            //    }
            //} else {
                // window.draw_ex(&b.bound_box, b.color, Transform::IDENTITY, 1.0);
                shared.building_tiles.borrow_mut().execute(|img| {
                    let origin = b.bound_box.pos - Vector { x: 8.0, y: 0.0 };
                    let tiles_in_x = b.bound_box.size.x as i32 / 16 + 1;
                    for (i, tile) in b.tiles.iter().enumerate() {
                        let tile_x = (tile % 4) * 16;
                        let tile_y = (tile / 4) * 16;

                        let pos_x = origin.x as i32 + (i as i32 % tiles_in_x) * 16;
                        let pos_y = origin.y as i32 + (i as i32 / tiles_in_x) * 16;
                        window.draw_ex(
                            &Rectangle::new((pos_x as i32, pos_y as i32), (16, 16)),
                            Img(&img.subimage(Rectangle::new(
                                (tile_x as u32, tile_y as u32),
                                (16, 16),
                            ))),
                            Transform::IDENTITY,
                            1.0,
                        );
                    }
                    Ok(())
                })?;
            //}
        }

        // draw gorillas
        window.draw_ex(
            &self.round.gorilla_left,
            Col(Color::RED),
            Transform::IDENTITY,
            3.0,
        );
        window.draw_ex(
            &self.round.gorilla_right,
            Col(Color::RED),
            Transform::IDENTITY,
            3.0,
        );

        // draw shot
        if let Some((circle, _)) = self.shot {
            window.draw_ex(&circle, Col(Color::YELLOW), Transform::IDENTITY, 3.0);
        } else if !self.new_game && self.shot.is_none() && self.explosion_state.is_none() {
            // draw aim
            let gorilla = self.gorilla_from_side(self.turn);
            let center = gorilla.pos + (gorilla.size / 2);
            let dir = match (self.turn, &self.bot_left, &self.bot_right) {
                (Side::Left, Some(bot), _) => bot.dir,
                (Side::Right, _, Some(bot)) => bot.dir,
                _ => (self.mouse_pos - center).normalize(),
            };
            window.draw_ex(
                &Line::new(center + (dir * START_OFFSET), center + (dir * END_OFFSET))
                    .with_thickness(4.0),
                Col(Color::YELLOW),
                Transform::IDENTITY,
                4.0,
            );
        }

        let power = match (self.turn, &self.bot_left, &self.bot_right) {
            (Side::Left, Some(bot), _) => bot.counter,
            (Side::Right, _, Some(bot)) => bot.counter,
            _ => self.counter,
        };

        // draw power bar
        window.draw_ex(
            &Rectangle::new((100, 500), (power, 50)),
            Col(Color::RED),
            Transform::IDENTITY,
            3.0,
        );

        // draw explosion frame
        shared.explosion.borrow_mut().execute(|img| {
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
        shared.font.borrow_mut().execute(|f| {
            if let Ok(ref text) = f.render(
                &format!("{:02}-{:02}", self.points_left, self.points_right),
                &shared.default_style,
            ) {
                window.draw_ex(
                    &text.area().with_center((WINDOW_X / 2.0, 100.0)),
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

    fn event_player(&mut self, event: &Event) {
        match (event, self.counting, self.shot) {
            (Event::MouseMoved(pos), _, _) => self.mouse_pos = *pos,
            (Event::MouseButton(MouseButton::Left, ButtonState::Pressed), false, None) => {
                self.counting = true;
            }
            (Event::MouseButton(MouseButton::Left, ButtonState::Released), true, None) => {
                self.counting = false;
                let gorilla = self.gorilla_from_side(self.turn);
                let center = gorilla.pos + (gorilla.size / 2);
                let dir = (self.mouse_pos - center).normalize();
                self.shot = Some((
                    Circle::new(center + dir * 4 * SHOT_RADIUS, SHOT_RADIUS),
                    dir * 0.006 * self.counter as f32,
                ));
            }
            _ => (),
        }
    }

    pub fn event(&mut self, event: &Event) {
        match (self.turn, &mut self.bot_right, &mut self.bot_left) {
            (Side::Left, _, None) => self.event_player(event),
            (Side::Right, None, _) => self.event_player(event),
            _ => (),
        }
    }

    fn update_bot(bot: &mut Bot, center: Vector) -> Option<(Circle, Vector)> {
        bot.counter = cmp::min(bot.counter + 3, BOT_COUNTER_MAX);
        if bot.counter == BOT_COUNTER_MAX {
            let power = bot.counter;
            bot.counter = 0;
            Some((
                Circle::new(center + bot.dir * 4 * SHOT_RADIUS, SHOT_RADIUS),
                bot.dir * 0.006 * power,
            ))
        } else {
            None
        }
    }

    fn reset_bots(&mut self) {
        if self.bot_right.is_some() {
            self.bot_right = Some(Bot::new(Side::Right, &self.round));
        }
        if self.bot_left.is_some() {
            self.bot_left = Some(Bot::new(Side::Left, &self.round));
        }
    }

    fn update_aim(&mut self, pos: Vector) {
        match (self.turn, &mut self.bot_right, &mut self.bot_left) {
            (Side::Left, _, Some(bot)) => bot.aim(pos),
            (Side::Right, Some(bot), _) => bot.aim(pos),
            _ => (),
        };
    }

    pub fn update(&mut self, window: &mut Window) -> Result<()> {
        let gorilla = self.gorilla_from_side(self.turn);
        let center = gorilla.pos + (gorilla.size / 2);

        if self.shot.is_none() && self.explosion_state.is_none() {
            let bot_shot = match (self.turn, &mut self.bot_right, &mut self.bot_left) {
                (Side::Left, _, Some(bot)) => Game::update_bot(bot, center),
                (Side::Right, Some(bot), _) => Game::update_bot(bot, center),
                _ => None,
            };
            if let Some(shot) = bot_shot {
                self.shot = Some(shot);
            }
        }

        if let Some(ref mut state) = self.explosion_state {
            if state.frame / 2 == EXPLOSION_FRAMES {
                self.explosion_state = None;
                self.turn = next_side(self.turn);
                if self.new_game {
                    self.round = Round::new();
                    self.explosion_masks.clear();
                    self.reset_bots();
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
                Collision::Player(side, xs) => {
                    match side {
                        Side::Left => self.points_right += 1,
                        Side::Right => self.points_left += 1,
                    }
                    self.destroy_terrain(&circle, xs);
                    self.on_explode(pos);
                    self.new_game = true;
                }
                Collision::Buildings(xs) => {
                    self.update_aim(pos);
                    self.destroy_terrain(&circle, xs);
                    self.on_explode(pos);
                }
                _ => {
                    self.update_aim(pos);
                    self.turn = next_side(self.turn);
                    self.shot = None;
                }
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
