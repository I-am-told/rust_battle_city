use macroquad::prelude::*;
use crate::input::KeyBindings;
use crate::map::GameMap;
use crate::entities::bullet::Bullet;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn vector(self) -> IVec2 {
        match self {
            Direction::Up    => ivec2( 0, -1),
            Direction::Down  => ivec2( 0,  1),
            Direction::Left  => ivec2(-1,  0),
            Direction::Right => ivec2( 1,  0),
        }
    }
}

pub struct Tank {
    pub pos: Vec2,
    pub direction: Direction,
    pub size: f32,
    pub speed: f32,
    /// Сколько пуль танк может держать в воздухе одновременно.
    /// Пока 1. С ростом ранга будем увеличивать (2, 3, ...).
    pub max_active_bullets: u8,
    /// Уникальный идентификатор танка (0 — игрок; врагам дадим 1, 2, ...).
    pub id: u32,
}

impl Tank {
    pub fn new(pos: Vec2, direction: Direction, size: f32, speed: f32) -> Self {
        Self {
            pos,
            direction,
            size,
            speed,
            max_active_bullets: 1,
            id: 0,
        }
    }

    fn tile_origin(&self, tile_size: f32) -> IVec2 {
        let tx = (self.pos.x / tile_size).round() as i32;
        let ty = (self.pos.y / tile_size).round() as i32;
        ivec2(tx, ty)
    }

    fn is_clear_at_grid(&self, origin: IVec2, map: &GameMap) -> bool {
        for dy in 0..2 {
            for dx in 0..2 {
                if map.is_blocked(origin.x + dx, origin.y + dy) {
                    return false;
                }
            }
        }
        true
    }

    fn is_clear_at_px(&self, pos: Vec2, map: &GameMap, tile_size: f32) -> bool {
        let epsilon = 0.001;
        let left   = (pos.x / tile_size).floor() as i32;
        let right  = ((pos.x + self.size - epsilon) / tile_size).floor() as i32;
        let top    = (pos.y / tile_size).floor() as i32;
        let bottom = ((pos.y + self.size - epsilon) / tile_size).floor() as i32;

        for ty in top..=bottom {
            for tx in left..=right {
                if map.is_blocked(tx, ty) {
                    return false;
                }
            }
        }
        true
    }

    fn snap_to_grid(&mut self, tile_size: f32) {
        let tx = (self.pos.x / tile_size).round();
        let ty = (self.pos.y / tile_size).round();
        self.pos = vec2(tx * tile_size, ty * tile_size);
    }

    pub fn update(&mut self, dt: f32, keys: &KeyBindings, map: &GameMap, tile_size: f32) {
        let mut desired: Option<Direction> = None;
        if is_key_down(keys.up)    { desired = Some(Direction::Up); }
        if is_key_down(keys.down)  { desired = Some(Direction::Down); }
        if is_key_down(keys.left)  { desired = Some(Direction::Left); }
        if is_key_down(keys.right) { desired = Some(Direction::Right); }

        let desired = match desired {
            Some(d) => d,
            None => return,
        };

        if desired != self.direction {
            self.turn_to(desired, map, tile_size);
        }

        let dist = self.speed * dt;
        self.move_along(self.direction, dist, map, tile_size);
    }

    fn turn_to(&mut self, new_dir: Direction, map: &GameMap, tile_size: f32) {
        self.snap_to_grid(tile_size);

        let origin = self.tile_origin(tile_size);

        // Прямой поворот на месте.
        if self.is_clear_at_grid(origin + new_dir.vector(), map) {
            self.direction = new_dir;
            return;
        }

        // Пробуем срезать угол: сдвинуться на 1 тайл в старом направлении.
        let old_origin = origin + self.direction.vector();
        if self.is_clear_at_grid(old_origin, map) {
            let check_origin = old_origin + new_dir.vector();
            if self.is_clear_at_grid(check_origin, map) {
                self.pos = vec2(
                    old_origin.x as f32 * tile_size,
                    old_origin.y as f32 * tile_size,
                );
                self.direction = new_dir;
                return;
            }
        }

        // Не срезалось — только поворачиваем пушку.
        self.direction = new_dir;
    }

    fn move_along(&mut self, dir: Direction, dist: f32, map: &GameMap, tile_size: f32) {
        let axis = vec2(dir.vector().x as f32, dir.vector().y as f32);
        let target = self.pos + axis * dist;

        if self.is_clear_at_px(target, map, tile_size) {
            self.pos = target;
            return;
        }

        let chunk: f32 = 0.5;
        let mut moved: f32 = 0.0;
        while moved < dist.abs() {
            let advance = chunk.min(dist.abs() - moved);
            let try_pos = self.pos + axis * (moved + advance);
            if self.is_clear_at_px(try_pos, map, tile_size) {
                moved += advance;
            } else {
                break;
            }
        }
        self.pos = self.pos + axis * moved;
    }

    /// Пробует выстрелить. Возвращает `Some(Bullet)`, если выстрел удался.
    /// `active_bullets` — сколько пуль этого танка сейчас в воздухе.
    /// Если `active_bullets >= max_active_bullets` — не стреляем.
    /// Если дуло упирается в стену — не стреляем (чтобы пуля не спавнилась в блоке).
    pub fn try_fire(
        &self,
        active_bullets: u8,
        map: &GameMap,
        tile_size: f32,
        bullet_speed: f32,
        bullet_size: f32,
    ) -> Option<Bullet> {
        if active_bullets >= self.max_active_bullets {
            return None;
        }

        let dir_vec = self.direction.vector();
        let dir_vec_f = vec2(dir_vec.x as f32, dir_vec.y as f32);
        let center = self.pos + vec2(self.size / 2.0, self.size / 2.0);

        // Точка появления пули — чуть впереди дула.
        let muzzle = center + dir_vec_f * (self.size / 2.0 + 1.0);

        // Если дуло в стене — не стреляем.
        let mtx = (muzzle.x / tile_size).floor() as i32;
        let mty = (muzzle.y / tile_size).floor() as i32;
        if map.is_blocked(mtx, mty) {
            return None;
        }

        // Позиция верхнего левого угла пули, чтобы её центр был в muzzle.
        let pos = muzzle - vec2(bullet_size / 2.0, bullet_size / 2.0);

        Some(Bullet::new(pos, self.direction, bullet_speed, bullet_size, self.id))
    }

    pub fn draw(&self) {
        draw_rectangle(self.pos.x, self.pos.y, self.size, self.size, GREEN);
        draw_rectangle_lines(self.pos.x, self.pos.y, self.size, self.size, 4.0, WHITE);

        let center = self.pos + vec2(self.size / 2.0, self.size / 2.0);
        let v = self.direction.vector();
        let tip = center + vec2(v.x as f32, v.y as f32) * (self.size / 2.0);
        draw_line(center.x, center.y, tip.x, tip.y, self.size * 0.2, RED);
    }
}