mod config;
mod input;
mod map;
mod entities;
mod game;

use macroquad::prelude::*;
use config::load_config;
use game::state::{Game, GameState};
use input::{any_down, any_pressed, warn_conflicts, KeyBindings, SystemBindings};

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
    let sys: SystemBindings = (&cfg.system_keys).into();
    let p1: KeyBindings = (&cfg.player1_keys).into();
    warn_conflicts(&p1, &sys);

    let playfield_w = cfg.tile_size * cfg.grid_width as f32;
    let playfield_h = cfg.tile_size * cfg.grid_height as f32;

    let scale = if cfg.window_scale > 0.0 { cfg.window_scale } else { 1.0 };
    request_new_screen_size(playfield_w * scale, playfield_h * scale);

    let mut game = Game::new(&cfg, p1.clone());
    // Сколько секунд удерживается клавиша выхода.
    let mut quit_held: f32 = 0.0;

    loop {
        let dt = get_frame_time();

        // Выход защищён от случайного нажатия (ребёнок!):
        // короткое нажатие — пауза, удержание quit_hold_seconds — выход.
        if any_pressed(&sys.quit) {
            if sys.quit_hold_seconds <= 0.0 {
                break;
            }
            game.pause();
        }
        if any_down(&sys.quit) {
            quit_held += dt;
            if sys.quit_hold_seconds > 0.0 && quit_held >= sys.quit_hold_seconds {
                break;
            }
        } else {
            quit_held = 0.0;
        }

        // Start (по умолчанию Enter или P):
        // - во время игры — пауза / продолжить (как Start на Dandy)
        // - Victory → следующий уровень (жизни и ранг сохраняются)
        // - Defeat → новая игра с 1-го уровня
        if any_pressed(&sys.start) {
            match game.state {
                GameState::Victory => game.advance_level(&cfg),
                GameState::Defeat => game = Game::new(&cfg, p1.clone()),
                GameState::Playing => game.toggle_pause(),
            }
        }

        game.update(dt);
        game.draw();
        draw_quit_hint(&game, &sys, quit_held);

        next_frame().await;
    }
}

/// Подсказка на паузе и полоска удержания клавиши выхода.
fn draw_quit_hint(game: &Game, sys: &SystemBindings, quit_held: f32) {
    if sys.quit_hold_seconds <= 0.0 {
        return;
    }
    let show = game.paused || quit_held > 0.0;
    if !show {
        return;
    }
    let quit_name = sys.quit.first().map(|k| format!("{k:?}")).unwrap_or_default();
    let text = format!("Hold {quit_name} to quit");
    let fs = 24.0;
    let dims = measure_text(&text, None, fs as u16, 1.0);
    let x = screen_width() / 2.0 - dims.width / 2.0;
    // Под надписью PAUSE, на тёмной подложке — чтобы не сливалось с картой.
    let y = screen_height() / 2.0 + 60.0;
    draw_rectangle(x - 10.0, y - dims.height - 8.0, dims.width + 20.0, dims.height + 34.0, Color::new(0.0, 0.0, 0.0, 0.8));
    draw_text(&text, x, y, fs, LIGHTGRAY);

    if quit_held > 0.0 {
        let w = 200.0;
        let progress = (quit_held / sys.quit_hold_seconds).min(1.0);
        let bx = screen_width() / 2.0 - w / 2.0;
        let by = y + 10.0;
        draw_rectangle_lines(bx, by, w, 10.0, 2.0, LIGHTGRAY);
        draw_rectangle(bx, by, w * progress, 10.0, RED);
    }
}
