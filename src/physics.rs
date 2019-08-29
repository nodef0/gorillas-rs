use crate::game_common::*;
use crate::game_constants::*;
use quicksilver::geom::{Circle, Rectangle, Shape, Vector};

pub enum Collision {
    None,
    Sky,
    Buildings(Vec<usize>),
    Player(Side, Vec<usize>),
}

pub fn update_shot(pos: Vector, speed: Vector) -> (Vector, Vector) {
    (
        Vector::new(pos.x + DELTAT_MS * speed.x, pos.y + DELTAT_MS * speed.y),
        Vector::new(speed.x, speed.y + GRAVITY),
    )
}

pub fn update_shot_windy(pos: Vector, speed: Vector, wind: Vector) -> (Vector, Vector) {
    (
        Vector::new(pos.x + DELTAT_MS * speed.x, pos.y + DELTAT_MS * speed.y),
        Vector::new(wind.x + speed.x, wind.y + speed.y + GRAVITY),
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

fn collide_buildings(circle: Circle, buildings: &[Rectangle], parts: &[Vec<Rectangle>]) -> bool {
    for (i, building) in buildings.iter().enumerate() {
        if circle.overlaps(building) {
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
fn collide_shot(circle: Circle, buildings: &[Rectangle], parts: &[Vec<Rectangle>]) -> Vec<usize> {
    let mut v = vec![];
    for (i, building) in buildings.iter().enumerate() {
        if circle.overlaps(building) {
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

pub fn collision(
    circle: Circle,
    left: &Rectangle,
    right: &Rectangle,
    buildings: &[Rectangle],
    parts: &[Vec<Rectangle>],
) -> Collision {
    let hits = collide_buildings(circle, buildings, parts);
    let explosion = Circle::new(circle.pos, circle.radius * 4.0);
    if hits {
        Collision::Buildings(collide_shot(explosion, buildings, parts))
    } else if collide_field(circle.pos) {
        Collision::Sky
    } else if collide_player(circle, left) {
        let terrain_damage = collide_shot(explosion, buildings, parts);
        Collision::Player(Side::Left, terrain_damage)
    } else if collide_player(circle, right) {
        let terrain_damage = collide_shot(explosion, buildings, parts);
        Collision::Player(Side::Right, terrain_damage)
    } else {
        Collision::None
    }
}

pub fn destroy_terrain(circle: &Circle, xs: Vec<usize>, parts: &mut [Vec<Rectangle>]) -> Circle {
    let explosion = Circle::new(circle.pos, circle.radius * EXPLOSION_DESTROY_SCALE);
    for i in xs {
        remove_parts(&explosion, &mut parts[i]);
    }
    explosion
}
