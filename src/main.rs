mod config;
mod input;
mod map;
mod entities;

use macroquad::prelude::*;
use config::load_config;
use input::KeyBindings;
use map::GameMap;
use entities::tank::Tank;
use crate::entities::tank::Direction;

#[macroquad::main("Battle City (Rust)")]
async fn main() {
    let cfg = load_config();

    let playfield_w = cfg.tile_size * cfg.grid_width as f32;
    let playfield_h = cfg.tile_size * cfg.grid_height as f32;
    let tank_size = cfg.tile_size * 2.0;

    let keys_p1: KeyBindings = (&cfg.player1_keys).into();
    let map = GameMap::new(cfg.grid_width, cfg.grid_height);

    let start_x = 5.0 * cfg.tile_size;
    let start_y = 10.0 * cfg.tile_size;

    let mut player = Tank::new(
        vec2(start_x, start_y),
        Direction::Up,
        tank_size,
        cfg.tank_speed,
    );

    request_new_screen_size(playfield_w, playfield_h);

    loop {
        let dt = get_frame_time();
        if is_key_pressed(KeyCode::Escape) { break; }

        player.update(dt, &keys_p1, &map, cfg.tile_size);

        clear_background(BLACK);

        let cam = Camera2D {
            target: vec2(playfield_w / 2.0, playfield_h / 2.0),
            zoom: vec2(2.0 / playfield_w, 2.0 / playfield_h),
            ..Default::default()
        };
        set_camera(&cam);

        map.draw(cfg.tile_size);
        player.draw();

        set_default_camera();

        next_frame().await;
    }
}