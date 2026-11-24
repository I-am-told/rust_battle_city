mod config;
mod input;
mod map;
mod entities;
mod game;
mod menu;
mod controls;
mod ui_text;
mod levels;
mod sound;

use macroquad::prelude::*;
use crate::ui_text::{draw_text, measure_text};
use config::load_config;
use game::state::{Game, GameState};
use input::{any_down, any_pressed, warn_conflicts, KeyBindings, SystemBindings};
use config::{
    config_dir, difficulty_path, hiscore_path, keys_file_path, load_difficulty, load_hiscore,
    load_keys_file, load_stage, save_difficulty, save_hiscore, save_stage, stage_path, PlayerKeys,
    helper_path, load_helper, save_helper,
};
use menu::{Menu, MenuAction};
use std::rc::Rc;

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

    // Заполнить атлас шрифта заранее — см. ui_text (чёрные квадраты вместо текста).
    ui_text::warm_up();

    let cfg = load_config();
    let sys: SystemBindings = (&cfg.system_keys).into();
    // Клавиши обоих игроков (в игре на одного используются только P1).
    // base_keys — из config.ron (к ним возвращает RESET в меню CONTROLS),
    // all_keys — с учётом keys.ron (назначены в меню, главнее config.ron).
    let base_keys: Vec<KeyBindings> = vec![
        KeyBindings::from_config("P1", &cfg.player1_keys, &PlayerKeys::player1_default()),
        KeyBindings::from_config("P2", &cfg.player2_keys, &PlayerKeys::player2_default()),
    ];
    let keys_path = keys_file_path(&config_dir());
    let mut all_keys = base_keys.clone();
    if let Some(kf) = load_keys_file(&keys_path) {
        if let Some(pk) = &kf.player1_keys {
            all_keys[0] = KeyBindings::from_config("P1 (keys.ron)", pk, &PlayerKeys::player1_default());
        }
        if let Some(pk) = &kf.player2_keys {
            all_keys[1] = KeyBindings::from_config("P2 (keys.ron)", pk, &PlayerKeys::player2_default());
        }
    }
    warn_conflicts(&all_keys, &sys);

    // Окно = поле + рамка с панелью (как на Dandy). Размер окна можно
    // менять мышью — пропорции сохраняются, лишнее место будет чёрным.
    let frame = game::hud::frame_size(cfg.tile_size, cfg.grid_width, cfg.grid_height);
    let scale = if cfg.window_scale > 0.0 { cfg.window_scale } else { 1.0 };
    request_new_screen_size(frame.x * scale, frame.y * scale);

    let mut audio = sound::Audio::load(cfg.sound_volume, cfg.engine_sound).await;

    // Рекорд хранится в hiscore.txt рядом с config.ron.
    let hiscore_file = hiscore_path(&config_dir());
    let mut hiscore = load_hiscore(&hiscore_file);
    // Сложность (по умолчанию лёгкая) — в difficulty.txt.
    let difficulty_file = difficulty_path(&config_dir());
    let mut difficulty = load_difficulty(&difficulty_file);

    // Уровень, на котором остановились, — в stage.txt (меню предлагает его).
    let stage_file = stage_path(&config_dir());
    let mut stage = load_stage(&stage_file);
    // Сколько карт — чтобы меню знало, где начинается второй круг.
    let map_count = || {
        levels::load_levels(&config_dir().join("levels"), cfg.grid_width, cfg.grid_height).maps.len() as u32
    };

    // Помощник из бонуса «мама» (по умолчанию случайный) — в helper.txt.
    let helper_file = helper_path(&config_dir());
    let mut helper = load_helper(&helper_file);

    let mut screen = Screen::Menu(Menu::new(0, &keys_path, hiscore, difficulty, stage, map_count(), helper));
    // Последний выбор в меню (после GAME OVER курсор стоит на нём).
    let mut last_menu_choice: usize = 0;
    // Сколько секунд удерживается клавиша выхода.
    let mut quit_held: f32 = 0.0;

    loop {
        // Не больше 50 мс за кадр: если окно тащили или система «подвисла»,
        // танки не проскочат сквозь стены одним прыжком.
        let dt = get_frame_time().min(0.05);

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
                let action = menu.update(&mut all_keys, &base_keys, &sys);
                // Сложность переключили в меню — запомнить.
                if menu.helper != helper {
                    helper = menu.helper;
                    save_helper(&helper_file, helper);
                }
                if menu.difficulty != difficulty {
                    difficulty = menu.difficulty;
                    save_difficulty(&difficulty_file, difficulty);
                }
                if let Some(MenuAction::Start(n)) = action {
                    last_menu_choice = menu.selected;
                    stage = menu.stage;
                    save_stage(&stage_file, stage);
                    let keys = all_keys[..n.min(all_keys.len())].to_vec();
                    // Карты перечитываются при каждом старте: нарисовал —
                    // вышел в меню — играешь, без перезапуска программы.
                    let lv = levels::load_levels(
                        &config_dir().join("levels"),
                        cfg.grid_width,
                        cfg.grid_height,
                    );
                    next = Some(Screen::Game(Box::new(Game::new_at(&cfg, keys, Rc::new(lv), hiscore, difficulty, helper, stage))));
                }
                audio.set_engine(false);
                menu.draw(&all_keys);
                draw_quit_hint(false, &sys, quit_held);
            }
            Screen::Game(game) => {
                // Win+P — «чёрный ящик»: сохранить состояние и последние секунды
                // в debug_<время>.txt рядом с config.ron (для разбора редких багов).
                // Пока зажат Win, P не ставит паузу.
                let win_held = is_key_down(KeyCode::LeftSuper) || is_key_down(KeyCode::RightSuper);
                if win_held && is_key_pressed(KeyCode::P) {
                    game.save_debug_dump(&config_dir());
                }
                // Start (по умолчанию Enter или P):
                // - во время игры — пауза / продолжить (как Start на Dandy)
                // - Victory → следующий уровень (жизни и ранги сохраняются)
                // - Defeat → в меню
                if any_pressed(&sys.start) && !win_held {
                    match game.state {
                        GameState::Victory => game.advance_level(&cfg),
                        GameState::Defeat => {
                            next = Some(Screen::Menu(Menu::new(last_menu_choice, &keys_path, hiscore, difficulty, stage, map_count(), helper)));
                        }
                        // Пауза после последнего врага — Enter её пропускает
                        // (сразу экран победы); иначе Enter — пауза.
                        GameState::Playing => {
                            if !game.skip_victory_wait() {
                                game.toggle_pause();
                            }
                        }
                    }
                }
                // Начался новый уровень — запомнить: после GAME OVER или выхода
                // меню предложит продолжить с него.
                if game.current_level != stage {
                    stage = game.current_level;
                    save_stage(&stage_file, stage);
                }
                game.blackbox.tick(dt);
                game.update(dt);
                // Звуки за кадр: одинаковые не дублируем (три взрыва
                // в одном кадре — один звук, а не втрое громче).
                let mut played: Vec<sound::Sfx> = Vec::new();
                for s in game.sfx.drain(..) {
                    if !played.contains(&s) {
                        audio.play(s);
                        played.push(s);
                    }
                }
                audio.set_engine(game.player_moving());
                // Новый рекорд сохраняем, когда уровень закончился
                // (победа или поражение), а не на каждом выстреле.
                if game.state != GameState::Playing && game.hiscore > hiscore {
                    hiscore = game.hiscore;
                    save_hiscore(&hiscore_file, hiscore);
                }
                game.draw();
                game.blackbox.draw_note();
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
