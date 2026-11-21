use macroquad::prelude::*;
use crate::ui_text::{draw_text, measure_text};
use crate::config::Config;
use crate::input::KeyBindings;
use crate::map::GameMap;
use crate::entities::tank::{AiGoal, Tank, Direction, Team, TurnAssist};
use crate::entities::bullet::{Bullet, BulletHit, BulletSpec};
use crate::map::WallHit;
use crate::sound::Sfx;
use crate::entities::spawner::EnemySpawner;
use crate::config::{Difficulty, Gameplay};
use crate::entities::bonus::{Bonus, BonusKind};
use crate::entities::effects::{draw_spawn_star, Effect, EffectKind};
use crate::entities::monkey::{Cloud, Monkey};
use crate::game::level::LevelConfig;
use crate::game::hud::{self, draw_tank_icon};
use crate::entities::enemy_kind::EnemyKind;
use crate::entities::tank_render::enemy_color;
use crate::game::player::{HitResult, Player, PLAYER_ID_BASE};
use crate::levels::LevelSet;
use std::rc::Rc;

/// Максимальный ранг игрока (см. Tank::apply_rank).
pub const MAX_RANK: u8 = 7;

/// Что переносится между уровнями.
pub struct Progress {
    pub level: u32,
    /// Общий запас жизней.
    pub lives: u32,
    /// Ранг каждого игрока.
    pub ranks: Vec<u8>,
    /// Очки каждого игрока.
    pub scores: Vec<u32>,
    /// Рекорд (лучший результат за всё время).
    pub hiscore: u32,
    /// Сложность (переносится между уровнями).
    pub difficulty: Difficulty,
}

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
    /// Враги, которые появляются: над точкой горит звезда (сколько прошло).
    pub pending: Vec<(Tank, f32)>,
    pub spawner: EnemySpawner,

    pub bullets: Vec<Bullet>,
    pub bonuses: Vec<Bonus>,
    /// Взрывы.
    pub effects: Vec<Effect>,
    /// Обезьянка бонуса «мама» (пока действует).
    pub monkey: Option<Monkey>,
    /// Облачки обезьянки в полёте.
    pub clouds: Vec<Cloud>,
    /// Звуковые события за кадр (main забирает и проигрывает).
    pub sfx: Vec<Sfx>,
    /// Взрыв штаба уже показан.
    base_exploded: bool,

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
    /// Карты уровней (из папки levels).
    pub levels: Rc<LevelSet>,
    /// Рекорд (обновляется сразу, как только кто-то его побил).
    pub hiscore: u32,
    /// Настройки игрового процесса из config.ron.
    pub tuning: Gameplay,
    /// Сложность.
    pub difficulty: Difficulty,
}

impl Game {
    /// Новая игра с уровня `level` (пункт STAGE в меню); жизни, ранги
    /// и очки — как при старте с 1-го. Число игроков = длина `keys` (1 или 2).
    /// Клавиши разбираются один раз в main, чтобы предупреждения
    /// о кривых клавишах не повторялись каждый уровень.
    pub fn new_at(
        cfg: &Config,
        keys: Vec<KeyBindings>,
        levels: Rc<LevelSet>,
        hiscore: u32,
        difficulty: Difficulty,
        level: u32,
    ) -> Self {
        let n = keys.len();
        let progress = Progress {
            level: level.max(1),
            // Запас общий: при двух игроках на старте вдвое больше.
            lives: cfg.gameplay.start_lives_per_player * n as u32,
            ranks: vec![0; n],
            scores: vec![0; n],
            hiscore,
            difficulty,
        };
        Self::new_with_progress(cfg, keys, levels, progress)
    }

    /// Создаёт игру на уровне `progress.level` с сохранением прогресса:
    /// общий запас жизней, ранг и очки каждого игрока, рекорд.
    pub fn new_with_progress(
        cfg: &Config,
        keys: Vec<KeyBindings>,
        levels: Rc<LevelSet>,
        progress: Progress,
    ) -> Self {
        let Progress { level, lives, ranks, scores, hiscore, difficulty } = progress;
        let layout = levels.layout_for(level);
        let from_file = layout.is_some();
        let mut map = GameMap::with_layout(cfg.grid_width, cfg.grid_height, layout);
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
                tank.ice_slide_tiles = cfg.gameplay.ice_slide_tiles;
                tank.rank4_speed_mult = cfg.gameplay.rank4_speed_mult;
                tank.turn_slide_time = cfg.gameplay.turn_slide_time;
                tank.sand_speed_mult = cfg.gameplay.sand_speed_mult;
                tank.apply_rank(ranks.get(i).copied().unwrap_or(0));
                let mut p = Player::new(tank, k, tile, cfg.tile_size, &cfg.gameplay);
                p.score = scores.get(i).copied().unwrap_or(0);
                p
            })
            .collect();

        // В начале уровня игроки тоже появляются через звёздочку.
        let mut players = players;
        for p in players.iter_mut() {
            p.begin_spawn();
        }

        let level_cfg = LevelConfig::for_level(level);
        let spawner = EnemySpawner::new(level_cfg, cfg.grid_width);

        // Точки появления врагов и игроков должны быть свободны —
        // расчищаем, если на карте там что-то нарисовано.
        let mut spawn_areas: Vec<IVec2> = spawner.spawn_points().to_vec();
        spawn_areas.extend(players.iter().map(|p| p.spawn_tile));
        for t in spawn_areas {
            if map.clear_area(t.x, t.y, 2, 2) && from_file {
                eprintln!(
                    "Уровень {level}: точка появления (тайл {}, {}) была занята на карте — расчищена.",
                    t.x, t.y
                );
            }
        }

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
            pending: Vec::new(),
            spawner,
            bullets: Vec::new(),
            bonuses: Vec::new(),
            effects: Vec::new(),
            monkey: None,
            clouds: Vec::new(),
            sfx: vec![Sfx::LevelStart],
            base_exploded: false,
            freeze_timer: 0.0,
            shovel_timer: 0.0,
            shovel_tiles: Vec::new(),
            state: GameState::Playing,
            paused: false,
            victory_timer: None,
            picked_bonuses: 0,
            current_level: level,
            levels,
            hiscore,
            tuning: cfg.gameplay,
            difficulty,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.paused {
            return;
        }
        // Взрывы доигрываются и после GAME OVER / победы.
        self.effects.retain_mut(|e| e.update(dt));
        if self.state != GameState::Playing {
            return;
        }
        self.update_playing(dt);
    }

    fn explode(&mut self, center: Vec2, kind: EffectKind) {
        self.effects.push(Effect::new(center, kind, self.tank_size));
    }

    /// Включить/выключить паузу. Работает только во время игры.
    pub fn toggle_pause(&mut self) {
        if self.state == GameState::Playing {
            self.paused = !self.paused;
            if self.paused {
                self.sfx.push(Sfx::Pause);
            }
        }
    }

    /// Поставить на паузу (не снимает её). Работает только во время игры.
    pub fn pause(&mut self) {
        if self.state == GameState::Playing && !self.paused {
            self.paused = true;
            self.sfx.push(Sfx::Pause);
        }
    }

    /// Едет ли сейчас хоть один игрок (для гула мотора).
    pub fn player_moving(&self) -> bool {
        self.state == GameState::Playing
            && !self.paused
            && self.players.iter().any(|p| {
                p.tank.alive && (p.tank.pos - p.tank.prev_pos).length_squared() > 0.0001
            })
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

    /// Точки, где горит звёздочка появления, как танки-препятствия:
    /// пока звезда горит, в эту точку никто не въедет.
    /// (В пули не передаются — в звезду попасть нельзя.)
    fn star_obstacles(&self) -> Vec<Tank> {
        let mut v: Vec<Tank> = self.pending.iter().map(|(t, _)| t.clone()).collect();
        for p in self.players.iter().filter(|p| p.star.is_some()) {
            let mut t = p.tank.clone();
            t.pos = p.spawn;
            t.alive = true;
            v.push(t);
        }
        v
    }

    fn update_playing(&mut self, dt: f32) {
        // Защитные поля и возрождение.
        for i in 0..self.players.len() {
            self.players[i].tick_shield(dt);
            self.players[i].guard_cooldown = (self.players[i].guard_cooldown - dt).max(0.0);
            self.players[i].tick_star(dt);

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
                self.players[i].begin_spawn();
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
        // Догоревшие звёзды врагов — враг появляется.
        for (_, t) in self.pending.iter_mut() {
            *t += dt;
        }
        let (ready, waiting): (Vec<_>, Vec<_>) =
            self.pending.drain(..).partition(|(_, t)| *t >= self.tuning.spawn_star_time);
        self.pending = waiting;
        for (mut tank, _) in ready {
            tank.frozen = frozen_now;
            tank.sand_speed_mult = self.tuning.sand_speed_mult;
            self.enemies.push(tank);
        }

        let np = self.players.len();
        // Индексы: игроки, враги, затем звёзды появления (только препятствия).
        let mut snapshot = self.all_tanks();
        snapshot.extend(self.star_obstacles());

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
                    self.sfx.push(Sfx::Shot);
                }
            }
        }

        // Второй круг карт и дальше — враги «умнее».
        let smart = self.smart_ai();
        let base_center = self.map.base_origin
            .filter(|_| !self.map.base_destroyed)
            .map(|o| (o.as_vec2() + vec2(1.0, 1.0)) * self.tile_size);
        let player_centers: Vec<Vec2> = self.players.iter()
            .filter(|p| p.tank.alive)
            .map(|p| p.tank.pos + vec2(p.tank.size, p.tank.size) / 2.0)
            .collect();
        let bias = self.tuning.smart_ai_bias;
        let tile_size = self.tile_size;

        for (i, enemy) in self.enemies.iter_mut().enumerate() {
            // Цель умного врага: «охотники» (нечётный id) — за ближайшим игроком,
            // остальные — к штабу; но игрока в 5 тайлах атакует любой.
            let goal = if smart && bias > 0.0 {
                let me = enemy.pos + vec2(enemy.size, enemy.size) / 2.0;
                let nearest = player_centers.iter().copied()
                    .min_by(|a, b| (*a - me).length().total_cmp(&(*b - me).length()));
                let close = nearest.filter(|p| (*p - me).length() < 5.0 * tile_size);
                let target = close
                    .or(if enemy.id % 2 == 1 { nearest } else { None })
                    .or(base_center)
                    .or(nearest);
                target.map(|t| AiGoal { target: t, bias })
            } else {
                None
            };
            enemy.ai_goal = goal;
            // Замороженные часами и одурманенные облачком — стоят.
            if !enemy.alive || enemy.frozen || enemy.gassed { continue; }

            let active = self.bullets
                .iter()
                .filter(|b| b.alive && b.owner_id == enemy.id)
                .count() as u8;

            if let Some(bullet) = enemy.update_ai(
                dt,
                &self.map,
                &snapshot,
                self.tile_size,
                BulletSpec {
                    speed: self.bullet_speed * self.tuning.enemy_bullet_speed_mult,
                    size: self.bullet_size,
                },
                active,
            ) {
                self.bullets.push(bullet);
            }
            snapshot[np + i].pos = enemy.pos;
            snapshot[np + i].direction = enemy.direction;
        }

        // Враги «на поле» с учётом тех, что уже появляются под звездой.
        let alive_enemies = self.enemies.iter().filter(|e| e.alive).count() + self.pending.len();
        let mut tanks_for_spawn = self.all_tanks();
        tanks_for_spawn.extend(self.star_obstacles());

        if let Some(new_enemy) = self.spawner.update(
            dt,
            alive_enemies,
            &tanks_for_spawn,
            self.tile_size,
            self.tank_size,
            self.tank_speed * self.tuning.enemy_speed_mult,
        ) {
            // Сначала звёздочка, танк выедет через spawn_star_time.
            self.pending.push((new_enemy, 0.0));
        }

        // Бонус «мама»: обезьянка и её облачка.
        self.monkey_step(dt);

        // Ранг 6: телепорт-защитник — до того, как пули полетят.
        self.guardian_step();

        // Пули. Игроки в одной команде — пули проходят сквозь напарника.
        let tanks_now = self.all_tanks();
        // (в кого попали, чья пуля)
        // (в кого попали, чья пуля, куда летела)
        let mut hit_ids: Vec<(u32, u32, Direction)> = Vec::new();
        for b in self.bullets.iter_mut() {
            if let Some(hit_id) = b.update(dt, &mut self.map, &tanks_now, self.tile_size) {
                hit_ids.push((hit_id, b.owner_id, b.direction));
            }
        }

        Bullet::resolve_mutual_destruction(&mut self.bullets);
        // Маленький взрыв там, где пуля закончила путь (стена, край поля,
        // танк, встречная пуля).
        let pops: Vec<Vec2> = self
            .bullets
            .iter()
            .filter(|b| !b.alive)
            .map(|b| b.pos + vec2(b.size, b.size) / 2.0)
            .collect();
        for p in pops {
            self.explode(p, EffectKind::Small);
        }
        // Звук попаданий — только для пуль игроков (как в оригинале).
        for b in self.bullets.iter().filter(|b| !b.alive && b.owner_team == Team::Player) {
            match b.hit {
                Some(BulletHit::Wall(WallHit::Brick)) => self.sfx.push(Sfx::HitBrick),
                Some(BulletHit::Wall(WallHit::Steel)) | Some(BulletHit::Edge) => self.sfx.push(Sfx::HitSteel),
                _ => {}
            }
        }
        self.bullets.retain(|b| b.alive);

        if self.map.base_destroyed && !self.base_exploded {
            self.base_exploded = true;
            if let Some(b) = self.map.base_origin {
                let c = vec2(b.x as f32 + 1.0, b.y as f32 + 1.0) * self.tile_size;
                self.explode(c, EffectKind::Big);
            }
            self.sfx.push(Sfx::BaseBoom);
        }

        let mut pending_bonuses: u32 = 0;
        let son_rules = self.son_rules();
        for (id, owner, bullet_dir) in hit_ids {
            if let Some(p) = self.players.iter_mut().find(|p| p.tank.id == id) {
                // Лёгкий режим: сына (P2) не убить в спину — пуля летит
                // туда же, куда смотрит танк, значит попала сзади.
                if son_rules && p.tank.player_slot == 1 && bullet_dir == p.tank.direction {
                    continue;
                }
                // Player::hit сам проверяет alive/invulnerable: вторая пуля
                // в том же кадре ничего не делает.
                match p.hit() {
                    HitResult::Died => {
                        let c = p.tank.pos + vec2(p.tank.size, p.tank.size) / 2.0;
                        self.effects.push(Effect::new(c, EffectKind::Big, self.tank_size));
                        self.sfx.push(Sfx::PlayerBoom);
                    }
                    // Броня приняла удар — танк цел, но броню видно, что сорвало.
                    HitResult::ArmorLost => self.sfx.push(Sfx::HitArmor),
                    HitResult::Nothing => {}
                }
            } else if let Some(e) = self.enemies.iter_mut().find(|e| e.id == id) {
                // Как в оригинале: бонус выпадает при ПЕРВОМ попадании
                // в мигающий танк (у бронированного — не после 4-го),
                // после чего танк перестаёт мигать.
                let was_bonus = e.is_bonus;
                e.is_bonus = false;
                e.take_hit();
                if e.alive {
                    self.sfx.push(Sfx::HitArmor);
                }
                if was_bonus {
                    pending_bonuses += 1;
                }
                if !e.alive {
                    // Очки тому, чья пуля добила.
                    if let (Some(kind), Some(pi)) = (
                        e.kind,
                        self.players.iter().position(|p| p.tank.id == owner),
                    ) {
                        let c = e.pos + vec2(e.size, e.size) / 2.0;
                        self.players[pi].kills[kind.index()] += 1;
                        self.award(pi, kind.score(), c);
                    }
                }
            }
        }

        if pending_bonuses > 0 {
            let mut forbidden: Vec<IVec2> = self.spawner.spawn_points().to_vec();
            forbidden.extend(self.players.iter().map(|p| p.spawn_tile));
            // И не под живыми игроками — иначе бонус подберётся сам.
            let ts = self.tile_size;
            // (танк может стоять между тайлами — берём оба соседних).
            for p in self.players.iter().filter(|p| p.tank.alive) {
                let (fx, fy) = (p.tank.pos.x / ts, p.tank.pos.y / ts);
                for x in [fx.floor() as i32, fx.ceil() as i32] {
                    for y in [fy.floor() as i32, fy.ceil() as i32] {
                        forbidden.push(ivec2(x, y));
                    }
                }
            }

            for _ in 0..pending_bonuses {
                // Лёгкий режим: бонус появляется рядом с сыном (P2).
                let spot = match self.son_tile() {
                    Some(near) if son_rules => self
                        .map
                        .bonus_spawn_near(near, &forbidden)
                        .or_else(|| self.map.random_bonus_spawn(&forbidden)),
                    _ => self.map.random_bonus_spawn(&forbidden),
                };
                if let Some(tile) = spot {
                    let pos = vec2(
                        tile.x as f32 * self.tile_size,
                        tile.y as f32 * self.tile_size,
                    );
                    let mut bonus = Bonus::new(pos, self.tank_size, self.tuning.bonus_lifetime);
                    if son_rules {
                        bonus.kind = self.easy_bonus_kind();
                    }
                    if bonus.kind == BonusKind::Mama {
                        bonus.size = self.tank_size * 3.0;
                        bonus.pos = self.mama_pos(pos);
                    }
                    self.bonuses.push(bonus);
                    self.sfx.push(Sfx::BonusAppear);
                    forbidden.push(tile);
                }
            }
        }

        // Большой взрыв на месте каждого уничтоженного врага
        // (пулей или гранатой — граната убивает, а убираем здесь).
        let booms: Vec<Vec2> = self
            .enemies
            .iter()
            .filter(|e| !e.alive)
            .map(|e| e.pos + vec2(e.size, e.size) / 2.0)
            .collect();
        for c in booms {
            self.explode(c, EffectKind::Big);
            self.sfx.push(Sfx::EnemyBoom);
        }
        self.enemies.retain(|e| e.alive);

        for b in self.bonuses.iter_mut() {
            b.update(dt);
        }

        // Бонус берёт любой живой игрок; звезда — тому, кто взял,
        // «+» — в общий запас, остальное действует на всех.
        let mut picked: Vec<(usize, BonusKind, Vec2)> = Vec::new();
        for b in self.bonuses.iter_mut() {
            for (pi, p) in self.players.iter().enumerate() {
                if p.tank.alive && b.try_pickup(p.tank.pos, p.tank.size) {
                    picked.push((pi, b.kind, b.pos + vec2(b.size, b.size) / 2.0));
                }
            }
        }
        self.bonuses.retain(|b| b.alive);
        for (pi, kind, at) in picked {
            self.picked_bonuses += 1;
            self.sfx.push(if kind == BonusKind::Life { Sfx::LifeUp } else { Sfx::BonusPickup });
            self.apply_bonus(pi, kind);
            self.award(pi, self.tuning.bonus_points, at);
        }

        // Поражение: штаб уничтожен, или все игроки погибли и жизней нет.
        // Игрок под звёздочкой — тоже «жив»: жизнь на него уже потрачена.
        let anyone_alive = self.players.iter().any(|p| p.in_game());
        if self.map.base_destroyed || (!anyone_alive && self.lives == 0) {
            self.state = GameState::Defeat;
            self.sfx.push(Sfx::GameOver);
        } else if self.spawner.remaining_to_spawn == 0 && self.enemies.is_empty() && self.pending.is_empty() {
            // Враги кончились — игра идёт ещё victory_delay секунд, потом победа.
            // Поражение в это время (штаб, последняя жизнь) всё ещё возможно —
            // оно проверяется выше и раньше.
            let t = self.victory_timer.get_or_insert(self.tuning.victory_delay);
            *t -= dt;
            if *t <= 0.0 {
                self.state = GameState::Victory;
            }
        }
    }

    /// Правила лёгкого режима — только когда играют вдвоём: всё помогает
    /// второму игроку (сыну). В игре на одного лёгкий режим = обычный.
    pub fn son_rules(&self) -> bool {
        self.difficulty == Difficulty::Easy && self.players.len() >= 2
    }

    /// Центр того, кого оберегает обезьянка: сын (P2), в игре на одного — P1.
    /// Пока игрок ждёт возрождения — его точка появления.
    pub(crate) fn guarded_center(&self) -> Option<Vec2> {
        let p = self.players.get(1).or(self.players.first())?;
        let half = vec2(self.tank_size, self.tank_size) / 2.0;
        Some(if p.tank.alive { p.tank.pos } else { p.spawn } + half)
    }

    /// Враги «умнее» со второго круга карт: уровень больше числа карт
    /// (без папки levels — после 15-го уровня).
    pub fn smart_ai(&self) -> bool {
        let maps = match self.levels.maps.len() { 0 => 15, n => n };
        self.current_level as usize > maps
    }

    /// Тайл, где сейчас сын (P2), или его точка появления.
    fn son_tile(&self) -> Option<IVec2> {
        let p = self.players.get(1)?;
        let pos = if p.tank.alive { p.tank.pos } else { p.spawn };
        Some(ivec2(
            (pos.x / self.tile_size).round() as i32,
            (pos.y / self.tile_size).round() as i32,
        ))
    }

    /// Лёгкий режим: какой бонус выпадет. Помогаем по обстановке:
    /// - стена штаба < 30% — только лопата;
    /// - жизней ≤ 1 — только жизнь;
    /// - иначе случайно, но с перевесом: стена ≤ 50% — сильно больше
    ///   лопат, ≤ 70% — больше; жизней < 3 — больше жизней; ранг P1 не
    ///   максимальный — больше звёзд, максимальный — звёзд нет совсем.
    ///   Порядок важности при всём сразу: штаб, потом жизни, потом звёзды.
    fn easy_bonus_kind(&self) -> BonusKind {
        let wall = self.map.base_wall_integrity();
        if wall < 0.3 {
            return BonusKind::Shovel;
        }
        if self.lives <= 1 {
            return BonusKind::Life;
        }
        let p1_max = self.players.first().is_some_and(|p| p.tank.rank >= MAX_RANK);
        let mut w: Vec<(BonusKind, u32)> = vec![
            (BonusKind::Star, if p1_max { 0 } else { 1 + 3 }),
            (BonusKind::Life, if self.lives < 3 { 1 + 4 } else { 1 }),
            (BonusKind::Grenade, 1),
            (BonusKind::Timer, 1),
            (BonusKind::Shovel, if wall <= 0.5 { 1 + 8 } else if wall <= 0.7 { 1 + 2 } else { 1 }),
            (BonusKind::Helmet, 1),
            // «Мама»: вместо звезды на максимальном ранге P1; чаще, когда
            // врагов на поле много или жизней мало.
            (BonusKind::Mama, {
                let many = self.enemies.iter().filter(|e| e.alive).count() >= 3;
                (if p1_max { 4 } else { 0 }) + if many { 2 } else { 0 } + if self.lives < 3 { 2 } else { 0 }
            }),
        ];
        w.retain(|(_, x)| *x > 0);
        let total: u32 = w.iter().map(|(_, x)| x).sum();
        let mut r = rand::gen_range(0, total);
        for (k, x) in w {
            if r < x {
                return k;
            }
            r -= x;
        }
        BonusKind::Helmet
    }

    /// Где положить огромную «маму» (3×3 танка). Рядом с сыном, но НЕ на
    /// нём — иначе он подберёт её в тот же миг и не успеет прочитать:
    /// край таблички в одном тайле от его танка, со стороны центра поля
    /// (если там упираемся в край — пробуем другие стороны).
    /// `fallback` — место обычного бонуса (если сына нет).
    fn mama_pos(&self, fallback: Vec2) -> Vec2 {
        let big = self.tank_size * 3.0;
        let field = vec2(self.map.width as f32, self.map.height as f32) * self.tile_size;
        let clamp = |p: Vec2| vec2(p.x.clamp(0.0, field.x - big), p.y.clamp(0.0, field.y - big));
        let Some(son) = self.players.get(1) else {
            let c = fallback + vec2(self.tank_size, self.tank_size) / 2.0;
            return clamp(c - vec2(big, big) / 2.0);
        };
        let s0 = if son.tank.alive { son.tank.pos } else { son.spawn };
        let sc = s0 + vec2(self.tank_size, self.tank_size) / 2.0;
        let gap = big / 2.0 + self.tank_size / 2.0 + self.tile_size;
        let to_center = field / 2.0 - sc;
        let mut dirs = vec![
            vec2(to_center.x.signum(), 0.0),
            vec2(0.0, to_center.y.signum()),
            vec2(-to_center.x.signum(), 0.0),
            vec2(0.0, -to_center.y.signum()),
        ];
        if to_center.y.abs() > to_center.x.abs() {
            dirs.swap(0, 1);
        }
        let overlaps_son = |p: Vec2| {
            p.x < s0.x + self.tank_size && p.x + big > s0.x && p.y < s0.y + self.tank_size && p.y + big > s0.y
        };
        for d in dirs {
            let p = clamp(sc + d * gap - vec2(big, big) / 2.0);
            if !overlaps_son(p) {
                return p;
            }
        }
        clamp(sc - vec2(big, big) / 2.0)
    }

    /// Enter, пока идёт пауза после последнего врага: не ждать —
    /// сразу экран победы. Возвращает true, если так и сделали.
    pub fn skip_victory_wait(&mut self) -> bool {
        if self.state == GameState::Playing && !self.paused && self.victory_timer.is_some() {
            self.victory_timer = Some(0.0);
            return true;
        }
        false
    }

    /// Обезьянка: пукает облачками в ближайшего ещё не одурманенного
    /// врага; облачко летит сквозь стены и навсегда усыпляет первого,
    /// кого заденет.
    fn monkey_step(&mut self, dt: f32) {
        let guarded = self.guarded_center();
        if let Some(m) = &mut self.monkey {
            m.tick(dt);
            if let Some(c) = guarded {
                let r = self.tuning.monkey_orbit_radius * self.tank_size;
                m.orbit_around(c, r, self.tuning.monkey_orbit_speed, dt);
            }
            if m.cooldown <= 0.0 {
                let target = self
                    .enemies
                    .iter()
                    .filter(|e| e.alive && !e.gassed)
                    .map(|e| (e.id, e.pos + vec2(e.size, e.size) / 2.0))
                    .min_by(|a, b| {
                        (a.1 - m.pos).length().total_cmp(&(b.1 - m.pos).length())
                    });
                if let Some((id, t)) = target {
                    let speed = self.tuning.cloud_speed * self.tile_size;
                    let cloud = m.shoot(t, id, speed, self.tuning.monkey_fire_interval);
                    self.clouds.push(cloud);
                    self.sfx.push(Sfx::Fart);
                }
            }
            if !m.alive() {
                self.monkey = None;
            }
        }

        let field = vec2(self.map.width as f32, self.map.height as f32) * self.tile_size;
        for c in self.clouds.iter_mut() {
            // Наведение: за своей целью; если её уже нет или она в дымке —
            // на ближайшего свободного врага.
            let free = |e: &&Tank| e.alive && !e.gassed;
            let aim = c
                .target
                .and_then(|id| self.enemies.iter().filter(free).find(|e| e.id == id))
                .or_else(|| {
                    self.enemies.iter().filter(free).min_by(|a, b| {
                        (a.pos - c.pos).length().total_cmp(&(b.pos - c.pos).length())
                    })
                })
                .map(|e| (e.id, e.pos + vec2(e.size, e.size) / 2.0));
            if let Some((id, to)) = aim {
                c.target = Some(id);
                c.steer(to);
            }
            c.update(dt);
            if c.pos.x < -c.radius || c.pos.y < -c.radius || c.pos.x > field.x + c.radius || c.pos.y > field.y + c.radius {
                c.alive = false;
            }
            if !c.alive {
                continue;
            }
            if let Some(e) = self.enemies.iter_mut().find(|e| e.alive && !e.gassed && c.touches(e.pos, e.size)) {
                e.gassed = true;
                c.alive = false;
                let at = e.pos + vec2(e.size, e.size) / 2.0;
                self.effects.push(Effect::new(at, EffectKind::Gas, self.tank_size));
            }
        }
        self.clouds.retain(|c| c.alive);
    }

    /// Ранг 6 — телепорт-защитник. Если вражеская пуля вот-вот попадёт
    /// в напарника, танк 6-го ранга мгновенно встаёт вплотную к напарнику
    /// со стороны пули, стволом к врагу, и принимает удар на себя
    /// (на броню, если она есть). Потом — перезарядка guardian_cooldown.
    fn guardian_step(&mut self) {
        if self.players.len() < 2 {
            return;
        }
        let size = self.tank_size;
        let field = vec2(self.map.width as f32, self.map.height as f32) * self.tile_size;
        let son_rules = self.son_rules();
        for gi in 0..self.players.len() {
            let qi = (gi + 1) % self.players.len();
            // Лёгкий режим: защищаем только сына — к нему телепортируется
            // отец (P1), сын к отцу — нет.
            if son_rules && !(gi == 0 && qi == 1) {
                continue;
            }
            let (g, q) = (&self.players[gi], &self.players[qi]);
            if !g.tank.alive || g.tank.rank < 6 || g.guard_cooldown > 0.0 {
                continue;
            }
            if !q.tank.alive || q.tank.invulnerable {
                continue;
            }
            // Вражеская пуля, которая на ближайших полутора танках пути
            // попадёт в напарника.
            let look = size * 1.5;
            let (q0, q1) = (q.tank.pos, q.tank.pos + vec2(size, size));
            let threat = self
                .bullets
                .iter()
                .filter(|b| b.alive && b.owner_team == Team::Enemy)
                .find_map(|b| {
                    let v = b.direction.vector();
                    let d = vec2(v.x as f32, v.y as f32);
                    let (a, e) = (b.pos, b.pos + d * look);
                    let (lo, hi) = (a.min(e), a.max(e) + vec2(b.size, b.size));
                    let hits = lo.x < q1.x && hi.x > q0.x && lo.y < q1.y && hi.y > q0.y;
                    hits.then_some(b.direction)
                });
            let Some(bullet_dir) = threat else { continue };
            let v = bullet_dir.vector();
            let spot = q.tank.pos - vec2(v.x as f32, v.y as f32) * size;

            // Место должно быть свободно: поле, стены/вода, другие танки.
            if spot.x < 0.0 || spot.y < 0.0 || spot.x + size > field.x || spot.y + size > field.y {
                continue;
            }
            if self.map.rect_blocked(spot, vec2(size, size), self.tile_size, false) {
                continue;
            }
            let g_id = g.tank.id;
            let overlaps = |t: &Tank| {
                t.alive && t.id != g_id
                    && spot.x < t.pos.x + t.size - 0.01 && spot.x + size > t.pos.x + 0.01
                    && spot.y < t.pos.y + t.size - 0.01 && spot.y + size > t.pos.y + 0.01
            };
            if self.enemies.iter().any(overlaps) || self.players.iter().any(|p| overlaps(&p.tank)) {
                continue;
            }

            let half = vec2(size, size) / 2.0;
            let from = self.players[gi].tank.pos + half;
            let g = &mut self.players[gi];
            g.tank.pos = spot;
            g.tank.prev_pos = spot;
            g.tank.slide_target = None;
            g.tank.ice_glide = 0.0;
            // Стволом к врагу — навстречу пуле.
            g.tank.direction = bullet_dir.opposite();
            g.guard_cooldown = self.tuning.guardian_cooldown;
            self.effects.push(Effect::new(from, EffectKind::Teleport, size));
            self.effects.push(Effect::new(spot + half, EffectKind::Teleport, size));
            self.sfx.push(Sfx::Teleport);
        }
    }

    /// Начислить очки игроку `pi`, показать их в точке `at`,
    /// выдать жизни за каждые 20 000 и обновить рекорд.
    fn award(&mut self, pi: usize, points: u32, at: Vec2) {
        let lives = self.players[pi].add_score(points);
        if lives > 0 {
            self.lives += lives;
            self.sfx.push(Sfx::LifeUp);
        }
        self.effects.push(Effect::new(at, EffectKind::Points(points), self.tank_size));
        self.hiscore = self.hiscore.max(self.players[pi].score);
    }

    /// Переход на следующий уровень: общий запас жизней, ранги и очки
    /// сохраняются. Все игроки выходят на новый уровень, даже если ждали
    /// возрождения.
    pub fn advance_level(&mut self, cfg: &Config) {
        let progress = Progress {
            level: self.current_level + 1,
            lives: self.lives,
            ranks: self.players.iter().map(|p| p.tank.rank).collect(),
            scores: self.players.iter().map(|p| p.score).collect(),
            hiscore: self.hiscore,
            difficulty: self.difficulty,
        };
        let keys: Vec<KeyBindings> = self.players.iter().map(|p| p.keys.clone()).collect();
        let levels = self.levels.clone();
        *self = Self::new_with_progress(cfg, keys, levels, progress);
    }

    /// Экран подсчёта после уровня (как на Dandy): по каждому типу врага —
    /// сколько уничтожено и сколько это очков, у каждого игрока свой столбец.
    fn draw_tally(&self, w: f32, h: f32) {
        let ts = self.tile_size;
        draw_rectangle(0.0, 0.0, w, h, BLACK);
        let center = |text: &str, x: f32, y: f32, size: f32, color: Color| {
            let d = measure_text(text, None, size as u16, 1.0);
            draw_text(text, x - d.width / 2.0, y, size, color);
        };
        center(&format!("HI-SCORE  {}", self.hiscore), w / 2.0, ts * 2.5, ts * 1.1, ORANGE);
        center(&format!("STAGE  {}", self.current_level), w / 2.0, ts * 4.5, ts * 1.2, WHITE);

        let n = self.players.len();
        // Столбцы игроков; значки врагов — посередине (на двоих)
        // или справа от столбца (на одного).
        let cols: Vec<f32> = if n == 1 { vec![w * 0.38] } else { vec![w * 0.22, w * 0.78] };
        let icon_x = if n == 1 { w * 0.68 } else { w * 0.5 };
        let icon = ts * 1.3;

        for (i, p) in self.players.iter().enumerate() {
            let x = cols[i];
            center(&format!("{} PLAYER", if i == 0 { "I" } else { "II" }), x, ts * 7.5, ts * 0.9, player_color(i));
            center(&p.score.to_string(), x, ts * 9.0, ts * 1.0, ORANGE);
        }

        let mut y = ts * 11.5;
        for kind in EnemyKind::all() {
            draw_tank_icon(icon_x - icon / 2.0, y - icon * 0.85, icon, enemy_color(kind));
            for (i, p) in self.players.iter().enumerate() {
                let count = p.kills[kind.index()];
                let line = format!("{:>2} x{:>3} = {:>4}", count, kind.score(), count * kind.score());
                center(&line, cols[i], y, ts * 0.8, WHITE);
            }
            y += ts * 2.2;
        }

        // Итого: всего врагов у каждого.
        draw_rectangle(w * 0.1, y - ts * 1.2, w * 0.8, ts * 0.1, WHITE);
        for (i, p) in self.players.iter().enumerate() {
            let total: u32 = p.kills.iter().sum();
            center(&format!("TOTAL {total}"), cols[i], y + ts * 0.2, ts * 0.9, WHITE);
        }

        center("Press ENTER for next level", w / 2.0, h - ts * 1.5, ts * 0.75, LIGHTGRAY);
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
        for b in self.bullets.iter() { b.draw(); }
        for e in self.enemies.iter() { e.draw(); }
        for p in self.players.iter() { p.tank.draw(); }
        // Лес поверх танков и пуль (как в оригинале), бонусы — поверх леса.
        self.map.draw_forest(self.tile_size);
        // Лёгкий режим: танк сына (P2) виден и в лесу.
        if self.son_rules() {
            for p in self.players.iter().filter(|p| p.tank.player_slot == 1) {
                p.tank.draw();
            }
        }
        for b in self.bonuses.iter() { b.draw(); }
        // Звёздочки появления.
        let half = vec2(self.tank_size, self.tank_size) / 2.0;
        for (t, el) in self.pending.iter() {
            draw_spawn_star(t.pos + half, self.tank_size, *el, self.tuning.spawn_star_time);
        }
        for p in self.players.iter() {
            if let Some(el) = p.star {
                draw_spawn_star(p.spawn + half, self.tank_size, el, self.tuning.spawn_star_time);
            }
        }
        for c in self.clouds.iter() { c.draw(); }
        if let Some(m) = &self.monkey { m.draw(); }
        for fx in self.effects.iter() { fx.draw(); }

        match self.state {
            GameState::Victory => self.draw_tally(playfield_w, playfield_h),
            GameState::Defeat => {
                // Тёмная плашка, чтобы текст читался поверх карты.
                let box_h = 175.0 + self.players.len() as f32 * 36.0;
                draw_rectangle(
                    playfield_w * 0.15,
                    playfield_h / 2.0 - 100.0,
                    playfield_w * 0.7,
                    box_h,
                    Color::new(0.0, 0.0, 0.0, 0.8),
                );
                let text = "GAME OVER";
                let fs = 64;
                let dims = measure_text(text, None, fs, 1.0);
                draw_text(
                    text,
                    playfield_w / 2.0 - dims.width / 2.0,
                    playfield_h / 2.0 - 40.0,
                    fs as f32,
                    RED,
                );
                // Очки игроков и рекорд.
                let mut y = playfield_h / 2.0 + 10.0;
                for (i, p) in self.players.iter().enumerate() {
                    let line = format!("P{}  {}", i + 1, p.score);
                    let d = measure_text(&line, None, 32, 1.0);
                    draw_text(&line, playfield_w / 2.0 - d.width / 2.0, y, 32.0, player_color(i));
                    y += 36.0;
                }
                let hs = format!("HI-SCORE  {}", self.hiscore);
                let d = measure_text(&hs, None, 28, 1.0);
                draw_text(&hs, playfield_w / 2.0 - d.width / 2.0, y + 6.0, 28.0, ORANGE);
                let hint = "Press ENTER for menu";
                let hfs = 24;
                let hdims = measure_text(hint, None, hfs, 1.0);
                draw_text(
                    hint,
                    playfield_w / 2.0 - hdims.width / 2.0,
                    y + 50.0,
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
                mama_timer: self.monkey.as_ref().map_or(0.0, |m| m.time_left),
            },
        );

        set_default_camera();
    }
}

/// Цвет игрока: P1 — жёлтый, P2 — зелёный.
fn player_color(i: usize) -> Color {
    if i == 0 { YELLOW } else { GREEN }
}
