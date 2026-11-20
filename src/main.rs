mod config;
mod input;
mod map;
mod entities;
mod game;
mod menu;

use macroquad::prelude::*;
use config::load_config;
use game::state::{Game, GameState};
use input::{any_down, any_pressed, warn_conflicts, KeyBindings, SystemBindings};
use config::PlayerKeys;
use menu::{Menu, MenuAction};

/// Текущий экран.
enum Screen {
    Menu(Menu),
    Game(Box<Game>),
}

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
    // Клавиши обоих игроков (в игре на одного используются только P1).
    let all_keys: Vec<KeyBindings> = vec![
        KeyBindings::from_config("P1", &cfg.player1_keys, &PlayerKeys::default()),
        KeyBindings::from_config("P2", &cfg.player2_keys, &PlayerKeys::player2_default()),
    ];
    warn_conflicts(&all_keys, &sys);

    let playfield_w = cfg.tile_size * cfg.grid_width as f32;
    let playfield_h = cfg.tile_size * cfg.grid_height as f32;

    let scale = if cfg.window_scale > 0.0 { cfg.window_scale } else { 1.0 };
    request_new_screen_size(playfield_w * scale, playfield_h * scale);

    let mut screen = Screen::Menu(Menu::new());
    // Последний выбор в меню (после GAME OVER курсор стоит на нём).
    let mut last_menu_choice: usize = 0;
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
            if let Screen::Game(game) = &mut screen {
                game.pause();
            }
        }
        if any_down(&sys.quit) {
            quit_held += dt;
            if sys.quit_hold_seconds > 0.0 && quit_held >= sys.quit_hold_seconds {
                break;
            }
        } else {
            quit_held = 0.0;
        }

        let mut next: Option<Screen> = None;
        match &mut screen {
            Screen::Menu(menu) => {
                if let Some(MenuAction::Start(n)) = menu.update(&all_keys, &sys) {
                    last_menu_choice = menu.selected;
                    let keys = all_keys[..n.min(all_keys.len())].to_vec();
                    next = Some(Screen::Game(Box::new(Game::new(&cfg, keys))));
                }
                menu.draw(&all_keys);
                draw_quit_hint(false, &sys, quit_held);
            }
            Screen::Game(game) => {
                // Start (по умолчанию Enter или P):
                // - во время игры — пауза / продолжить (как Start на Dandy)
                // - Victory → следующий уровень (жизни и ранги сохраняются)
                // - Defeat → в меню
                if any_pressed(&sys.start) {
                    match game.state {
                        GameState::Victory => game.advance_level(&cfg),
                        GameState::Defeat => {
                            next = Some(Screen::Menu(Menu { selected: last_menu_choice }));
                        }
                        GameState::Playing => game.toggle_pause(),
                    }
                }
                game.update(dt);
                game.draw();
                draw_quit_hint(game.paused, &sys, quit_held);
            }
        }
        if let Some(s) = next {
            screen = s;
        }

        next_frame().await;
    }
}

/// Подсказка на паузе и полоска удержания клавиши выхода.
fn draw_quit_hint(paused: bool, sys: &SystemBindings, quit_held: f32) {
    if sys.quit_hold_seconds <= 0.0 {
        return;
    }
    let show = paused || quit_held > 0.0;
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
