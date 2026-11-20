use macroquad::prelude::*;
use crate::map::GameMap;
use crate::entities::bullet::Bullet;
use crate::entities::enemy_kind::EnemyKind;

pub use crate::entities::direction::Direction;

/// Помощь при повороте (для игрока). Задаётся в config.ron: `turn_assist`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TurnAssist {
    /// Как в оригинале: при повороте танк выравнивается до ближайшего тайла.
    Off,
    /// Если в новую сторону не проехать, а через тайл впереди можно —
    /// танк мгновенно переносится на этот тайл и поворачивает.
    Jump,
    /// То же, что Jump, но танк доезжает до точки поворота плавно.
    Smooth,
}

impl TurnAssist {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "off" => Some(TurnAssist::Off),
            "jump" => Some(TurnAssist::Jump),
            "smooth" => Some(TurnAssist::Smooth),
            _ => None,
        }
    }
}

/// За сколько секунд (не дольше) танк доезжает до точки поворота в режиме Smooth.
pub const TURN_SLIDE_TIME: f32 = 0.1;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Team {
    Player,
    Enemy,
}

#[derive(Clone)]
pub struct Tank {
    pub pos: Vec2,
    /// Позиция на начало кадра — для отката при коллизии танк-танк.
    pub prev_pos: Vec2,
    pub direction: Direction,
    pub size: f32,
    pub speed: f32,
    pub max_active_bullets: u8,
    pub id: u32,
    pub pierces_steel: bool,
    pub team: Team,
    pub alive: bool,
    pub invulnerable: bool,
    pub is_bonus: bool,
    pub frozen: bool,
    pub rank: u8,
    pub bullet_speed_multiplier: f32,
    pub kind: Option<EnemyKind>,
    pub hp: u8,
    pub max_hp: u8,
    pub ai_dir_timer: f32,
    pub ai_shoot_timer: f32,
    pub turn_assist: TurnAssist,
    /// Номер игрока (0 — P1, 1 — P2); для врагов не используется.
    pub player_slot: u8,
    /// Точка, к которой танк плавно доезжает при повороте (режим Smooth).
    pub slide_target: Option<Vec2>,
    pub slide_speed: f32,
}

impl Tank {
    pub fn new(pos: Vec2, direction: Direction, size: f32, speed: f32) -> Self {
        Self {
            pos,
            prev_pos: pos,
            direction,
            size,
            speed,
            max_active_bullets: 1,
            id: 0,
            pierces_steel: false,
            team: Team::Player,
            alive: true,
            invulnerable: false,
            is_bonus: false,
            frozen: false,
            rank: 0,
            bullet_speed_multiplier: 1.0,
            kind: None,
            hp: 1,
            max_hp: 1,
            ai_dir_timer: 0.0,
            ai_shoot_timer: 0.0,
            turn_assist: TurnAssist::Off,
            player_slot: 0,
            slide_target: None,
            slide_speed: 0.0,
        }
    }

    pub fn new_enemy(
        pos: Vec2,
        direction: Direction,
        size: f32,
        base_speed: f32,
        id: u32,
        is_bonus: bool,
        kind: EnemyKind,
    ) -> Self {
        let mut t = Self::new(pos, direction, size, base_speed * kind.speed_mult());
        t.team = Team::Enemy;
        t.id = id;
        t.is_bonus = is_bonus;
        t.kind = Some(kind);
        t.hp = kind.max_hp();
        t.max_hp = kind.max_hp();
        t.bullet_speed_multiplier = kind.bullet_speed_mult();
        t.ai_dir_timer = 0.0;
        t.ai_shoot_timer = 1.0;
        t
    }

    pub fn apply_rank(&mut self, rank: u8) {
        self.rank = rank;
        match rank {
            0 => {
                self.max_active_bullets = 1;
                self.pierces_steel = false;
                self.bullet_speed_multiplier = 1.0;
            }
            1 => {
                self.max_active_bullets = 1;
                self.pierces_steel = false;
                self.bullet_speed_multiplier = 1.5;
            }
            2 => {
                self.max_active_bullets = 2;
                self.pierces_steel = false;
                self.bullet_speed_multiplier = 1.5;
            }
            _ => {
                self.max_active_bullets = 2;
                self.pierces_steel = true;
                self.bullet_speed_multiplier = 1.5;
            }
        }
    }

    pub fn take_hit(&mut self) {
        if !self.alive { return; }
        if self.invulnerable { return; }
        self.hp = self.hp.saturating_sub(1);
        if self.hp == 0 {
            self.alive = false;
        }
    }

    fn tile_origin(&self, tile_size: f32) -> IVec2 {
        let tx = (self.pos.x / tile_size).round() as i32;
        let ty = (self.pos.y / tile_size).round() as i32;
        ivec2(tx, ty)
    }

    /// Площадь пересечения танка (стоящего в `pos`) с другим танком.
    fn overlap_area(&self, pos: Vec2, other: &Tank) -> f32 {
        let w = (pos.x + self.size).min(other.pos.x + other.size) - pos.x.max(other.pos.x);
        let h = (pos.y + self.size).min(other.pos.y + other.size) - pos.y.max(other.pos.y);
        if w <= 0.0 || h <= 0.0 { 0.0 } else { w * h }
    }

    /// Мешает ли `other` встать в `pos`.
    ///
    /// Танк не может въехать в другой танк, но если они УЖЕ перекрываются
    /// (появление, рывок при повороте и т.п.), разрешаем движение, которое
    /// не увеличивает перекрытие, — иначе оба застрянут навсегда.
    fn blocked_by_tank(&self, pos: Vec2, other: &Tank) -> bool {
        if other.id == self.id || !other.alive {
            return false;
        }
        let new_area = self.overlap_area(pos, other);
        if new_area <= 0.0 {
            return false;
        }
        let cur_area = self.overlap_area(self.pos, other);
        new_area > cur_area + 0.001
    }

    fn is_clear_at_grid(&self, origin: IVec2, map: &GameMap, others: &[Tank], tile_size: f32) -> bool {
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
        let probe = vec2(origin.x as f32 * tile_size, origin.y as f32 * tile_size);
        for other in others {
            if self.blocked_by_tank(probe, other) {
                return false;
            }
        }
        true
    }

    fn is_clear_at_px(&self, pos: Vec2, map: &GameMap, others: &[Tank], tile_size: f32) -> bool {
        let playfield_w = map.width as f32 * tile_size;
        let playfield_h = map.height as f32 * tile_size;

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

        for other in others {
            if self.blocked_by_tank(pos, other) {
                return false;
            }
        }

        true
    }

    fn snap_to_grid(&mut self, tile_size: f32) {
        let tx = (self.pos.x / tile_size).round();
        let ty = (self.pos.y / tile_size).round();
        self.pos = vec2(tx * tile_size, ty * tile_size);
    }

    pub fn update(
        &mut self,
        dt: f32,
        desired: Option<Direction>,
        map: &GameMap,
        others: &[Tank],
        tile_size: f32,
    ) {
        if !self.alive { return; }

        self.prev_pos = self.pos;

        // Плавное выравнивание при повороте: пока танк доезжает,
        // ввод не обрабатываем (это ~0.1 с).
        if self.slide_target.is_some() {
            self.continue_slide(dt, map, others, tile_size);
            return;
        }

        let desired = match desired {
            Some(d) => d,
            None => return,
        };

        if desired != self.direction {
            self.turn_to(desired, map, others, tile_size);
        }

        let dist = self.speed * dt;
        self.move_along(self.direction, dist, map, others, tile_size);
    }

    pub fn update_ai(
        &mut self,
        dt: f32,
        map: &GameMap,
        others: &[Tank],
        tile_size: f32,
        bullet_speed: f32,
        bullet_size: f32,
        active_bullets: u8,
    ) -> Option<Bullet> {
        if !self.alive {
            return None;
        }
        if self.frozen {
            return None;
        }

        self.prev_pos = self.pos;

        self.ai_dir_timer -= dt;

        if self.ai_dir_timer <= 0.0 {
            self.choose_new_direction(map, others, tile_size);
            self.ai_dir_timer = 2.0 + rand::gen_range(0.0, 2.0);
        }

        let before = self.pos;
        let dist = self.speed * dt;
        self.move_along(self.direction, dist, map, others, tile_size);
        let moved = (self.pos - before).length_squared() > 0.01;

        if !moved {
            self.choose_new_direction(map, others, tile_size);
            self.ai_dir_timer = 2.0 + rand::gen_range(0.0, 2.0);
        }

        self.ai_shoot_timer -= dt;
        if self.ai_shoot_timer <= 0.0 {
            self.ai_shoot_timer = 0.3 + rand::gen_range(0.0, 1.2);
            return self.try_fire(active_bullets, map, tile_size, bullet_speed, bullet_size);
        }

        None
    }

    fn choose_new_direction(&mut self, map: &GameMap, others: &[Tank], tile_size: f32) {
        let origin = self.tile_origin(tile_size);

        let mut alternatives: Vec<Direction> = Vec::new();
        for d in Direction::all() {
            if d == self.direction {
                continue;
            }
            if self.is_clear_at_grid(origin + d.vector(), map, others, tile_size) {
                alternatives.push(d);
            }
        }

        if alternatives.is_empty() {
            let back = self.direction.opposite();
            if self.is_clear_at_grid(origin + back.vector(), map, others, tile_size) {
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

    fn turn_to(&mut self, new_dir: Direction, map: &GameMap, others: &[Tank], tile_size: f32) {
        // Разворот на 180° — без выравнивания: танк остаётся на своей оси.
        if new_dir == self.direction.opposite() {
            self.direction = new_dir;
            return;
        }

        // Ближайшая точка сетки (как в оригинале).
        let snapped = vec2(
            (self.pos.x / tile_size).round() * tile_size,
            (self.pos.y / tile_size).round() * tile_size,
        );
        let origin = ivec2(
            (snapped.x / tile_size).round() as i32,
            (snapped.y / tile_size).round() as i32,
        );

        let mut target = snapped;
        if self.turn_assist != TurnAssist::Off
            && !self.is_clear_at_grid(origin + new_dir.vector(), map, others, tile_size)
        {
            // В новую сторону не проехать. Если через тайл впереди поворот
            // свободен — поворачиваем там (помощь, чтобы легче «попадать»).
            let ahead = origin + self.direction.vector();
            if self.is_clear_at_grid(ahead, map, others, tile_size)
                && self.is_clear_at_grid(ahead + new_dir.vector(), map, others, tile_size)
            {
                target = vec2(ahead.x as f32 * tile_size, ahead.y as f32 * tile_size);
            }
        }

        self.direction = new_dir;

        if self.turn_assist == TurnAssist::Smooth {
            let dist = (target - self.pos).length();
            if dist > 0.01 {
                self.slide_target = Some(target);
                self.slide_speed = (dist / TURN_SLIDE_TIME).max(self.speed);
            }
        } else {
            self.pos = target;
        }
    }

    /// Шаг плавного выравнивания. Если путь перекрыли (другой танк) —
    /// выравнивание прерывается, танк остаётся где стоит.
    fn continue_slide(&mut self, dt: f32, map: &GameMap, others: &[Tank], tile_size: f32) {
        let Some(target) = self.slide_target else { return };
        let delta = target - self.pos;
        let step = self.slide_speed * dt;
        let next = if delta.length() <= step {
            target
        } else {
            self.pos + delta.normalize() * step
        };
        if self.is_clear_at_px(next, map, others, tile_size) {
            self.pos = next;
            if next == target {
                self.slide_target = None;
            }
        } else {
            self.slide_target = None;
        }
    }

    fn move_along(
        &mut self,
        dir: Direction,
        dist: f32,
        map: &GameMap,
        others: &[Tank],
        tile_size: f32,
    ) {
        let axis = vec2(dir.vector().x as f32, dir.vector().y as f32);
        let target = self.pos + axis * dist;

        if self.is_clear_at_px(target, map, others, tile_size) {
            self.pos = target;
            return;
        }

        let chunk: f32 = 0.5;
        let mut moved: f32 = 0.0;
        while moved < dist.abs() {
            let advance = chunk.min(dist.abs() - moved);
            let try_pos = self.pos + axis * (moved + advance);
            if self.is_clear_at_px(try_pos, map, others, tile_size) {
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
        if !self.alive {
            return None;
        }
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
        let actual_speed = bullet_speed * self.bullet_speed_multiplier;
        let mut bullet = Bullet::new(
            pos,
            self.direction,
            actual_speed,
            bullet_size,
            self.id,
            self.team,
        );
        bullet.pierces_steel = self.pierces_steel;
        Some(bullet)
    }

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
}