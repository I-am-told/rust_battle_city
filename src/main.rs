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

#[macroquad::main("Battle City (Rust)")]
async fn main() {
    let cfg = load_config();

    let playfield_w = cfg.tile_size * cfg.grid_width as f32;
    let playfield_h = cfg.tile_size * cfg.grid_height as f32;
    let tank_size = cfg.tile_size * 2.0;

    let keys_p1: KeyBindings = (&cfg.player1_keys).into();
    let mut map = GameMap::new(cfg.grid_width, cfg.grid_height);

    let start_x = 5.0 * cfg.tile_size;
    let start_y = 10.0 * cfg.tile_size;

    let mut player = Tank::new(
        vec2(start_x, start_y),
        Direction::Up,
        tank_size,
        cfg.tank_speed,
    );
    player.id = 0;

    let mut enemies: Vec<Tank> = Vec::new();
    // 10 врагов всего за уровень, 3 одновременно. Позже max_on_field будет
    // зависеть от уровня игры (см. комментарий в EnemySpawner).
    let mut spawner = EnemySpawner::new(10, 3, cfg.grid_width);

    let mut bullets: Vec<Bullet> = Vec::new();

    request_new_screen_size(playfield_w, playfield_h);

    loop {
        let dt = get_frame_time();
        if is_key_pressed(KeyCode::Escape) { break; }

        // ============================================================
        // 1) Снимок танков на начало кадра — для коллизий танк-танк.
        // ============================================================
        let snapshot: Vec<Tank> = std::iter::once(player.clone())
            .chain(enemies.iter().cloned())
            .collect();

        // --- Игрок ---
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

        // --- Враги ---
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

        // --- Спавн врагов ---
        // Передаём ВСЕ танки (включая игрока), чтобы спавн не наложился на игрока.
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

        // ============================================================
        // 2) Актуальный снимок танков (для проверки попаданий пуль).
        //    Танки уже сдвинулись в этом кадре.
        // ============================================================
        let tanks_now: Vec<Tank> = std::iter::once(player.clone())
            .chain(enemies.iter().cloned())
            .collect();

        // --- Пули ---
        let mut hit_ids: Vec<u32> = Vec::new();
        for b in bullets.iter_mut() {
            if let Some(hit_id) = b.update(dt, &mut map, &tanks_now, cfg.tile_size) {
                hit_ids.push(hit_id);
            }
        }
        bullets.retain(|b| b.alive);

        // Применяем попадания.
        for id in hit_ids {
            if id == player.id {
                player.take_hit();
            } else if let Some(e) = enemies.iter_mut().find(|e| e.id == id) {
                e.take_hit();
            }
        }

        enemies.retain(|e| e.alive);

        // --- Отрисовка ---
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

        set_default_camera();

        next_frame().await;
    }
}