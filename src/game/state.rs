use macroquad::prelude::*;
use crate::config::Config;
use crate::input::KeyBindings;
use crate::map::GameMap;
use crate::entities::tank::{Tank, Direction};
use crate::entities::bullet::Bullet;
use crate::entities::spawner::EnemySpawner;
use crate::entities::bonus::Bonus;
use crate::game::bonuses::apply_bonus;
use crate::game::level::LevelConfig;

pub const BONUS_LIFETIME: f32 = 15.0;
pub const MAX_RANK: u8 = 3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GameState {
    Playing,
    Victory,
    Defeat,
}

pub struct Game {
    pub tile_size: f32,
    pub tank_size: f32,
    pub keys_p1: KeyBindings,
    pub bullet_speed: f32,
    pub bullet_size: f32,
    pub tank_speed: f32,

    pub map: GameMap,
    pub player: Tank,
    pub player_spawn: Vec2,
    pub player_spawn_tile: IVec2,
    pub player_lives: u32,
    pub respawn_timer: f32,
    pub invuln_timer: f32,

    pub enemies: Vec<Tank>,
    pub spawner: EnemySpawner,

    pub bullets: Vec<Bullet>,
    pub bonuses: Vec<Bonus>,

    pub freeze_timer: f32,
    pub shovel_timer: f32,
    pub shovel_tiles: Vec<IVec2>,

    pub state: GameState,
    pub picked_bonuses: u32,
    /// Текущий уровень (1-based). При победе увеличивается.
    pub current_level: u32,
    /// Конфиг (нужен для пересоздания карты/спавнера при переходе на след. уровень).
    pub cfg_grid_width: i32,
    pub cfg_grid_height: i32,
}

impl Game {
    pub fn new(cfg: &Config) -> Self {
        Self::new_with_progress(cfg, 1, 3, 0)
    }

    /// Создаёт игру на уровне `level`, с сохранением прогресса
    /// (жизни и ранг игрока переносятся между уровнями).
    pub fn new_with_progress(
        cfg: &Config,
        level: u32,
        player_lives: u32,
        player_rank: u8,
    ) -> Self {
        let keys_p1: KeyBindings = (&cfg.player1_keys).into();
        let map = GameMap::new(cfg.grid_width, cfg.grid_height);
        let tank_size = cfg.tile_size * 2.0;

        let player_spawn_tile = {
            if let Some(base) = map.base_origin {
                let x = (base.x - 4).max(0);
                let y = (base.y - 1).max(0);
                ivec2(x, y)
            } else {
                ivec2(1, cfg.grid_height - 3)
            }
        };
        let player_spawn = vec2(
            player_spawn_tile.x as f32 * cfg.tile_size,
            player_spawn_tile.y as f32 * cfg.tile_size,
        );

        let mut player = Tank::new(
            player_spawn,
            Direction::Up,
            tank_size,
            cfg.tank_speed,
        );
        player.id = 0;
        player.apply_rank(player_rank);

        let level_cfg = LevelConfig::for_level(level);
        let spawner = EnemySpawner::new(level_cfg, cfg.grid_width);

        Self {
            tile_size: cfg.tile_size,
            tank_size,
            keys_p1,
            bullet_speed: cfg.bullet_speed,
            bullet_size: cfg.bullet_size,
            tank_speed: cfg.tank_speed,
            map,
            player,
            player_spawn,
            player_spawn_tile,
            player_lives,
            respawn_timer: 0.0,
            invuln_timer: 0.0,
            enemies: Vec::new(),
            spawner,
            bullets: Vec::new(),
            bonuses: Vec::new(),
            freeze_timer: 0.0,
            shovel_timer: 0.0,
            shovel_tiles: Vec::new(),
            state: GameState::Playing,
            picked_bonuses: 0,
            current_level: level,
            cfg_grid_width: cfg.grid_width,
            cfg_grid_height: cfg.grid_height,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.state != GameState::Playing {
            return;
        }
        self.update_playing(dt);
    }

    fn player_spawn_occupied(&self) -> bool {
        let px = self.player_spawn.x;
        let py = self.player_spawn.y;
        for e in self.enemies.iter() {
            if !e.alive { continue; }
            let dx = (e.pos.x - px).abs();
            let dy = (e.pos.y - py).abs();
            if dx < self.tank_size && dy < self.tank_size {
                return true;
            }
        }
        false
    }

    fn update_playing(&mut self, dt: f32) {
        if self.invuln_timer > 0.0 {
            self.invuln_timer -= dt;
            if self.invuln_timer <= 0.0 {
                self.player.invulnerable = false;
            }
        }

        if !self.player.alive && self.respawn_timer > 0.0 {
            self.respawn_timer -= dt;
            if self.respawn_timer <= 0.0 {
                if self.player_spawn_occupied() {
                    self.respawn_timer = 0.2;
                } else {
                    self.player.pos = self.player_spawn;
                    self.player.direction = Direction::Up;
                    self.player.alive = true;
                    self.player.invulnerable = true;
                    self.invuln_timer = 3.0;
                }
            }
        }

        if self.freeze_timer > 0.0 {
            self.freeze_timer -= dt;
            if self.freeze_timer < 0.0 { self.freeze_timer = 0.0; }
        }
        let frozen_now = self.freeze_timer > 0.0;
        for e in self.enemies.iter_mut() {
            e.frozen = frozen_now;
        }

        if self.shovel_timer > 0.0 {
            self.shovel_timer -= dt;
            if self.shovel_timer <= 0.0 {
                self.map.remove_shovel(&self.shovel_tiles);
                self.shovel_tiles.clear();
            }
        }

        // Снимок всех танков для проверок столкновений. После движения
        // каждого танка его позиция в снимке обновляется, чтобы следующие
        // танки видели актуальное положение и не въезжали друг в друга.
        // Индекс 0 — игрок, индекс i + 1 — враг i.
        let mut snapshot: Vec<Tank> = std::iter::once(self.player.clone())
            .chain(self.enemies.iter().cloned())
            .collect();

        self.player.update(dt, &self.keys_p1, &self.map, &snapshot, self.tile_size);
        snapshot[0].pos = self.player.pos;
        snapshot[0].direction = self.player.direction;

        // Мёртвый игрок (ожидание возрождения) не стреляет.
        if self.player.alive && is_key_pressed(self.keys_p1.fire) {
            let active = self.bullets
                .iter()
                .filter(|b| b.alive && b.owner_id == self.player.id)
                .count() as u8;

            if let Some(bullet) = self.player.try_fire(
                active,
                &self.map,
                self.tile_size,
                self.bullet_speed,
                self.bullet_size,
            ) {
                self.bullets.push(bullet);
            }
        }

        for (i, enemy) in self.enemies.iter_mut().enumerate() {
            if !enemy.alive || enemy.frozen { continue; }

            let active = self.bullets
                .iter()
                .filter(|b| b.alive && b.owner_id == enemy.id)
                .count() as u8;

            if let Some(bullet) = enemy.update_ai(
                dt,
                &self.map,
                &snapshot,
                self.tile_size,
                self.bullet_speed * 0.75,
                self.bullet_size,
                active,
            ) {
                self.bullets.push(bullet);
            }
            snapshot[i + 1].pos = enemy.pos;
            snapshot[i + 1].direction = enemy.direction;
        }

        let alive_enemies = self.enemies.iter().filter(|e| e.alive).count();
        let tanks_for_spawn: Vec<Tank> = std::iter::once(self.player.clone())
            .chain(self.enemies.iter().cloned())
            .collect();

        if let Some(new_enemy) = self.spawner.update(
            dt,
            alive_enemies,
            &tanks_for_spawn,
            self.tile_size,
            self.tank_size,
            self.player.speed * 0.75,
        ) {
            self.enemies.push(new_enemy);
        }

        let tanks_now: Vec<Tank> = std::iter::once(self.player.clone())
            .chain(self.enemies.iter().cloned())
            .collect();

        let mut hit_ids: Vec<u32> = Vec::new();
        for b in self.bullets.iter_mut() {
            if let Some(hit_id) = b.update(dt, &mut self.map, &tanks_now, self.tile_size) {
                hit_ids.push(hit_id);
            }
        }

        Bullet::resolve_mutual_destruction(&mut self.bullets);
        self.bullets.retain(|b| b.alive);

        let mut pending_bonuses: u32 = 0;
        for id in hit_ids {
            if id == self.player.id {
                // Проверка alive: если две пули попали в одном кадре,
                // вторая не должна отнимать ещё одну жизнь.
                if self.player.alive && !self.player.invulnerable {
                    self.player.take_hit();
                    self.player_lives = self.player_lives.saturating_sub(1);
                    if self.player_lives == 0 {
                        self.state = GameState::Defeat;
                    } else {
                        self.respawn_timer = 1.0;
                    }
                }
            } else if let Some(e) = self.enemies.iter_mut().find(|e| e.id == id) {
                let was_bonus = e.is_bonus;
                e.take_hit();
                if !e.alive && was_bonus {
                    pending_bonuses += 1;
                }
            }
        }

        if pending_bonuses > 0 {
            let mut forbidden: Vec<IVec2> = self.spawner.spawn_points().to_vec();
            forbidden.push(self.player_spawn_tile);

            for _ in 0..pending_bonuses {
                if let Some(tile) = self.map.random_bonus_spawn(&forbidden) {
                    let pos = vec2(
                        tile.x as f32 * self.tile_size,
                        tile.y as f32 * self.tile_size,
                    );
                    self.bonuses.push(Bonus::new(pos, self.tile_size, BONUS_LIFETIME));
                    forbidden.push(tile);
                }
            }
        }

        self.enemies.retain(|e| e.alive);

        for b in self.bonuses.iter_mut() {
            b.update(dt);
        }

        if self.player.alive {
            for b in self.bonuses.iter_mut() {
                if b.try_pickup(self.player.pos, self.player.size) {
                    self.picked_bonuses += 1;
                    apply_bonus(
                        b.kind,
                        &mut self.player,
                        &mut self.player_lives,
                        &mut self.enemies,
                        &mut self.map,
                        &mut self.freeze_timer,
                        &mut self.shovel_timer,
                        &mut self.shovel_tiles,
                        MAX_RANK,
                    );
                }
            }
        }

        self.bonuses.retain(|b| b.alive);

        if self.map.base_destroyed {
            self.state = GameState::Defeat;
        } else if self.spawner.remaining_to_spawn == 0 && self.enemies.is_empty() {
            self.state = GameState::Victory;
        }
    }

    /// Переход на следующий уровень: жизни и ранг сохраняются.
    pub fn advance_level(&mut self, cfg: &Config) {
        let next_level = self.current_level + 1;
        let lives = self.player_lives;
        let rank = self.player.rank;
        *self = Self::new_with_progress(cfg, next_level, lives, rank);
    }

    pub fn draw(&self) {
        clear_background(BLACK);

        let playfield_w = self.map.width as f32 * self.tile_size;
        let playfield_h = self.map.height as f32 * self.tile_size;

        let cam = Camera2D {
            target: vec2(playfield_w / 2.0, playfield_h / 2.0),
            zoom: vec2(2.0 / playfield_w, 2.0 / playfield_h),
            ..Default::default()
        };
        set_camera(&cam);

        self.map.draw(self.tile_size);
        for b in self.bonuses.iter() { b.draw(); }
        for b in self.bullets.iter() { b.draw(); }
        for e in self.enemies.iter() { e.draw(); }
        self.player.draw();

        match self.state {
            GameState::Victory => {
                let text = "VICTORY";
                let fs = 64;
                let dims = measure_text(text, None, fs, 1.0);
                draw_text(
                    text,
                    playfield_w / 2.0 - dims.width / 2.0,
                    playfield_h / 2.0 - 20.0,
                    fs as f32,
                    GREEN,
                );
                let hint = "Press ENTER for next level";
                let hfs = 24;
                let hdims = measure_text(hint, None, hfs, 1.0);
                draw_text(
                    hint,
                    playfield_w / 2.0 - hdims.width / 2.0,
                    playfield_h / 2.0 + 30.0,
                    hfs as f32,
                    LIGHTGRAY,
                );
            }
            GameState::Defeat => {
                let text = "GAME OVER";
                let fs = 64;
                let dims = measure_text(text, None, fs, 1.0);
                draw_text(
                    text,
                    playfield_w / 2.0 - dims.width / 2.0,
                    playfield_h / 2.0 - 20.0,
                    fs as f32,
                    RED,
                );
                let hint = "Press ENTER for new game";
                let hfs = 24;
                let hdims = measure_text(hint, None, hfs, 1.0);
                draw_text(
                    hint,
                    playfield_w / 2.0 - hdims.width / 2.0,
                    playfield_h / 2.0 + 30.0,
                    hfs as f32,
                    LIGHTGRAY,
                );
            }
            GameState::Playing => {}
        }

        set_default_camera();

        draw_text(
            &format!("LEVEL: {}", self.current_level),
            10.0,
            24.0,
            24.0,
            WHITE,
        );
        draw_text(
            &format!("LIVES: {}", self.player_lives),
            10.0,
            48.0,
            24.0,
            WHITE,
        );
        draw_text(
            &format!(
                "ENEMIES: {} + {} in reserve",
                self.enemies.iter().filter(|e| e.alive).count(),
                self.spawner.remaining_to_spawn
            ),
            10.0,
            72.0,
            20.0,
            LIGHTGRAY,
        );
        draw_text(
            &format!("BONUSES PICKED: {}", self.picked_bonuses),
            10.0,
            96.0,
            20.0,
            LIGHTGRAY,
        );
        draw_text(
            &format!("RANK: {}", self.player.rank),
            10.0,
            120.0,
            20.0,
            LIGHTGRAY,
        );

        if self.freeze_timer > 0.0 {
            draw_text(
                &format!("FREEZE: {:.1}s", self.freeze_timer),
                10.0,
                144.0,
                20.0,
                SKYBLUE,
            );
        }
        if self.shovel_timer > 0.0 {
            draw_text(
                &format!("SHOVEL: {:.1}s", self.shovel_timer),
                10.0,
                168.0,
                20.0,
                ORANGE,
            );
        }
    }
}