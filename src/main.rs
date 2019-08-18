mod game;
mod game_constants;
mod ws;

use crate::game::*;
use crate::game_constants::*;


use quicksilver::{
    geom::{Rectangle, Shape, Transform, Vector},
    graphics::{Background::Img, Color, Font, FontStyle, Image},
    input::{ButtonState, Key, MouseButton},
    lifecycle::{run, Asset, Event, Settings, State, Window},
    Result,
};

use std::cell::RefCell;

pub struct SharedAssets {
    player_tiles: RefCell<Asset<Image>>,
    building_tiles: RefCell<Asset<Image>>,
    sky: RefCell<Asset<Image>>,
    explosion: RefCell<Asset<Image>>,
    font: RefCell<Asset<Font>>,
    default_style: FontStyle,
    hoover_style: FontStyle,
}

pub struct SharedData {
    particle_buffer: Vec<(Vector, Vector, Color)>,
    parts: Vec<Vec<Rectangle>>,
}

pub struct GameConfig {
    bot_left: bool,
    bot_right: bool,
}

struct PauseMenu;

#[derive(Copy, Clone)]
enum Hoover {
    None,
    Left,
    Right,
    Play,
}

struct MainMenu {
    dirty: bool,
    bot_right: bool,
    bot_left: bool,
    hoover: Hoover,
    areas: Vec<(Rectangle, Hoover)>,
}

enum Focus {
    Main,
    Game,
    Pause,
}

struct States {
    shared_assets: SharedAssets,
    shared_data: SharedData,
    focus: Focus,
    game: Option<Game>,
    pause_menu: PauseMenu,
    main_menu: MainMenu,
}

impl PauseMenu {
    fn draw(&mut self, shared: &SharedAssets, window: &mut Window) -> Result<()> {
        shared.font.borrow_mut().execute(|f| {
            if let Ok(ref text) = f.render("PAUSED", &shared.default_style) {
                window.draw_ex(
                    &text.area().with_center(CENTER),
                    Img(text),
                    Transform::IDENTITY,
                    4.0,
                );
            } else {
                eprintln!("Failed to render pause menu")
            }
            Ok(())
        })
    }
}

impl MainMenu {
    fn draw(&mut self, shared: &SharedAssets, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;
        shared.font.borrow_mut().execute(|f| {
            let mut draw_at_center = |s, center, style| -> Option<Rectangle> {
                if let Ok(ref text) = f.render(s, style) {
                    let rect = text.area().with_center(center);
                    window.draw_ex(&rect, Img(text), Transform::IDENTITY, 1.0);
                    Some(rect)
                } else {
                    eprintln!("Failed to render: {}", s);
                    None
                }
            };

            let select = |bot| {
                if bot {
                    "Gorilla"
                } else {
                    "Human"
                }
            };

            let (left, right, play) = match self.hoover {
                Hoover::Left => (
                    &shared.hoover_style,
                    &shared.default_style,
                    &shared.default_style,
                ),
                Hoover::Right => (
                    &shared.default_style,
                    &shared.hoover_style,
                    &shared.default_style,
                ),
                Hoover::Play => (
                    &shared.default_style,
                    &shared.default_style,
                    &shared.hoover_style,
                ),
                _ => (
                    &shared.default_style,
                    &shared.default_style,
                    &shared.default_style,
                ),
            };

            draw_at_center(
                TITLE,
                (WINDOW_X / 2.0, WINDOW_Y / 4.0),
                &shared.default_style,
            );

            let area_left = draw_at_center(
                select(self.bot_left),
                (WINDOW_X / 4.0, WINDOW_Y / 2.0),
                left,
            );
            let area_right = draw_at_center(
                select(self.bot_right),
                (WINDOW_X * 3.0 / 4.0, WINDOW_Y / 2.0),
                right,
            );
            let area_play = draw_at_center(PLAY, (WINDOW_X / 2.0, WINDOW_Y * 3.0 / 4.0), play);

            if self.dirty {
                if let (Some(area_left), Some(area_right), Some(area_play)) =
                    (area_left, area_right, area_play)
                {
                    self.areas = vec![
                        (area_left, Hoover::Left),
                        (area_right, Hoover::Right),
                        (area_play, Hoover::Play),
                    ]
                }
                self.dirty = false;
            }

            Ok(())
        })
    }

    fn event_hoover(&mut self, pos: Vector) {
        self.hoover = Hoover::None;
        for (area, id) in self.areas.iter() {
            if area.contains(pos) {
                self.hoover = *id;
                break;
            }
        }
    }

    fn event(&mut self, event: &Event, _window: &mut Window) -> Option<GameConfig> {
        match event {
            Event::Key(Key::Return, ButtonState::Pressed) => Some(GameConfig {
                bot_left: self.bot_left,
                bot_right: self.bot_right,
            }),
            Event::MouseMoved(pos) => {
                self.event_hoover(*pos);
                None
            }
            Event::MouseButton(MouseButton::Left, ButtonState::Pressed) => {
                self.dirty = true;
                match self.hoover {
                    Hoover::Left => {
                        self.bot_left = !self.bot_left;
                        None
                    }
                    Hoover::Right => {
                        self.bot_right = !self.bot_right;
                        None
                    }
                    Hoover::Play => Some(GameConfig {
                        bot_left: self.bot_left,
                        bot_right: self.bot_right,
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl State for States {
    fn new() -> Result<States> {
        let mut parts = vec![];
        for _ in 0..11 {
            parts.push(Vec::with_capacity(512));
        }

        Ok(States {
            shared_assets: SharedAssets {
                explosion: RefCell::new(Asset::new(Image::load("Explosion.png"))),
                sky: RefCell::new(Asset::new(Image::load("Sky.png"))),
                building_tiles: RefCell::new(Asset::new(Image::load("Buildings.png"))),
                player_tiles: RefCell::new(Asset::new(Image::load("Bowtie.png"))),
                font: RefCell::new(Asset::new(Font::load("UI.ttf"))),
                default_style: FontStyle::new(64.0, Color::WHITE),
                hoover_style: FontStyle::new(64.0, Color::RED),
            },
            shared_data: SharedData {
                particle_buffer: Vec::with_capacity(PARTICLE_COUNT),
                parts,
            },
            focus: Focus::Main,
            game: None,
            pause_menu: PauseMenu,
            main_menu: MainMenu {
                bot_left: false,
                bot_right: false,
                dirty: true,
                hoover: Hoover::None,
                areas: vec![],
            },
        })
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        match self.focus {
            Focus::Main => self.main_menu.draw(&self.shared_assets, window),
            Focus::Game => {
                if let Some(game) = &mut self.game {
                    game.draw(&self.shared_assets, &self.shared_data, window)?;
                }
                Ok(())
            }
            Focus::Pause => {
                if let Some(game) = &mut self.game {
                    game.draw(&self.shared_assets, &self.shared_data, window)?;
                }
                self.pause_menu.draw(&self.shared_assets, window)
            }
        }
    }

    fn event(&mut self, event: &Event, window: &mut Window) -> Result<()> {
        match (event, &self.focus) {
            // main menu
            (_, Focus::Main) => {
                if let Some(config) = self.main_menu.event(event, window) {
                    self.game = Some(Game::new(config, &mut self.shared_data.parts)?);
                    self.focus = Focus::Game;
                }
                Ok(())
            }
            // pause
            (Event::Key(Key::Space, ButtonState::Pressed), Focus::Pause) => {
                self.focus = Focus::Game;
                Ok(())
            }
            // game
            (Event::Key(Key::Escape, ButtonState::Pressed), Focus::Game) => {
                self.focus = Focus::Main;
                Ok(())
            }
            (Event::Key(Key::Space, ButtonState::Pressed), Focus::Game) => {
                self.focus = Focus::Pause;
                Ok(())
            }
            (_, Focus::Game) => {
                if let Some(game) = &mut self.game {
                    game.event(event);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        match self.focus {
            Focus::Game => {
                if let Some(game) = &mut self.game {
                    game.update(&mut self.shared_data, window)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

fn main() {
    run::<States>("Gorillas-rs", Vector::new(800, 600), Settings::default());
}
