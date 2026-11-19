mod config;
mod input;
mod map;
mod entities;
mod game;

use macroquad::prelude::*;
use config::load_config;
use input::KeyBindings;
use map::GameMap;
use entities::tank::{Tank, Direction};
use entities::bullet::Bullet;
use entities::spawner::EnemySpawner;
use entities::bonus::Bonus;
use game::bonuses::apply_bonus;

#[derive(Clone, Copy, PartialEq, Debug)]
enum GameState {
    Playing,
    Victory,
    Defeat,
}

const BONUS_LIFETIME: f32 = 15.0;
const MAX_RANK: u8 = 3;

#[macroquad::main("Battle City (Rust)")]
async fn main() {
    let cfg = load_config();

    let playfield_w = cfg.tile_size * cfg.grid_width as f32;
    let playfield_h = cfg.tile_size * cfg.grid_height as f32;
    let tank_size = cfg.tile_size * 2.0;

    let keys_p1: KeyBindings = (&cfg.player1_keys).into();
    let mut map = GameMap::new(cfg.grid_width, cfg.grid_height);

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
    player.apply_rank(0);

    let mut player_lives: u32 = 3;
    let mut respawn_timer: f32 = 0.0;
    let mut invuln_timer: f32 = 0.0;

    let mut enemies: Vec<Tank> = Vec::new();
    let mut spawner = EnemySpawner::new(100, 3, cfg.grid_width);

    let mut bullets: Vec<Bullet> = Vec::new();
    let mut bonuses: Vec<Bonus> = Vec::new();

    let mut picked_bonuses: u32 = 0;

    let mut state = GameState::Playing;

    request_new_screen_size(playfield_w, playfield_h);

    loop {
        let dt = get_frame_time();
        if is_key_pressed(KeyCode::Escape) { break; }

        if state == GameState::Playing {
            if invuln_timer > 0.0 {
                invuln_timer -= dt;
                if invuln_timer <= 0.0 {
                    player.invulnerable = false;
                }
            }

            if !player.alive {
                if respawn_timer > 0.0 {
                    respawn_timer -= dt;
                    if respawn_timer <= 0.0 {
                        player.pos = player_spawn;
                        player.direction = Direction::Up;
                        player.alive = true;
                        player.invulnerable = true;
                        invuln_timer = 3.0;
                    }
                }
            }

            let snapshot: Vec<Tank> = std::iter::once(player.clone())
                .chain(enemies.iter().cloned())
                .collect();

            player.update(dt, &keys_p1, &map, &snapshot, cfg.tile_size);

            if is_key_pressed(keys_p1.fire) {
                let active = bullets
                    .iter()
                    .filter(|b| b.alive && b.owner_id == player.id)
                    .count() as u8;

                if let Some(bullet) = player.try_fire(
                    active,
                    &map,
                    cfg.tile_size,
                    cfg.bullet_speed,
                    cfg.bullet_size,
                ) {
                    bullets.push(bullet);
                }
            }

            for enemy in enemies.iter_mut() {
                if !enemy.alive { continue; }

                let active = bullets
                    .iter()
                    .filter(|b| b.alive && b.owner_id == enemy.id)
                    .count() as u8;

                if let Some(bullet) = enemy.update_ai(
                    dt,
                    &map,
                    &snapshot,
                    cfg.tile_size,
                    cfg.bullet_speed * 0.75,
                    cfg.bullet_size,
                    active,
                ) {
                    bullets.push(bullet);
                }
            }

            let alive_enemies = enemies.iter().filter(|e| e.alive).count();
            let tanks_for_spawn: Vec<Tank> = std::iter::once(player.clone())
                .chain(enemies.iter().cloned())
                .collect();

            if let Some(new_enemy) = spawner.update(
                dt,
                alive_enemies,
                &tanks_for_spawn,
                cfg.tile_size,
                tank_size,
                cfg.tank_speed * 0.75,
            ) {
                enemies.push(new_enemy);
            }

            let tanks_now: Vec<Tank> = std::iter::once(player.clone())
                .chain(enemies.iter().cloned())
                .collect();

            let mut hit_ids: Vec<u32> = Vec::new();
            for b in bullets.iter_mut() {
                if let Some(hit_id) = b.update(dt, &mut map, &tanks_now, cfg.tile_size) {
                    hit_ids.push(hit_id);
                }
            }

            Bullet::resolve_mutual_destruction(&mut bullets);

            bullets.retain(|b| b.alive);

            let mut pending_bonuses: u32 = 0;
            for id in hit_ids {
                if id == player.id {
                    if !player.invulnerable {
                        player.take_hit();
                        player_lives = player_lives.saturating_sub(1);
                        if player_lives == 0 {
                            state = GameState::Defeat;
                        } else {
                            respawn_timer = 1.0;
                        }
                    }
                } else if let Some(e) = enemies.iter_mut().find(|e| e.id == id) {
                    if e.is_bonus {
                        pending_bonuses += 1;
                    }
                    e.take_hit();
                }
            }

            if pending_bonuses > 0 {
                let mut forbidden: Vec<IVec2> = spawner.spawn_points().to_vec();
                forbidden.push(player_spawn_tile);

                for _ in 0..pending_bonuses {
                    if let Some(tile) = map.random_bonus_spawn(&forbidden) {
                        let pos = vec2(
                            tile.x as f32 * cfg.tile_size,
                            tile.y as f32 * cfg.tile_size,
                        );
                        bonuses.push(Bonus::new(pos, cfg.tile_size, BONUS_LIFETIME));
                        forbidden.push(tile);
                    }
                }
            }

            enemies.retain(|e| e.alive);

            for b in bonuses.iter_mut() {
                b.update(dt);
            }

            if player.alive {
                for b in bonuses.iter_mut() {
                    if b.try_pickup(player.pos, player.size) {
                        picked_bonuses += 1;
                        apply_bonus(
                            b.kind,
                            &mut player,
                            &mut player_lives,
                            &mut enemies,
                            MAX_RANK,
                        );
                    }
                }
            }

            bonuses.retain(|b| b.alive);

            if map.base_destroyed {
                state = GameState::Defeat;
            } else if spawner.remaining_to_spawn == 0 && enemies.is_empty() {
                state = GameState::Victory;
            }
        }

        clear_background(BLACK);

        let cam = Camera2D {
            target: vec2(playfield_w / 2.0, playfield_h / 2.0),
            zoom: vec2(2.0 / playfield_w, 2.0 / playfield_h),
            ..Default::default()
        };
        set_camera(&cam);

        map.draw(cfg.tile_size);
        for b in bonuses.iter() { b.draw(); }
        for b in bullets.iter() { b.draw(); }
        for enemy in enemies.iter() { enemy.draw(); }
        player.draw();

        match state {
            GameState::Victory => {
                let text = "VICTORY";
                let font_size = 64;
                let dims = measure_text(text, None, font_size, 1.0);
                draw_text(
                    text,
                    playfield_w / 2.0 - dims.width / 2.0,
                    playfield_h / 2.0,
                    font_size as f32,
                    GREEN,
                );
            }
            GameState::Defeat => {
                let text = "GAME OVER";
                let font_size = 64;
                let dims = measure_text(text, None, font_size, 1.0);
                draw_text(
                    text,
                    playfield_w / 2.0 - dims.width / 2.0,
                    playfield_h / 2.0,
                    font_size as f32,
                    RED,
                );
            }
            GameState::Playing => {}
        }

        set_default_camera();

        draw_text(
            &format!("LIVES: {}", player_lives),
            10.0,
            24.0,
            24.0,
            WHITE,
        );
        draw_text(
            &format!(
                "ENEMIES: {} + {} in reserve",
                enemies.iter().filter(|e| e.alive).count(),
                spawner.remaining_to_spawn
            ),
            10.0,
            48.0,
            20.0,
            LIGHTGRAY,
        );
        draw_text(
            &format!("BONUSES PICKED: {}", picked_bonuses),
            10.0,
            72.0,
            20.0,
            LIGHTGRAY,
        );
        draw_text(
            &format!("RANK: {}", player.rank),
            10.0,
            96.0,
            20.0,
            LIGHTGRAY,
        );

        next_frame().await;
    }
}