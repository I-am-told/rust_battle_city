use macroquad::prelude::*;
use crate::config::Gameplay;
use crate::entities::tank::{Direction, Tank};
use crate::input::{DirectionInput, KeyBindings};

/// Id танков игроков. Враги нумеруются с 1, поэтому у игроков —
/// большие id, чтобы никогда не пересечься.
pub const PLAYER_ID_BASE: u32 = 1_000_000;


/// Игрок: танк + его управление + состояние возрождения.
pub struct Player {
    pub tank: Tank,
    pub keys: KeyBindings,
    pub dir_input: DirectionInput,
    pub spawn: Vec2,
    pub spawn_tile: IVec2,
    /// Ждёт возрождения: сколько секунд осталось. None — жив
    /// (или ещё ни разу не погибал).
    pub respawn_timer: Option<f32>,
    pub invuln_timer: f32,
    /// Горит звёздочка появления: сколько секунд прошло. None — не горит.
    pub star: Option<f32>,
    /// Очки за всю игру.
    pub score: u32,
    /// Сколько врагов каждого типа уничтожено на этом уровне
    /// (индекс — EnemyKind::index), для экрана подсчёта.
    pub kills: [u32; 4],
    /// Настройки из config.ron (gameplay).
    tuning: Gameplay,
}

impl Player {
    pub fn new(tank: Tank, keys: KeyBindings, spawn_tile: IVec2, tile_size: f32, tuning: &Gameplay) -> Self {
        let spawn = vec2(spawn_tile.x as f32 * tile_size, spawn_tile.y as f32 * tile_size);
        Self {
            tank,
            keys,
            dir_input: DirectionInput::default(),
            spawn,
            spawn_tile,
            respawn_timer: None,
            invuln_timer: 0.0,
            star: None,
            score: 0,
            kills: [0; 4],
            tuning: *tuning,
        }
    }

    /// Начислить очки. Возвращает, сколько дополнительных жизней
    /// заработано (каждые `extra_life_every` очков; 0 — не давать).
    pub fn add_score(&mut self, points: u32) -> u32 {
        let every = self.tuning.extra_life_every;
        let before = self.score;
        self.score = self.score.saturating_add(points);
        // checked_div: при every == 0 жизни за очки не даются.
        match (self.score.checked_div(every), before.checked_div(every)) {
            (Some(now), Some(was)) => now - was,
            _ => 0,
        }
    }

    /// Начать появление: на точке мерцает звёздочка, танка ещё нет.
    /// Через `spawn_star_time` танк появится с защитным полем (`tick_star`).
    pub fn begin_spawn(&mut self) {
        self.tank.alive = false;
        self.respawn_timer = None;
        self.star = Some(0.0);
    }

    /// Игрок в игре: танк на поле или вот-вот появится (горит звезда).
    pub fn in_game(&self) -> bool {
        self.tank.alive || self.star.is_some()
    }

    /// Звёздочка появления. Когда догорит — танк появляется.
    pub fn tick_star(&mut self, dt: f32) {
        if let Some(t) = self.star {
            let t = t + dt;
            if t >= self.tuning.spawn_star_time {
                self.star = None;
                self.respawn();
            } else {
                self.star = Some(t);
            }
        }
    }

    /// Появиться на точке появления с защитным полем.
    fn respawn(&mut self) {
        self.tank.pos = self.spawn;
        self.tank.prev_pos = self.spawn;
        self.tank.slide_target = None;
        self.tank.direction = Direction::Up;
        self.tank.alive = true;
        self.tank.hp = self.tank.max_hp;
        self.invuln_timer = 0.0;
        self.give_shield(self.tuning.spawn_shield);
        self.respawn_timer = None;
    }

    /// Попадание пули. Возвращает true, если танк погиб.
    pub fn hit(&mut self) -> bool {
        if !self.tank.alive || self.tank.invulnerable {
            return false;
        }
        self.tank.take_hit();
        if self.tank.alive {
            return false;
        }
        // Как в оригинале: после гибели ранг (звёзды) сбрасывается.
        self.tank.apply_rank(0);
        self.respawn_timer = Some(self.tuning.respawn_delay);
        true
    }

    /// Включить защитное поле на `secs` секунд (не сокращает уже идущее).
    pub fn give_shield(&mut self, secs: f32) {
        self.tank.invulnerable = true;
        self.invuln_timer = self.invuln_timer.max(secs);
    }

    /// Таймер неуязвимости.
    pub fn tick_shield(&mut self, dt: f32) {
        if self.invuln_timer > 0.0 {
            self.invuln_timer -= dt;
            if self.invuln_timer <= 0.0 {
                self.tank.invulnerable = false;
            }
        }
    }
}
