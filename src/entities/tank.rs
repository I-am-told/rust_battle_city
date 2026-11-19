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

    pub fn all() -> [Direction; 4] {
        [Direction::Up, Direction::Down, Direction::Left, Direction::Right]
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::Up    => Direction::Down,
            Direction::Down  => Direction::Up,
            Direction::Left  => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Team {
    Player,
    Enemy,
}

pub struct Tank {
    pub pos: Vec2,
    pub direction: Direction,
    pub size: f32,
    pub speed: f32,
    pub max_active_bullets: u8,
    pub id: u32,
    pub pierces_steel: bool,
    pub team: Team,
    pub alive: bool,
    // --- ИИ ---
    pub ai_dir_timer: f32,
    pub ai_shoot_timer: f32,
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
            pierces_steel: false,
            team: Team::Player,
            alive: true,
            ai_dir_timer: 0.0,
            ai_shoot_timer: 0.0,
        }
    }

    pub fn new_enemy(
        pos: Vec2,
        direction: Direction,
        size: f32,
        speed: f32,
        id: u32,
    ) -> Self {
        let mut t = Self::new(pos, direction, size, speed);
        t.team = Team::Enemy;
        t.id = id;
        t.ai_dir_timer = 0.0;
        t.ai_shoot_timer = 1.0;
        t
    }

    fn tile_origin(&self, tile_size: f32) -> IVec2 {
        let tx = (self.pos.x / tile_size).round() as i32;
        let ty = (self.pos.y / tile_size).round() as i32;
        ivec2(tx, ty)
    }

    /// Свободна ли (в терминах тайлов) позиция, если танк стоит от `origin` (2×2).
    /// Включает проверку выхода за границы поля.
    fn is_clear_at_grid(&self, origin: IVec2, map: &GameMap) -> bool {
        // Границы поля — жёсткая стена.
        if origin.x < 0 || origin.y < 0 {
            return false;
        }
        if origin.x + 2 > map.width || origin.y + 2 > map.height {
            return false;
        }
        for dy in 0..2 {
            for dx in 0..2 {
                if map.is_blocked_for_tank(origin.x + dx, origin.y + dy) {
                    return false;
                }
            }
        }
        true
    }

    /// Свободна ли (в пикселях) позиция `pos`. Включает проверку границ поля.
    fn is_clear_at_px(&self, pos: Vec2, map: &GameMap, tile_size: f32) -> bool {
        let playfield_w = map.width as f32 * tile_size;
        let playfield_h = map.height as f32 * tile_size;

        // Границы поля.
        if pos.x < 0.0 || pos.y < 0.0 {
            return false;
        }
        if pos.x + self.size > playfield_w || pos.y + self.size > playfield_h {
            return false;
        }

        let epsilon = 0.001;
        let left   = (pos.x / tile_size).floor() as i32;
        let right  = ((pos.x + self.size - epsilon) / tile_size).floor() as i32;
        let top    = (pos.y / tile_size).floor() as i32;
        let bottom = ((pos.y + self.size - epsilon) / tile_size).floor() as i32;

        for ty in top..=bottom {
            for tx in left..=right {
                if map.is_blocked_for_tank(tx, ty) {
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

    pub fn update_ai(
        &mut self,
        dt: f32,
        map: &GameMap,
        tile_size: f32,
        bullet_speed: f32,
        bullet_size: f32,
        active_bullets: u8,
    ) -> Option<Bullet> {
        if !self.alive {
            return None;
        }

        self.ai_dir_timer -= dt;

        if self.ai_dir_timer <= 0.0 {
            self.choose_new_direction(map, tile_size);
            self.ai_dir_timer = 2.0 + rand::gen_range(0.0, 2.0);
        }

        let before = self.pos;
        let dist = self.speed * dt;
        self.move_along(self.direction, dist, map, tile_size);
        let moved = (self.pos - before).length_squared() > 0.01;

        if !moved {
            self.choose_new_direction(map, tile_size);
            self.ai_dir_timer = 2.0 + rand::gen_range(0.0, 2.0);
        }

        self.ai_shoot_timer -= dt;
        if self.ai_shoot_timer <= 0.0 {
            self.ai_shoot_timer = 1.5 + rand::gen_range(0.0, 1.5);
            return self.try_fire(active_bullets, map, tile_size, bullet_speed, bullet_size);
        }

        None
    }

    fn choose_new_direction(&mut self, map: &GameMap, tile_size: f32) {
        let origin = self.tile_origin(tile_size);

        let mut alternatives: Vec<Direction> = Vec::new();
        for d in Direction::all() {
            if d == self.direction {
                continue;
            }
            if self.is_clear_at_grid(origin + d.vector(), map) {
                alternatives.push(d);
            }
        }

        if alternatives.is_empty() {
            let back = self.direction.opposite();
            if self.is_clear_at_grid(origin + back.vector(), map) {
                self.direction = back;
            }
            return;
        }

        let idx = rand::gen_range(0, alternatives.len());
        let new_dir = alternatives[idx];

        if new_dir != self.direction {
            let on_grid = (self.pos.x / tile_size).fract().abs() < 0.001
                && (self.pos.y / tile_size).fract().abs() < 0.001;
            if !on_grid {
                self.snap_to_grid(tile_size);
            }
            self.direction = new_dir;
        }
    }

    fn turn_to(&mut self, new_dir: Direction, map: &GameMap, tile_size: f32) {
        self.snap_to_grid(tile_size);
        let origin = self.tile_origin(tile_size);

        if self.is_clear_at_grid(origin + new_dir.vector(), map) {
            self.direction = new_dir;
            return;
        }

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
        let muzzle = center + dir_vec_f * (self.size / 2.0 + 1.0);

        let mut spawn = muzzle;
        let step_back = 0.5;
        let mut dist_back = 0.0;
        let max_back = (self.size / 2.0) + 2.0;

        while !self.is_bullet_point_free(spawn, map, tile_size) {
            dist_back += step_back;
            if dist_back > max_back {
                return None;
            }
            spawn = muzzle - dir_vec_f * dist_back;
        }

        let pos = spawn - vec2(bullet_size / 2.0, bullet_size / 2.0);
        let mut bullet = Bullet::new(pos, self.direction, bullet_speed, bullet_size, self.id);
        bullet.pierces_steel = self.pierces_steel;
        Some(bullet)
    }

    /// Свободна ли точка `p` для спавна пули.
    /// Вода не мешает, границы поля — мешают.
    fn is_bullet_point_free(&self, p: Vec2, map: &GameMap, tile_size: f32) -> bool {
        let playfield_w = map.width as f32 * tile_size;
        let playfield_h = map.height as f32 * tile_size;
        if p.x < 0.0 || p.y < 0.0 || p.x >= playfield_w || p.y >= playfield_h {
            return false;
        }
        let tx = (p.x / tile_size).floor() as i32;
        let ty = (p.y / tile_size).floor() as i32;
        !map.is_blocked_for_bullet(tx, ty)
    }

    pub fn draw(&self) {
        if !self.alive { return; }
        let color = match self.team {
            Team::Player => GREEN,
            Team::Enemy  => RED,
        };
        draw_rectangle(self.pos.x, self.pos.y, self.size, self.size, color);
        draw_rectangle_lines(self.pos.x, self.pos.y, self.size, self.size, 4.0, WHITE);

        let center = self.pos + vec2(self.size / 2.0, self.size / 2.0);
        let v = self.direction.vector();
        let tip = center + vec2(v.x as f32, v.y as f32) * (self.size / 2.0);
        draw_line(center.x, center.y, tip.x, tip.y, self.size * 0.2, WHITE);
    }
}