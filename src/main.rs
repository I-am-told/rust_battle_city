mod config;
mod input;
mod map;
mod entities;
mod game;

use macroquad::prelude::*;
use config::load_config;
use game::state::{Game, GameState};

#[macroquad::main("Battle City (Rust)")]
async fn main() {
    // Без зерна ГСЧ macroquad стартует с одного и того же состояния,
    // и каждая игра (враги, бонусы) повторяется один в один.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    rand::srand(seed);

    let cfg = load_config();

    let playfield_w = cfg.tile_size * cfg.grid_width as f32;
    let playfield_h = cfg.tile_size * cfg.grid_height as f32;

    request_new_screen_size(playfield_w, playfield_h);

    let mut game = Game::new(&cfg);

    loop {
        let dt = get_frame_time();
        if is_key_pressed(KeyCode::Escape) { break; }

        // Enter после конца:
        // - Victory → следующий уровень (жизни и ранг сохраняются)
        // - Defeat → новая игра с 1-го уровня
        if is_key_pressed(KeyCode::Enter) {
            match game.state {
                GameState::Victory => {
                    game.advance_level(&cfg);
                }
                GameState::Defeat => {
                    game = Game::new(&cfg);
                }
                GameState::Playing => {}
            }
        }

        game.update(dt);
        game.draw();

        next_frame().await;
    }
}