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

    // Один враг сверху для теста.
    let mut enemies: Vec<Tank> = Vec::new();
    enemies.push(Tank::new_enemy(
        vec2(1.0 * cfg.tile_size, 1.0 * cfg.tile_size),
        Direction::Down,
        tank_size,
        cfg.tank_speed * 0.75,
        1,
    ));

    let mut bullets: Vec<Bullet> = Vec::new();

    request_new_screen_size(playfield_w, playfield_h);

    loop {
        let dt = get_frame_time();
        if is_key_pressed(KeyCode::Escape) { break; }

        // --- Игрок ---
        player.update(dt, &keys_p1, &map, cfg.tile_size);

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
            let active = bullets
                .iter()
                .filter(|b| b.alive && b.owner_id == enemy.id)
                .count() as u8;

            if let Some(bullet) = enemy.update_ai(
                dt,
                &map,
                cfg.tile_size,
                cfg.bullet_speed * 0.75,
                cfg.bullet_size,
                active,
            ) {
                bullets.push(bullet);
            }
        }

        // --- Пули ---
        for b in bullets.iter_mut() {
            b.update(dt, &mut map, cfg.tile_size);
        }
        bullets.retain(|b| b.alive);

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