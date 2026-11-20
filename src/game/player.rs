use macroquad::prelude::*;
use crate::entities::tank::{Direction, Tank};
use crate::input::{DirectionInput, KeyBindings};

/// Id танков игроков. Враги нумеруются с 1, поэтому у игроков —
/// большие id, чтобы никогда не пересечься.
pub const PLAYER_ID_BASE: u32 = 1_000_000;

/// Время до возрождения после гибели, секунд.
pub const RESPAWN_DELAY: f32 = 1.0;
/// Неуязвимость после возрождения, секунд.
pub const SPAWN_SHIELD: f32 = 3.0;

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
        }
    }

    /// Возродить на точке появления с защитным полем.
    pub fn respawn(&mut self) {
        self.tank.pos = self.spawn;
        self.tank.prev_pos = self.spawn;
        self.tank.slide_target = None;
        self.tank.direction = Direction::Up;
        self.tank.alive = true;
        self.tank.hp = self.tank.max_hp;
        self.tank.invulnerable = true;
        self.invuln_timer = SPAWN_SHIELD;
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
