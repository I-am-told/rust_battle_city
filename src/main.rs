mod config;
mod input;
mod map;
mod entities;

use macroquad::prelude::*;
use config::load_config;
use input::KeyBindings;
use map::GameMap;
use entities::tank::{Tank, Direction};
use entities::bullet::Bullet;
use entities::spawner::EnemySpawner;

#[derive(Clone, Copy, PartialEq, Debug)]
enum GameState {
    Playing,
    Victory,
    Defeat,
}

#[macroquad::main("Battle City (Rust)")]
async fn main() {
    let cfg = load_config();

    let playfield_w = cfg.tile_size * cfg.grid_width as f32;
    let playfield_h = cfg.tile_size * cfg.grid_height as f32;
    let tank_size = cfg.tile_size * 2.0;

    let keys_p1: KeyBindings = (&cfg.player1_keys).into();
    let mut map = GameMap::new(cfg.grid_width, cfg.grid_height);

    // Спавн игрока 1: слева от базы, с зазором 1 тайл от кирпичей
    // (base.x - 1) и 1 тайл от нижней границы поля.
    // Танк занимает 2 тайла по X: x .. x+1. Кирпич на base.x - 1.
    // Чтобы был зазор в 1 тайл: x + 1 = base.x - 2 → x = base.x - 3.
    // НО нам нужен ЗАЗОР: x + 1 < base.x - 1 → x < base.x - 2 → x = base.x - 3.
    // Проверим: base.x = 5 → x = 2, танк 2,3. Кирпич 4. Зазор — тайл 3? Нет!
    // Танк занимает тайлы 2 и 3. Кирпич на 4. Между 3 и 4 — нет зазора.
    // Нужно x = base.x - 4 → танк 1,2. Кирпич 4. Зазор — тайл 3. ✅
    let player_spawn = {
        if let Some(base) = map.base_origin {
            let sx = ((base.x - 4).max(0)) as f32 * cfg.tile_size;
            let sy = ((base.y - 1).max(0)) as f32 * cfg.tile_size;
            vec2(sx, sy)
        } else {
            vec2(1.0 * cfg.tile_size, (cfg.grid_height - 3) as f32 * cfg.tile_size)
        }
    };

    let mut player = Tank::new(
        player_spawn,
        Direction::Up,
        tank_size,
        cfg.tank_speed,
    );
    player.id = 0;

    let mut player_lives: u32 = 3;
    let mut respawn_timer: f32 = 0.0;
    let mut invuln_timer: f32 = 0.0;

    let mut enemies: Vec<Tank> = Vec::new();
    let mut spawner = EnemySpawner::new(10, 3, cfg.grid_width);

    let mut bullets: Vec<Bullet> = Vec::new();

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
                    e.take_hit();
                }
            }

            enemies.retain(|e| e.alive);

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

        next_frame().await;
    }
}