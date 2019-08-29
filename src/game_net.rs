use crate::game_common::*;
use crate::game_constants::*;
use crate::ws::ClientImpl;
use crate::ws::{GameEvents, WireStatePacket};
use crate::{GameConfig, Player, SharedAssets, SharedData};

use flate2::bufread::DeflateDecoder;
use rand::prelude::*;
use serde::Deserialize;
use std::io::prelude::*;
use std::sync::mpsc::Receiver;
use std::{cmp, iter};

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

struct NetState {
    rx: Receiver<Vec<u8>>, // TODO: change type to enum to indicate disconnect
    delta_base: Option<(u32, Vec<u8>)>,
    delta_last: Option<(u32, Option<Vec<u8>>)>,
}

#[derive(Deserialize, Debug)]
struct Round {
    buildings: Vec<Rectangle>,
    gorilla_left: Rectangle,
    gorilla_right: Rectangle,
    wind: Vector,
}

#[derive(Deserialize, Debug)]
struct WireState {
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
    mouse_pos: Vector,
}

// rule: if we get a net update, use it, otherwise use the local state for
// explosion frames
struct LocalState {
    juice: Option<f32>,
    rain: Vec<Vector>,
    surface: Option<Surface>,
    score: Option<Image>,
    render_score: bool,
    pending: bool,
}

pub struct GameNet {
    wire: Option<WireState>,
    net: NetState,
    local: LocalState,
    config: GameConfig,
    // TODO: add bot support for net games
}

fn from_delta(delta: &[u8], base: &[u8]) -> Vec<u8> {
    base.iter()
        .chain(iter::repeat(&0x00))
        .zip(delta.iter().chain(iter::repeat(&0x00)))
        .map(|(a, b)| a.wrapping_add(*b))
        .take(cmp::max(delta.len(), base.len()))
        .collect()
}

impl NetState {
    fn update_network(&mut self, client: &ClientImpl) -> Option<WireState> {
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
        DeflateDecoder::new(&packet.data[..])
            .read_to_end(&mut packet_data)
            .ok()?;

        let client_ack = packet.seq.1;

        let state = match self.delta_base.as_ref() {
            Some((_, base)) => Some(from_delta(&packet_data, &base)),
            None => Some(packet_data.clone()),
        }?;

        if self.delta_last.is_none() {
            self.delta_last = Some((client_ack, Some(packet_data)));
            client.send_ack(client_ack);
        }

        bincode::deserialize(&state[..]).ok()
    }
}

impl WireState {
    fn shot_in_progress(&self) -> bool {
        self.shot.is_some() || self.explosion_state.is_some()
    }

    fn gorilla_from_side(&self, side: Side) -> &Rectangle {
        match side {
            Side::Left => &self.round.gorilla_left,
            Side::Right => &self.round.gorilla_right,
        }
    }

    pub fn draw(
        &self,
        local: &mut LocalState, // required to update surfaces
        shared: &SharedAssets,
        data: &SharedData,
        window: &mut Window,
    ) -> Result<()> {
        if local.render_score {
            shared.font.borrow_mut().execute(|f| {
                if let Ok(text) = f.render(
                    &format!("{:02}-{:02}", self.points_left, self.points_right),
                    &shared.default_style,
                ) {
                    local.render_score = false;
                    local.score = Some(text);
                }
                Ok(())
            })?;
        }

        if local.surface.is_none() {
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
                    let mut rng = rand::thread_rng();
                    for b in self.round.buildings.iter() {
                        let origin = b.pos
                            - Vector {
                                x: TILE_SIZE.0 as f32 / 2.0,
                                y: 0.0,
                            };
                        let tiles_in_x = b.size.x as u32 / TILE_SIZE.0 + 1;

                        let mut i: u32 = 0;
                        let mut draw_tile = |tile: u32| {
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
                            i += 1;
                        };

                        let color_offset = 8 * rng.gen_range(0, 4);
                        let width = b.size.x as u32;
                        let height = b.size.y as u32;

                        draw_tile(color_offset);
                        for _ in 1..width / TILE_SIZE.0 {
                            draw_tile(1 + color_offset);
                        }
                        draw_tile(2 + color_offset);
                        for _ in 1..=height / 16 {
                            draw_tile(3 + color_offset);
                            for _ in 1..width / 16 {
                                let mid_tile = rng.gen_range(5, 8);
                                draw_tile(mid_tile + color_offset);
                            }
                            draw_tile(4 + color_offset);
                        }
                    }
                    Ok(())
                })?;
                Ok(())
            })?;
            local.surface = Some(surface);
        }

        if let Some(surface) = local.surface.as_ref() {
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
                // TODO: use config + turn to select either wire or local mouse_pos
                // draw aim
                let center = self.gorilla_from_side(self.turn).center();
                let dir = (self.mouse_pos - center).normalize();
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

        let power = self.counter;

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

        let wind_norm = self.round.wind.normalize();

        for &drop in local.rain.iter() {
            window.draw_ex(
                &Line::new(drop, drop + wind_norm * WIND_SHOW_RATIO).with_thickness(2.0),
                Col(Color::from_hex("dae0ea")),
                Transform::IDENTITY,
                6.0,
            );
        }
        if let Some(text) = local.score.as_ref() {
            window.draw_ex(
                &text.area().with_center((WINDOW_X / 2.0, 100.0)),
                Img(text),
                Transform::IDENTITY,
                4.0,
            );
        }

        Ok(())
    }
}

impl GameNet {
    pub fn new(config: GameConfig, rx: Receiver<Vec<u8>>) -> Self {
        GameNet {
            wire: None,
            net: NetState {
                rx,
                delta_base: None,
                delta_last: None,
            },
            local: LocalState {
                juice: None,
                rain: vec![],
                surface: None,
                score: None,
                render_score: true,
                pending: false,
            },
            config,
        }
    }

    fn update_locals(&mut self, window: &mut Window) {
        if let Some(juice) = self.local.juice.as_mut() {
            *juice += JUICE_COUNTER;
            let y = 10.0 * (-2.0 * *juice).exp2() * juice.sin();
            let x = 0.2 * (-2.0 * *juice).exp2() * juice.sin();
            let view = if y.abs() < 0.01 {
                self.local.juice = None;
                Rectangle::new_sized((800, 600))
            } else {
                Rectangle::new((x, y), (800, 600))
            };
            window.set_view(View::new(view));
        }
    }

    pub fn update(&mut self, client: &ClientImpl, data: &mut SharedData, window: &mut Window) {
        if let wire @ Some(_) = self.net.update_network(client) {
            self.wire = wire;
            // update render_score
            if let Some(wire) = &self.wire {
                if wire.new_game.is_some() {
                    self.local.pending = true;
                } else if self.local.pending {
                    self.local.surface = None;
                    self.local.score = None;
                    self.local.render_score = true;
                    self.local.pending = false;
                }
            }
        } else {
        }

        if self.wire.is_some() {
            self.update_locals(window);
        }
        client.on_console(&format!("{:?}", self.wire));
    }

    fn event_player(&mut self, event: &Event, client: &ClientImpl) {
        if let Some(wire) = &self.wire {
            match (event, wire.shot_in_progress()) {
                (Event::MouseMoved(pos), _) => client.send_game_event(GameEvents::MousePos(*pos)),
                (Event::MouseButton(MouseButton::Left, ButtonState::Pressed), false) => {
                    client.send_game_event(GameEvents::MousePressed)
                }
                (Event::MouseButton(MouseButton::Left, ButtonState::Released), false) => {
                    client.send_game_event(GameEvents::MouseReleased)
                }
                _ => (),
            }
        }
    }

    pub fn event(&mut self, event: &Event, client: &ClientImpl) {
        self.event_player(event, client);
        //match (wire.turn, self.config.left, self.config.right) {
        //    (Side::Left, Player::Human, _) => self.event_player(event, client),
        //    (Side::Right, _, Player::Human) => self.event_player(event, client),
        //    _ => (),
        //}
    }

    pub fn draw(
        &mut self,
        shared: &SharedAssets,
        data: &SharedData,
        window: &mut Window,
    ) -> Result<()> {
        if let Some(state) = &self.wire {
            state.draw(&mut self.local, shared, data, window)?;
        }
        Ok(())
    }
}
