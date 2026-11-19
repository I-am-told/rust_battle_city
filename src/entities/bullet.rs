use macroquad::prelude::*;
use crate::map::GameMap;
use crate::entities::tank::Direction;

pub struct Bullet {
    pub pos: Vec2,
    pub direction: Direction,
    pub speed: f32,
    pub size: f32,
    /// Кто выпустил (для подсчёта активных пуль и будущих коллизий с врагами).
    pub owner_id: u32,
    /// Флаг «жива». При попадании в блок/границу ставим false, в main удаляем.
    pub alive: bool,
}

impl Bullet {
    pub fn new(pos: Vec2, direction: Direction, speed: f32, size: f32, owner_id: u32) -> Self {
        Self { pos, direction, speed, size, owner_id, alive: true }
    }

    pub fn update(&mut self, dt: f32, map: &GameMap, tile_size: f32) {
        if !self.alive { return; }

        let v = self.direction.vector();
        let step = vec2(v.x as f32, v.y as f32) * self.speed * dt;

        // Двигаемся небольшими шагами, чтобы на быстрой пуле не «проскочить» тайл.
        let max_step = tile_size * 0.5;
        let total = step.length();
        let steps = (total / max_step).ceil().max(1.0) as i32;
        let sub_step = step / steps as f32;

        for _ in 0..steps {
            let next = self.pos + sub_step;
            if self.collides_at(next, map, tile_size) {
                self.alive = false;
                return;
            }
            self.pos = next;
        }
    }

    fn collides_at(&self, pos: Vec2, map: &GameMap, tile_size: f32) -> bool {
        let epsilon = 0.001;
        let left   = (pos.x / tile_size).floor() as i32;
        let right  = ((pos.x + self.size - epsilon) / tile_size).floor() as i32;
        let top    = (pos.y / tile_size).floor() as i32;
        let bottom = ((pos.y + self.size - epsilon) / tile_size).floor() as i32;

        for ty in top..=bottom {
            for tx in left..=right {
                if map.is_blocked(tx, ty) {
                    return true;
                }
            }
        }
        false
    }

    pub fn draw(&self) {
        if !self.alive { return; }
        draw_rectangle(self.pos.x, self.pos.y, self.size, self.size, YELLOW);
    }
}