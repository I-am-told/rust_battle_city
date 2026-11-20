use macroquad::prelude::*;
use crate::entities::effects::SPAWN_STAR_TIME;
use crate::entities::tank::{Direction, Tank};
use crate::input::{DirectionInput, KeyBindings};

/// Id танков игроков. Враги нумеруются с 1, поэтому у игроков —
/// большие id, чтобы никогда не пересечься.
pub const PLAYER_ID_BASE: u32 = 1_000_000;

/// Время до возрождения после гибели, секунд.
pub const RESPAWN_DELAY: f32 = 1.0;
/// Неуязвимость после возрождения, секунд.
pub const SPAWN_SHIELD: f32 = 3.0;
/// Каждые столько очков — дополнительная жизнь (как в оригинале).
pub const EXTRA_LIFE_EVERY: u32 = 20_000;

/// Неуязвимость от бонуса-каски, секунд (как в оригинале — заметно дольше).
pub const HELMET_SHIELD: f32 = 10.0;

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
}

impl Player {
    pub fn new(tank: Tank, keys: KeyBindings, spawn_tile: IVec2, tile_size: f32) -> Self {
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
        }
    }

    /// Начислить очки. Возвращает, сколько дополнительных жизней
    /// заработано (каждые EXTRA_LIFE_EVERY очков).
    pub fn add_score(&mut self, points: u32) -> u32 {
        let before = self.score / EXTRA_LIFE_EVERY;
        self.score = self.score.saturating_add(points);
        self.score / EXTRA_LIFE_EVERY - before
    }

    /// Начать появление: на точке мерцает звёздочка, танка ещё нет.
    /// Через SPAWN_STAR_TIME танк появится с защитным полем (`tick_star`).
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
            if t >= SPAWN_STAR_TIME {
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
        self.give_shield(SPAWN_SHIELD);
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
        self.respawn_timer = Some(RESPAWN_DELAY);
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
