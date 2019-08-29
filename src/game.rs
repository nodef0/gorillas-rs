use crate::game_constants::*;
use crate::{GameConfig, SharedAssets, SharedData};

use crate::ws::ClientImpl;

use quicksilver::{
    geom::{Circle, Line, Rectangle, Shape, Transform, Vector},
    graphics::{
        Background::{Col, Img},
        Color, Image, Surface, View,
    },
    input::{ButtonState, MouseButton},
    lifecycle::{Event, Window},
    Result,
};

use crate::ws::{WireState, WireStatePacket};
use flate2::bufread::DeflateDecoder;
use rand::prelude::*;
use std::io::prelude::*;
use std::sync::mpsc::{channel, Receiver};
use std::{cmp, iter};

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
    wind: Vector,
    rain: Vec<Vector>,
    surface: Option<Surface>,
    score: Option<Image>,
    render_score: bool,
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
    shot: Option<(Circle, Vector, f32)>,
    turn: Side,
    points_left: u32,
    points_right: u32,
    new_game: Option<Side>,
    juice: Option<f32>,
    mouse_pos: Vector,
    bot_right: Option<Bot>,
    bot_left: Option<Bot>,
    rx: Receiver<Vec<u8>>,
    client: Option<ClientImpl>,
    delta_base: Option<(u32, Vec<u8>)>,
    delta_last: Option<(u32, Option<Vec<u8>>)>,
}

struct Building {
    bound_box: Rectangle,
    tiles: Vec<usize>, // indicies into the tilemap
}

impl Bot {
    fn new(side: Side, round: &Round) -> Self {
        let (dir, target, pos) = match side {
            Side::Left => {
                let target = round.gorilla_right.center();
                let pos = round.gorilla_left.center();
                (target - pos, target, pos)
            }
            Side::Right => {
                let target = round.gorilla_left.center();
                let pos = round.gorilla_right.center();
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

impl Round {
    fn new(pools: &mut [Vec<Rectangle>]) -> Self {
        let buildings = Building::buildings(pools);
        let gorilla_left = place_gorilla(Side::Left, &buildings);
        let gorilla_right = place_gorilla(Side::Right, &buildings);
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(-1.0, 1.0);
        let y = rng.gen_range(0.0, 0.25); // do not consider upwards wind
        let strength = rng.gen_range(1.0, 2.0);
        let wind = Vector::new(x, y).normalize() * strength;
        let rain = (0..128)
            .map(|_| {
                Vector::new(
                    rng.gen_range(0.0, 1.0) * WINDOW_X,
                    rng.gen_range(0.0, 1.0) * WINDOW_Y,
                )
            })
            .collect::<Vec<_>>();
        Round {
            buildings,
            gorilla_left,
            gorilla_right,
            wind,
            rain,
            surface: None,
            score: None,
            render_score: true,
        }
    }
}

impl Building {
    fn buildings(pools: &mut [Vec<Rectangle>]) -> Vec<Building> {
        let mut b = vec![];
        let mut rng = rand::thread_rng();
        let mut last_x = 0;
        for pool in pools {
            let pos_y = TILE_SIZE.1 * rng.gen_range(16, 32);
            let height = WINDOW_Y as u32 - pos_y;
            let width = 64 + TILE_SIZE.0 * rng.gen_range(0, 3);
            let color_offset = 8 * rng.gen_range(0, 4);
            let mut tiles = vec![color_offset];
            for _ in 1..width / TILE_SIZE.0 {
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

            let bound_box = Rectangle::new((last_x, pos_y), (width, height));
            pool.push(bound_box);

            b.push(Building { bound_box, tiles });

            last_x += width + 8;
            if last_x > WINDOW_X as u32 {
                break;
            }
        }
        b
    }
}

fn from_delta(delta: &[u8], base: &[u8]) -> Vec<u8> {
    base.iter()
        .chain(iter::repeat(&0x00))
        .zip(delta.iter().chain(iter::repeat(&0x00)))
        .map(|(a, b)| a.wrapping_add(*b))
        .take(cmp::max(delta.len(), base.len()))
        .collect()
}

fn update_shot(pos: Vector, speed: Vector) -> (Vector, Vector) {
    (
        Vector::new(pos.x + DELTAT_MS * speed.x, pos.y + DELTAT_MS * speed.y),
        Vector::new(speed.x, speed.y + GRAVITY),
    )
}

fn update_shot_windy(pos: Vector, speed: Vector, wind: Vector) -> (Vector, Vector) {
    (
        Vector::new(pos.x + DELTAT_MS * speed.x, pos.y + DELTAT_MS * speed.y),
        Vector::new(wind.x + speed.x, wind.y + speed.y + GRAVITY),
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
            b.center().x - GORILLA_SIZE.0 as f32 / 2.0,
            b.pos.y - GORILLA_SIZE.1 as f32,
        ),
        GORILLA_SIZE,
    )
}

fn collide_field(pos: Vector) -> bool {
    pos.x > WINDOW_X || pos.y > WINDOW_Y || pos.x < 0.0
}

fn remove_parts(circle: &Circle, parts: &mut Vec<Rectangle>) {
    let overlap = parts
        .iter()
        .cloned()
        .filter(|x| circle.overlaps(x))
        .collect::<Vec<_>>();
    parts.retain(|x| !circle.overlaps(x));
    for i in overlap.iter() {
        parts.extend(create_parts(&circle, i));
    }
}

fn collide_buildings(circle: Circle, buildings: &[Building], parts: &[Vec<Rectangle>]) -> bool {
    for (i, building) in buildings.iter().enumerate() {
        if circle.overlaps(&building.bound_box) {
            match parts[i].len() {
                1 => return true,
                _ => {
                    for x in parts[i].iter() {
                        if circle.overlaps(x) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

// returns the index of the buildings affected by the shot
fn collide_shot(circle: Circle, buildings: &[Building], parts: &[Vec<Rectangle>]) -> Vec<usize> {
    let mut v = vec![];
    for (i, building) in buildings.iter().enumerate() {
        if circle.overlaps(&building.bound_box) {
            match parts[i].len() {
                1 => v.push(i),
                _ => {
                    for x in parts[i].iter() {
                        if circle.overlaps(x) {
                            v.push(i);
                            break;
                        }
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
    pub fn new(config: GameConfig, pools: &mut [Vec<Rectangle>]) -> Result<Self> {
        let (tx, rx) = channel();
        let round = Round::new(pools);
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
            new_game: None,
            juice: None,
            mouse_pos: Vector::ZERO,
            bot_left,
            bot_right,
            rx,
            client: ClientImpl::new("ws://localhost:2794", tx).ok(),
            delta_base: None,
            delta_last: None,
        })
    }

    fn gorilla_from_side(&self, side: Side) -> &Rectangle {
        match side {
            Side::Left => &self.round.gorilla_left,
            Side::Right => &self.round.gorilla_right,
        }
    }

    fn collision(&self, circle: Circle, parts: &[Vec<Rectangle>]) -> Collision {
        let hits = collide_buildings(circle, &self.round.buildings, parts);
        let explosion = Circle::new(circle.pos, circle.radius * 4.0);
        if hits {
            Collision::Buildings(collide_shot(explosion, &self.round.buildings, parts))
        } else if collide_field(circle.pos) {
            Collision::Sky
        } else if collide_player(circle, &self.round.gorilla_left) {
            let terrain_damage = collide_shot(explosion, &self.round.buildings, parts);
            Collision::Player(Side::Left, terrain_damage)
        } else if collide_player(circle, &self.round.gorilla_right) {
            let terrain_damage = collide_shot(explosion, &self.round.buildings, parts);
            Collision::Player(Side::Right, terrain_damage)
        } else {
            Collision::None
        }
    }

    fn on_explode(&mut self, data: &mut SharedData, pos: Vector, particle_pos: Option<Vector>) {
        if let Some(pos) = particle_pos {
            let mut rng = rand::thread_rng();
            for _ in 0..PARTICLE_COUNT {
                let color = Color::from_hex(PLAYER_PALETTE[rng.gen_range(0, PLAYER_PALETTE.len())]);
                let vel = Vector {
                    x: rng.gen_range(-1.0, 1.0),
                    y: rng.gen_range(-1.0, 1.0),
                };
                let vel = vel.normalize() * rng.gen_range(PARTICLE_MIN_VEL, PARTICLE_MAX_VEL);
                data.particle_buffer.push((pos, vel, color));
            }
        }

        self.shot = None;
        self.explosion_state = Some(Explosion {
            pos: pos - EXPLOSION_HALF_VEC,
            frame: 0,
        });
        self.juice = Some(0.0);
    }

    fn destroy_terrain(&mut self, circle: &Circle, xs: Vec<usize>, parts: &mut [Vec<Rectangle>]) {
        let explosion = Circle::new(circle.pos, circle.radius * EXPLOSION_DESTROY_SCALE);
        for i in xs {
            remove_parts(&explosion, &mut parts[i]);
        }
        self.explosion_masks.push(explosion);
    }

    pub fn draw(
        &mut self,
        shared: &SharedAssets,
        data: &SharedData,
        window: &mut Window,
    ) -> Result<()> {
        if self.round.render_score {
            shared.font.borrow_mut().execute(|f| {
                if let Ok(text) = f.render(
                    &format!("{:02}-{:02}", self.points_left, self.points_right),
                    &shared.default_style,
                ) {
                    self.round.render_score = false;
                    self.round.score = Some(text);
                }
                Ok(())
            })?;
        }

        if self.round.surface.is_none() {
            let surface = Surface::new(800, 600)?;
            surface.render_to(window, |w| {
                shared.sky.borrow_mut().execute(|sky| {
                    //draw sky
                    w.clear(Color::BLACK)?;
                    w.draw_ex(
                        &Rectangle::new_sized(w.screen_size()),
                        Img(&sky),
                        Transform::IDENTITY,
                        0.0,
                    );
                    Ok(())
                })?;
                shared.building_tiles.borrow_mut().execute(|img| {
                    for b in self.round.buildings.iter() {
                        let origin = b.bound_box.pos
                            - Vector {
                                x: TILE_SIZE.0 as f32 / 2.0,
                                y: 0.0,
                            };
                        let tiles_in_x = b.bound_box.size.x as u32 / TILE_SIZE.0 + 1;
                        for (i, tile) in b.tiles.iter().enumerate() {
                            let tile = *tile as u32;
                            let i = i as u32;
                            let tile_x = (tile % 4) * TILE_SIZE.0;
                            let tile_y = (tile / 4) * TILE_SIZE.1;

                            let pos_x = origin.x as i32 + ((i % tiles_in_x) * TILE_SIZE.0) as i32;
                            let pos_y = origin.y as i32 + ((i / tiles_in_x) * TILE_SIZE.1) as i32;
                            w.draw_ex(
                                &Rectangle::new((pos_x, pos_y), TILE_SIZE),
                                Img(&img.subimage(Rectangle::new((tile_x, tile_y), TILE_SIZE))),
                                Transform::IDENTITY,
                                1.0,
                            );
                        }
                    }
                    Ok(())
                })?;
                Ok(())
            })?;
            self.round.surface = Some(surface);
        }

        if let Some(surface) = self.round.surface.as_ref() {
            window.draw_ex(
                &Rectangle::new_sized(window.screen_size()),
                Img(surface.image()),
                Transform::IDENTITY,
                1.0,
            );
        }

        shared.sky.borrow_mut().execute(|sky| {
            //draw sky
            for mask in self.explosion_masks.iter() {
                window.draw_ex(
                    mask,
                    Img(&sky.subimage(mask.bounding_box())),
                    Transform::IDENTITY,
                    2.0,
                );
            }
            Ok(())
        })?;

        shared.building_tiles.borrow_mut().execute(|img| {
            // draw shot
            if let Some((circle, _, angle)) = self.shot {
                window.draw_ex(
                    &circle.bounding_box(),
                    Img(&img.subimage(Rectangle::new(BANANA_LOC, BANANA_SIZE))),
                    Transform::rotate(angle),
                    3.0,
                );
            } else if self.new_game.is_none()
                && self.shot.is_none()
                && self.explosion_state.is_none()
            {
                // draw aim
                let center = self.gorilla_from_side(self.turn).center();
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

            Ok(())
        })?;

        shared.player_tiles.borrow_mut().execute(|img| {
            // draw gorillas
            let (index_left, index_right) = match (self.turn, self.shot_in_progress()) {
                (_, false) => (0, 0),
                (Side::Left, true) => (1, 0),
                (Side::Right, true) => (0, 1),
            };

            let mut draw_gorilla = |side, index: u32| {
                window.draw_ex(
                    self.gorilla_from_side(side),
                    Img(&img.subimage(Rectangle::new((GORILLA_SIZE.0 * index, 0), GORILLA_SIZE))),
                    Transform::IDENTITY,
                    3.0,
                );
            };

            match self.new_game {
                None => {
                    draw_gorilla(Side::Left, index_left);
                    draw_gorilla(Side::Right, index_right);
                }
                Some(Side::Left) => {
                    draw_gorilla(Side::Left, index_left);
                }
                Some(Side::Right) => {
                    draw_gorilla(Side::Right, index_right);
                }
            }
            Ok(())
        })?;

        let power = match (self.turn, self.bot_left.as_ref(), self.bot_right.as_ref()) {
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

        shared.explosion.borrow_mut().execute(|img| {
            // draw explosion frame
            if let Some(explosion) = self.explosion_state.as_ref() {
                window.draw_ex(
                    &Rectangle::new(explosion.pos, EXPLOSION_SIZE),
                    Img(&img.subimage(Rectangle::new(
                        (EXPLOSION_SIZE.0 * (explosion.frame / 2), 0),
                        EXPLOSION_SIZE,
                    ))),
                    Transform::IDENTITY,
                    4.0,
                );
                // draw particles
                for particle in data.particle_buffer.iter() {
                    window.draw_ex(
                        &Rectangle::new(particle.0, PARTICLE_SIZE),
                        Col(particle.2),
                        Transform::IDENTITY,
                        5.0,
                    );
                }
            }
            Ok(())
        })?;

        //window.draw_ex(
        //        &Line::new(WIND_ORIGIN, WIND_ORIGIN + self.round.wind * WIND_SHOW_RATIO)
        //            .with_thickness(4.0),
        //        Col(Color::YELLOW),
        //        Transform::IDENTITY,
        //        4.0,
        //    );

        let wind_norm = self.round.wind.normalize();

        for &drop in self.round.rain.iter() {
            window.draw_ex(
                &Line::new(drop, drop + wind_norm * WIND_SHOW_RATIO).with_thickness(2.0),
                Col(Color::from_hex("dae0ea")),
                Transform::IDENTITY,
                6.0,
            );
        }
        if let Some(text) = self.round.score.as_ref() {
            window.draw_ex(
                &text.area().with_center((WINDOW_X / 2.0, 100.0)),
                Img(text),
                Transform::IDENTITY,
                4.0,
            );
        }

        Ok(())
    }

    fn event_player(&mut self, event: &Event) {
        match (event, self.counting, self.shot_in_progress()) {
            (Event::MouseMoved(pos), _, _) => self.mouse_pos = *pos,
            (Event::MouseButton(MouseButton::Left, ButtonState::Pressed), false, false) => {
                self.counting = true;
            }
            (Event::MouseButton(MouseButton::Left, ButtonState::Released), true, false) => {
                let center = self.gorilla_from_side(self.turn).center();
                let dir = (self.mouse_pos - center).normalize();
                self.counting = false;
                self.shot = Some((
                    Circle::new(center + dir * 6 * SHOT_RADIUS, SHOT_RADIUS),
                    dir * 0.006 * self.counter as f32,
                    0.0,
                ));
                if let Some(client) = self.client.as_ref() {
                    client.send_text(&format!("{} {} {}", dir.x, dir.y, self.counter));
                }
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

    fn update_bot(bot: &mut Bot, center: Vector) -> Option<(Circle, Vector, f32)> {
        bot.counter = cmp::min(bot.counter + 3, BOT_COUNTER_MAX);
        if bot.counter == BOT_COUNTER_MAX {
            let power = bot.counter;
            bot.counter = 0;
            Some((
                Circle::new(center + bot.dir * 4 * SHOT_RADIUS, SHOT_RADIUS),
                bot.dir * 0.006 * power,
                0.0,
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
        match (self.turn, self.bot_right.as_mut(), self.bot_left.as_mut()) {
            (Side::Left, _, Some(bot)) => bot.aim(pos),
            (Side::Right, Some(bot), _) => bot.aim(pos),
            _ => (),
        };
    }

    fn shot_in_progress(&self) -> bool {
        self.shot.is_some() || self.explosion_state.is_some()
    }


    fn update_network(&mut self) -> Option<WireState> {
        let mut last_state = None;
        let mut packet_data = vec![];
        let mut new_base = false;
        while let Ok(s) = self.rx.try_recv() {
            if !new_base {
                let packet: WireStatePacket = bincode::deserialize(&s).unwrap();
                if let Some(acked) = packet.seq.0 {
                    if let (None, Some((last_base, last_packet))) =
                        (self.delta_base.as_ref(), self.delta_last.as_mut())
                    {
                        // delta_base: None => Some
                        if acked == *last_base {
                            if let Some(new_base) = last_packet.take() {
                                self.delta_base = Some((acked, new_base));
                            }
                            self.delta_last = None;
                            last_state = Some(s);
                            new_base = true;
                            continue;
                        }
                    }
                    if let (Some((_, base_data)), Some((last_base, Some(last_data)))) =
                        (self.delta_base.as_ref(), self.delta_last.as_ref())
                    {
                        // delta base: Some => Some
                        if acked == *last_base {
                            self.delta_base = Some((acked, from_delta(&last_data, &base_data)));
                            self.delta_last = None;
                            last_state = Some(s);
                            new_base = true;
                            continue;
                        }
                    }
                }
            }
            last_state = Some(s);
        }
        let m = last_state?; 
        let packet: WireStatePacket = bincode::deserialize(&m).unwrap();
        DeflateDecoder::new(&packet.data[..]).read_to_end(&mut packet_data).ok()?; 

        let client_ack = packet.seq.1;

        let state = match self.delta_base.as_ref() {
            Some((_, base)) => Some(from_delta(&packet_data, &base)),
            None => Some(packet_data.clone()),
        }?;

        if self.delta_last.is_none() {
            self.delta_last = Some((client_ack, Some(packet_data)));
            self.client.as_ref()?.send_ack(client_ack);
        }

        bincode::deserialize(&state[..]).ok()
    }

    pub fn update(&mut self, data: &mut SharedData, window: &mut Window) -> Result<()> {
        self.update_network().map(|s| {
            self.client.as_ref().map(|c| c.on_console(&format!("{:?}", s)));
        });

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

        if let Some(state) = self.explosion_state.as_mut() {
            for particle in data.particle_buffer.iter_mut() {
                let (pos, speed) = update_shot(particle.0, particle.1);
                particle.0 = pos;
                particle.1 = speed;
            }

            if state.frame / 2 == EXPLOSION_FRAMES {
                data.particle_buffer.clear();
                self.explosion_state = None;
                self.turn = next_side(self.turn);
                if self.new_game.is_some() {
                    for part in data.parts.iter_mut() {
                        part.clear()
                    }
                    self.round = Round::new(&mut data.parts);
                    self.explosion_masks.clear();
                    self.reset_bots();
                    self.new_game = None;
                    return Ok(());
                }
            } else {
                state.frame += 1;
            }
        }

        if let Some((mut circle, prev_speed, mut angle)) = self.shot {
            let (pos, speed) =
                update_shot_windy(circle.pos, prev_speed, self.round.wind * WIND_PLAY_RATIO);
            circle.pos = pos;
            angle += BANANA_ANG_SPEED;
            match self.collision(circle, &data.parts) {
                Collision::None => self.shot = Some((circle, speed, angle)),
                Collision::Player(side, xs) => {
                    match side {
                        Side::Left => self.points_right += 1,
                        Side::Right => self.points_left += 1,
                    }
                    self.destroy_terrain(&circle, xs, &mut data.parts);
                    let gorilla = self.gorilla_from_side(side);
                    let gorilla = gorilla.pos + gorilla.size / 2;
                    self.on_explode(data, pos, Some(gorilla));
                    self.new_game = Some(next_side(side));
                    self.round.render_score = true;
                }
                Collision::Buildings(xs) => {
                    self.update_aim(pos);
                    self.destroy_terrain(&circle, xs, &mut data.parts);
                    self.on_explode(data, pos, None);
                }
                _ => {
                    self.update_aim(pos);
                    self.turn = next_side(self.turn);
                    self.shot = None;
                }
            }
        }

        for drop in self.round.rain.iter_mut() {
            *drop += (self.round.wind + Vector::new(0.0, GRAVITY)) * WIND_SHOW_RATIO;
            let mut rng = rand::thread_rng();
            if drop.y > WINDOW_Y {
                drop.x = rng.gen_range(0.0, 1.0) * WINDOW_X;
                drop.y = 0.0;
            } else if drop.x < 0.0 {
                drop.x = WINDOW_X;
                drop.y = rng.gen_range(0.0, 1.0) * WINDOW_Y;
            }
            if drop.x > WINDOW_X {
                drop.x = 0.0;
                drop.y = rng.gen_range(0.0, 1.0) * WINDOW_Y;
            }
        }

        if let Some(juice) = self.juice.as_mut() {
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
