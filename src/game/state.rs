use macroquad::prelude::*;
use crate::config::Config;
use crate::input::KeyBindings;
use crate::map::GameMap;
use crate::entities::tank::{Tank, Direction, TurnAssist};
use crate::entities::bullet::Bullet;
use crate::entities::spawner::EnemySpawner;
use crate::entities::bonus::Bonus;
use crate::game::bonuses::apply_bonus;
use crate::game::level::LevelConfig;
use crate::game::hud;
use crate::game::player::{Player, PLAYER_ID_BASE};

pub const BONUS_LIFETIME: f32 = 15.0;
pub const MAX_RANK: u8 = 3;
/// Сколько секунд игра продолжается после гибели последнего врага
/// (как в оригинале — можно успеть подобрать бонус).
pub const VICTORY_DELAY: f32 = 5.0;
/// Запасные жизни на старте — на каждого игрока (как в оригинале: «IP 2»).
/// Запас общий: при двух игроках на старте 4.
pub const START_LIVES_PER_PLAYER: u32 = 2;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GameState {
    Playing,
    Victory,
    Defeat,
}

pub struct Game {
    pub tile_size: f32,
    pub tank_size: f32,
    pub bullet_speed: f32,
    pub bullet_size: f32,
    pub tank_speed: f32,

    pub map: GameMap,
    /// Игроки (1 или 2). Индекс = номер игрока - 1.
    pub players: Vec<Player>,
    /// Общий запас жизней (не считая танков на поле).
    /// Гибель ничего не списывает — жизнь тратится в момент возрождения.
    pub lives: u32,

    pub enemies: Vec<Tank>,
    pub spawner: EnemySpawner,

    pub bullets: Vec<Bullet>,
    pub bonuses: Vec<Bonus>,

    pub freeze_timer: f32,
    pub shovel_timer: f32,
    pub shovel_tiles: Vec<IVec2>,

    pub state: GameState,
    /// Пауза (только в состоянии Playing).
    pub paused: bool,
    /// Обратный отсчёт до экрана победы; None — враги ещё есть.
    pub victory_timer: Option<f32>,
    pub picked_bonuses: u32,
    /// Текущий уровень (1-based). При победе увеличивается.
    pub current_level: u32,
}

impl Game {
    /// Новая игра с 1-го уровня. Число игроков = длина `keys` (1 или 2).
    /// Клавиши разбираются один раз в main, чтобы предупреждения
    /// о кривых клавишах не повторялись каждый уровень.
    pub fn new(cfg: &Config, keys: Vec<KeyBindings>) -> Self {
        let n = keys.len() as u32;
        let ranks = vec![0; keys.len()];
        Self::new_with_progress(cfg, keys, 1, START_LIVES_PER_PLAYER * n, ranks)
    }

    /// Создаёт игру на уровне `level` с сохранением прогресса:
    /// общий запас жизней и ранг каждого игрока.
    pub fn new_with_progress(
        cfg: &Config,
        keys: Vec<KeyBindings>,
        level: u32,
        lives: u32,
        ranks: Vec<u8>,
    ) -> Self {
        let map = GameMap::new(cfg.grid_width, cfg.grid_height);
        let tank_size = cfg.tile_size * 2.0;
        // Скорости и размеры в конфиге — в тайлах; внутри игры — пиксели.
        let tank_speed_px = cfg.tank_speed * cfg.tile_size;
        let bullet_speed_px = cfg.bullet_speed * cfg.tile_size;
        let bullet_size_px = cfg.bullet_size * cfg.tile_size;

        let turn_assist = TurnAssist::parse(&cfg.turn_assist).unwrap_or_else(|| {
            eprintln!(
                "Неизвестный turn_assist '{}' (варианты: off, jump, smooth) — беру smooth.",
                cfg.turn_assist
            );
            TurnAssist::Smooth
        });

        // Точки появления игроков: как в оригинале — слева (P1)
        // и справа (P2) от штаба, через 2 тайла от стены.
        let spawn_tiles: Vec<IVec2> = match map.base_origin {
            Some(base) => vec![
                ivec2((base.x - 4).max(0), (base.y - 1).max(0)),
                ivec2((base.x + 4).min(cfg.grid_width - 2), (base.y - 1).max(0)),
            ],
            None => vec![
                ivec2(1, cfg.grid_height - 3),
                ivec2(cfg.grid_width - 3, cfg.grid_height - 3),
            ],
        };

        let players: Vec<Player> = keys
            .into_iter()
            .enumerate()
            .map(|(i, k)| {
                let tile = spawn_tiles[i.min(spawn_tiles.len() - 1)];
                let pos = vec2(tile.x as f32 * cfg.tile_size, tile.y as f32 * cfg.tile_size);
                let mut tank = Tank::new(pos, Direction::Up, tank_size, tank_speed_px);
                tank.id = PLAYER_ID_BASE + i as u32;
                tank.player_slot = i as u8;
                tank.turn_assist = turn_assist;
                tank.apply_rank(ranks.get(i).copied().unwrap_or(0));
                Player::new(tank, k, tile, cfg.tile_size)
            })
            .collect();

        let level_cfg = LevelConfig::for_level(level);
        let spawner = EnemySpawner::new(level_cfg, cfg.grid_width);

        Self {
            tile_size: cfg.tile_size,
            tank_size,
            bullet_speed: bullet_speed_px,
            bullet_size: bullet_size_px,
            tank_speed: tank_speed_px,
            map,
            players,
            lives,
            enemies: Vec::new(),
            spawner,
            bullets: Vec::new(),
            bonuses: Vec::new(),
            freeze_timer: 0.0,
            shovel_timer: 0.0,
            shovel_tiles: Vec::new(),
            state: GameState::Playing,
            paused: false,
            victory_timer: None,
            picked_bonuses: 0,
            current_level: level,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.state != GameState::Playing || self.paused {
            return;
        }
        self.update_playing(dt);
    }

    /// Включить/выключить паузу. Работает только во время игры.
    pub fn toggle_pause(&mut self) {
        if self.state == GameState::Playing {
            self.paused = !self.paused;
        }
    }

    /// Поставить на паузу (не снимает её). Работает только во время игры.
    pub fn pause(&mut self) {
        if self.state == GameState::Playing {
            self.paused = true;
        }
    }

    /// Занята ли точка появления игрока `idx` живым танком (врагом или напарником).
    fn spawn_occupied(&self, idx: usize) -> bool {
        let sp = self.players[idx].spawn;
        let near = |t: &Tank| {
            t.alive && (t.pos.x - sp.x).abs() < self.tank_size && (t.pos.y - sp.y).abs() < self.tank_size
        };
        self.enemies.iter().any(near)
            || self.players.iter().enumerate().any(|(j, p)| j != idx && near(&p.tank))
    }

    /// Все танки: сначала игроки (индексы 0..players.len()), затем враги.
    fn all_tanks(&self) -> Vec<Tank> {
        self.players
            .iter()
            .map(|p| p.tank.clone())
            .chain(self.enemies.iter().cloned())
            .collect()
    }

    fn update_playing(&mut self, dt: f32) {
        // Защитные поля и возрождение.
        for i in 0..self.players.len() {
            self.players[i].tick_shield(dt);

            let Some(t) = self.players[i].respawn_timer else { continue };
            let t = t - dt;
            if t > 0.0 {
                self.players[i].respawn_timer = Some(t);
            } else if self.lives == 0 {
                // Жизней нет — ждём: вдруг напарник подберёт «+».
                self.players[i].respawn_timer = Some(0.0);
            } else if self.spawn_occupied(i) {
                self.players[i].respawn_timer = Some(0.2);
            } else {
                self.lives -= 1;
                self.players[i].respawn();
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
        let np = self.players.len();
        let mut snapshot = self.all_tanks();

        for (i, p) in self.players.iter_mut().enumerate() {
            let desired = p.dir_input.update(&p.keys);
            p.tank.update(dt, desired, &self.map, &snapshot, self.tile_size);
            snapshot[i].pos = p.tank.pos;
            snapshot[i].direction = p.tank.direction;

            // Мёртвый игрок (ожидание возрождения) не стреляет.
            if p.tank.alive && p.keys.fire_pressed() {
                let id = p.tank.id;
                let active = self.bullets
                    .iter()
                    .filter(|b| b.alive && b.owner_id == id)
                    .count() as u8;
                if let Some(bullet) = p.tank.try_fire(
                    active,
                    &self.map,
                    self.tile_size,
                    self.bullet_speed,
                    self.bullet_size,
                ) {
                    self.bullets.push(bullet);
                }
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
            snapshot[np + i].pos = enemy.pos;
            snapshot[np + i].direction = enemy.direction;
        }

        let alive_enemies = self.enemies.iter().filter(|e| e.alive).count();
        let tanks_for_spawn = self.all_tanks();

        if let Some(new_enemy) = self.spawner.update(
            dt,
            alive_enemies,
            &tanks_for_spawn,
            self.tile_size,
            self.tank_size,
            self.tank_speed * 0.75,
        ) {
            self.enemies.push(new_enemy);
        }

        // Пули. Игроки в одной команде — пули проходят сквозь напарника.
        let tanks_now = self.all_tanks();
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
            if let Some(p) = self.players.iter_mut().find(|p| p.tank.id == id) {
                // Player::hit сам проверяет alive/invulnerable: вторая пуля
                // в том же кадре ничего не делает.
                p.hit();
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
            forbidden.extend(self.players.iter().map(|p| p.spawn_tile));

            for _ in 0..pending_bonuses {
                if let Some(tile) = self.map.random_bonus_spawn(&forbidden) {
                    let pos = vec2(
                        tile.x as f32 * self.tile_size,
                        tile.y as f32 * self.tile_size,
                    );
                    self.bonuses.push(Bonus::new(pos, self.tank_size, BONUS_LIFETIME));
                    forbidden.push(tile);
                }
            }
        }

        self.enemies.retain(|e| e.alive);

        for b in self.bonuses.iter_mut() {
            b.update(dt);
        }

        // Бонус берёт любой живой игрок; звезда — тому, кто взял,
        // «+» — в общий запас, остальное действует на всех.
        for b in self.bonuses.iter_mut() {
            for p in self.players.iter_mut() {
                if !p.tank.alive {
                    continue;
                }
                if b.try_pickup(p.tank.pos, p.tank.size) {
                    self.picked_bonuses += 1;
                    apply_bonus(
                        b.kind,
                        &mut p.tank,
                        &mut self.lives,
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

        // Поражение: штаб уничтожен, или все игроки погибли и жизней нет.
        let anyone_alive = self.players.iter().any(|p| p.tank.alive);
        if self.map.base_destroyed || (!anyone_alive && self.lives == 0) {
            self.state = GameState::Defeat;
        } else if self.spawner.remaining_to_spawn == 0 && self.enemies.is_empty() {
            // Враги кончились — игра идёт ещё VICTORY_DELAY секунд, потом победа.
            // Поражение в это время (штаб, последняя жизнь) всё ещё возможно —
            // оно проверяется выше и раньше.
            let t = self.victory_timer.get_or_insert(VICTORY_DELAY);
            *t -= dt;
            if *t <= 0.0 {
                self.state = GameState::Victory;
            }
        }
    }

    /// Переход на следующий уровень: общий запас жизней и ранги сохраняются.
    /// Все игроки выходят на новый уровень, даже если ждали возрождения.
    pub fn advance_level(&mut self, cfg: &Config) {
        let next_level = self.current_level + 1;
        let ranks: Vec<u8> = self.players.iter().map(|p| p.tank.rank).collect();
        let keys: Vec<KeyBindings> = self.players.iter().map(|p| p.keys.clone()).collect();
        *self = Self::new_with_progress(cfg, keys, next_level, self.lives, ranks);
    }

    pub fn draw(&self) {
        clear_background(BLACK);

        let playfield_w = self.map.width as f32 * self.tile_size;
        let playfield_h = self.map.height as f32 * self.tile_size;

        // Камера с рамкой; пропорции сохраняются при любом размере окна.
        let cam = hud::game_camera(self.tile_size, self.map.width, self.map.height);
        set_camera(&cam);
        hud::draw_frame(self.tile_size, self.map.width, self.map.height);

        self.map.draw(self.tile_size);
        for b in self.bonuses.iter() { b.draw(); }
        for b in self.bullets.iter() { b.draw(); }
        for e in self.enemies.iter() { e.draw(); }
        for p in self.players.iter() { p.tank.draw(); }

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
                let hint = "Press ENTER for menu";
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
            GameState::Playing => {
                if self.paused {
                    // Мигающая надпись, как в оригинале.
                    if ((get_time() * 2.0) as i32) % 2 == 0 {
                        let text = "PAUSE";
                        let fs = 64;
                        let dims = measure_text(text, None, fs, 1.0);
                        draw_text(
                            text,
                            playfield_w / 2.0 - dims.width / 2.0,
                            playfield_h / 2.0,
                            fs as f32,
                            RED,
                        );
                    }
                }
            }
        }

        // Боковая панель (в той же камере).
        let colors: Vec<Color> = self
            .players
            .iter()
            .map(|p| if p.tank.player_slot == 0 { YELLOW } else { GREEN })
            .collect();
        hud::draw_panel(
            self.tile_size,
            self.map.width,
            self.map.height,
            &hud::PanelInfo {
                enemies_left: self.spawner.remaining_to_spawn,
                lives: self.lives,
                player_colors: &colors,
                level: self.current_level,
                freeze_timer: self.freeze_timer,
                shovel_timer: self.shovel_timer,
            },
        );

        set_default_camera();
    }
}