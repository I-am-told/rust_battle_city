use macroquad::prelude::*;
use crate::entities::tank::{Tank, Direction};

pub struct EnemySpawner {
    /// Через сколько секунд после последнего спавна рожать следующего.
    pub spawn_interval: f32,
    /// Сколько ждать при занятой точке спавна, прежде чем проверить снова.
    pub retry_interval: f32,
    timer: f32,
    /// Индекс текущей точки спавна в очереди.
    next_point: usize,
    pub remaining_to_spawn: u32,
    /// Максимум врагов ОДНОВРЕМЕННО на поле.
    /// ЗАДЕЛ НА БУДУЩЕЕ: в Battle City это зависит от уровня игры —
    /// на высоких уровнях одновременно может быть больше врагов.
    /// Сейчас задаётся при создании спавнера, позже будем менять при
    /// переходе между уровнями.
    pub max_on_field: u32,
    next_id: u32,
    spawn_points: Vec<IVec2>,
}

impl EnemySpawner {
    pub fn new(total_enemies: u32, max_on_field: u32, grid_width: i32) -> Self {
        Self {
            spawn_interval: 3.0,
            retry_interval: 0.5,
            timer: 1.5,
            next_point: 0,
            remaining_to_spawn: total_enemies,
            max_on_field,
            next_id: 1,
            spawn_points: Self::compute_spawn_points(grid_width),
        }
    }

    /// Три точки у верхнего края поля (y = 0):
    /// левая (x = 0), центральная, правая (x = grid_width - 2).
    /// Танк 2×2 тайла, поэтому координаты — левый верхний угол танка.
    fn compute_spawn_points(grid_width: i32) -> Vec<IVec2> {
        let left_x = 0;
        let right_x = (grid_width - 2).max(left_x);
        let center_x = ((grid_width - 2) / 2).max(left_x).min(right_x);
        let y = 0;

        vec![
            ivec2(left_x, y),
            ivec2(center_x, y),
            ivec2(right_x, y),
        ]
    }

    /// `all_tanks` — ВСЕ танки на поле (враги И игрок), чтобы корректно
    /// проверять занятость точки спавна. Раньше передавались только враги,
    /// из-за чего враг мог заспавниться в игрока.
    pub fn update(
        &mut self,
        dt: f32,
        current_enemies: usize,
        all_tanks: &[Tank],
        tile_size: f32,
        tank_size: f32,
        tank_speed: f32,
    ) -> Option<Tank> {
        if self.remaining_to_spawn == 0 {
            return None;
        }
        if (current_enemies as u32) >= self.max_on_field {
            return None;
        }

        self.timer -= dt;
        if self.timer > 0.0 {
            return None;
        }

        // Берём ТОЛЬКО текущую точку из очереди — без перебора других.
        let tile = self.spawn_points[self.next_point];
        let pos = vec2(tile.x as f32 * tile_size, tile.y as f32 * tile_size);

        if Self::point_occupied_by_tank(pos, tank_size, all_tanks) {
            // Точка занята (врагом ИЛИ игроком) — откладываем, указатель НЕ двигаем.
            self.timer = self.retry_interval;
            return None;
        }

        // Точка свободна — спавним.
        self.timer = self.spawn_interval;
        self.next_point = (self.next_point + 1) % self.spawn_points.len();
        self.remaining_to_spawn -= 1;

        let id = self.next_id;
        self.next_id += 1;

        Some(Tank::new_enemy(pos, Direction::Down, tank_size, tank_speed, id))
    }

    fn point_occupied_by_tank(pos: Vec2, tank_size: f32, tanks: &[Tank]) -> bool {
        for t in tanks {
            if !t.alive {
                continue;
            }
            let dx = (t.pos.x - pos.x).abs();
            let dy = (t.pos.y - pos.y).abs();
            if dx < tank_size && dy < tank_size {
                return true;
            }
        }
        false
    }
}