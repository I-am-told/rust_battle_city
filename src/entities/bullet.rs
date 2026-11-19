use macroquad::prelude::*;
use crate::map::GameMap;
use crate::entities::tank::Direction;

pub struct Bullet {
    pub pos: Vec2,
    pub direction: Direction,
    pub speed: f32,
    pub size: f32,
    pub owner_id: u32,
    pub alive: bool,
    pub pierces_steel: bool,
}

impl Bullet {
    pub fn new(pos: Vec2, direction: Direction, speed: f32, size: f32, owner_id: u32) -> Self {
        Self {
            pos,
            direction,
            speed,
            size,
            owner_id,
            alive: true,
            pierces_steel: false,
        }
    }

    pub fn update(&mut self, dt: f32, map: &mut GameMap, tile_size: f32) {
        if !self.alive { return; }

        let v = self.direction.vector();
        let step = vec2(v.x as f32, v.y as f32) * self.speed * dt;

        let max_step = tile_size * 0.5;
        let total = step.length();
        let steps = (total / max_step).ceil().max(1.0) as i32;
        let sub_step = step / steps as f32;

        for _ in 0..steps {
            let next = self.pos + sub_step;
            if self.out_of_bounds(next, map, tile_size) {
                self.alive = false;
                return;
            }
            if self.collides_tile(next, map, tile_size) {
                self.damage_hit_tile(next, map, tile_size);
                self.alive = false;
                return;
            }
            self.pos = next;
        }
    }

    fn out_of_bounds(&self, pos: Vec2, map: &GameMap, tile_size: f32) -> bool {
        let playfield_w = map.width as f32 * tile_size;
        let playfield_h = map.height as f32 * tile_size;
        pos.x < 0.0
            || pos.y < 0.0
            || pos.x + self.size > playfield_w
            || pos.y + self.size > playfield_h
    }

    fn collides_tile(&self, pos: Vec2, map: &GameMap, tile_size: f32) -> bool {
        let epsilon = 0.001;
        let left   = (pos.x / tile_size).floor() as i32;
        let right  = ((pos.x + self.size - epsilon) / tile_size).floor() as i32;
        let top    = (pos.y / tile_size).floor() as i32;
        let bottom = ((pos.y + self.size - epsilon) / tile_size).floor() as i32;

        for ty in top..=bottom {
            for tx in left..=right {
                if map.is_blocked_for_bullet(tx, ty) {
                    return true;
                }
            }
        }
        false
    }

    fn damage_hit_tile(&self, pos: Vec2, map: &mut GameMap, tile_size: f32) {
        let epsilon = 0.001;
        let left   = (pos.x / tile_size).floor() as i32;
        let right  = ((pos.x + self.size - epsilon) / tile_size).floor() as i32;
        let top    = (pos.y / tile_size).floor() as i32;
        let bottom = ((pos.y + self.size - epsilon) / tile_size).floor() as i32;

        for ty in top..=bottom {
            for tx in left..=right {
                if map.is_blocked_for_bullet(tx, ty) {
                    map.damage_at(tx, ty, self.pierces_steel);
                    return;
                }
            }
        }
    }

    pub fn draw(&self) {
        if !self.alive { return; }
        draw_rectangle(self.pos.x, self.pos.y, self.size, self.size, YELLOW);
    }
}