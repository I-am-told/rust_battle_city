use std::path::{Path, PathBuf};
use macroquad::prelude::*;
use crate::config::{key_name, save_keys_file, KeysFile};
use crate::input::{any_pressed, display_name, KeyBindings, SystemBindings, ACTIONS};

/// Строки экрана: 5 действий, затем RESET и BACK.
const ROW_RESET: usize = ACTIONS.len();
const ROW_BACK: usize = ACTIONS.len() + 1;
const ROWS: usize = ACTIONS.len() + 2;

/// Сколько секунд показывать сообщение внизу экрана.
const MESSAGE_TIME: f64 = 3.0;

pub enum ControlsResult {
    Stay,
    Back,
}

/// Экран переназначения клавиш.
///
/// Управление на этом экране не зависит от назначенных клавиш (чтобы
/// нельзя было «сломать» навигацию): стрелки — выбор клетки,
/// Enter — изменить, Esc — отмена / назад.
/// Новая клавиша заменяет все клавиши действия. Если она уже занята
/// другим действием — клавиши меняются местами (конфликтов не бывает).
pub struct ControlsScreen {
    row: usize,
    /// Столбец = номер игрока (0 — P1, 1 — P2).
    col: usize,
    /// Ждём нажатия новой клавиши.
    waiting: bool,
    /// Пропустить кадр после входа в ожидание: иначе сам Enter,
    /// которым выбрали клетку, тут же назначится как новая клавиша.
    skip_frame: bool,
    message: Option<(String, f64)>,
    keys_path: PathBuf,
}

impl ControlsScreen {
    pub fn new(keys_path: &Path) -> Self {
        Self {
            row: 0,
            col: 0,
            waiting: false,
            skip_frame: false,
            message: None,
            keys_path: keys_path.to_path_buf(),
        }
    }

    fn say(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), get_time()));
    }

    /// `keys` — текущие клавиши игроков (меняются здесь),
    /// `base` — клавиши из config.ron (для RESET).
    pub fn update(
        &mut self,
        keys: &mut [KeyBindings],
        base: &[KeyBindings],
        sys: &SystemBindings,
    ) -> ControlsResult {
        if self.waiting {
            if self.skip_frame {
                self.skip_frame = false;
                return ControlsResult::Stay;
            }
            if let Some(k) = get_last_key_pressed() {
                self.assign(keys, sys, k);
            }
            return ControlsResult::Stay;
        }

        if any_pressed(&sys.quit) || is_key_pressed(KeyCode::Backspace) {
            return ControlsResult::Back;
        }
        if is_key_pressed(KeyCode::Up) {
            self.row = (self.row + ROWS - 1) % ROWS;
        }
        if is_key_pressed(KeyCode::Down) {
            self.row = (self.row + 1) % ROWS;
        }
        let ncols = keys.len().max(1);
        if is_key_pressed(KeyCode::Left) {
            self.col = (self.col + ncols - 1) % ncols;
        }
        if is_key_pressed(KeyCode::Right) {
            self.col = (self.col + 1) % ncols;
        }

        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            match self.row {
                ROW_BACK => return ControlsResult::Back,
                ROW_RESET => self.reset(keys, base),
                _ => {
                    self.waiting = true;
                    self.skip_frame = true;
                }
            }
        }
        ControlsResult::Stay
    }

    /// Назначить клавишу `k` выбранной клетке.
    fn assign(&mut self, keys: &mut [KeyBindings], sys: &SystemBindings, k: KeyCode) {
        if sys.quit.contains(&k) {
            // Esc — отмена.
            self.waiting = false;
            return;
        }
        if sys.start.contains(&k) {
            self.say(format!("{} is reserved for START/PAUSE", display_name(k)));
            return; // продолжаем ждать
        }
        if key_name(k).is_none() {
            self.say(format!("{k:?} is not supported, try another key"));
            return;
        }

        let (p, a) = (self.col, self.row);
        let old: Vec<KeyCode> = keys[p].action(a).clone();

        // Если клавиша занята другой клеткой — отдаём той клетке нашу старую.
        let mut swapped: Option<String> = None;
        for (pi, kb) in keys.iter_mut().enumerate() {
            for (ai, aname) in ACTIONS.iter().enumerate() {
                if (pi, ai) == (p, a) || !kb.action(ai).contains(&k) {
                    continue;
                }
                let list = kb.action_mut(ai);
                list.retain(|x| *x != k);
                if list.is_empty() {
                    if let Some(first) = old.first() {
                        list.push(*first);
                    }
                }
                swapped = Some(format!("P{} {}", pi + 1, aname));
            }
        }
        *keys[p].action_mut(a) = vec![k];
        self.waiting = false;

        let saved = self.save(keys);
        let what = format!("P{} {} = {}", p + 1, ACTIONS[a], display_name(k));
        match (saved, swapped) {
            (Err(e), _) => self.say(format!("{what}, but save failed: {e}")),
            (Ok(()), Some(other)) => self.say(format!("{what} (swapped with {other})")),
            (Ok(()), None) => self.say(format!("{what}, saved")),
        }
    }

    fn save(&self, keys: &[KeyBindings]) -> Result<(), String> {
        let file = KeysFile {
            player1_keys: keys.first().map(|k| k.to_player_keys()),
            player2_keys: keys.get(1).map(|k| k.to_player_keys()),
        };
        save_keys_file(&self.keys_path, &file)
    }

    /// RESET: вернуть клавиши из config.ron и удалить keys.ron.
    fn reset(&mut self, keys: &mut [KeyBindings], base: &[KeyBindings]) {
        for (k, b) in keys.iter_mut().zip(base.iter()) {
            *k = b.clone();
        }
        match std::fs::remove_file(&self.keys_path) {
            Ok(()) => self.say("Keys reset to config.ron"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => self.say("Keys reset to config.ron"),
            Err(e) => self.say(format!("Reset, but could not delete {}: {e}", self.keys_path.display())),
        }
    }

    pub fn draw(&self, keys: &[KeyBindings]) {
        clear_background(BLACK);
        let w = screen_width();
        let h = screen_height();
        let center_text = |text: &str, y: f32, size: f32, color: Color| {
            let dims = measure_text(text, None, size as u16, 1.0);
            draw_text(text, w / 2.0 - dims.width / 2.0, y, size, color);
        };

        center_text("CONTROLS", h * 0.14, 56.0, ORANGE);

        let fs = 28.0;
        let row_h = 42.0;
        let top = h * 0.26;
        let col_label = w * 0.18;
        let col_x = [w * 0.42, w * 0.70];
        let player_color = [YELLOW, GREEN];

        for (i, x) in col_x.iter().enumerate().take(keys.len()) {
            draw_text(format!("P{}", i + 1), *x, top, fs, player_color[i]);
        }

        for (a, name) in ACTIONS.iter().enumerate() {
            let y = top + (a + 1) as f32 * row_h;
            draw_text(name, col_label, y, fs, LIGHTGRAY);
            for (p, kb) in keys.iter().enumerate() {
                let selected = self.row == a && self.col == p;
                let text = if selected && self.waiting {
                    "press a key...".to_string()
                } else {
                    kb.action(a).iter().map(|k| display_name(*k)).collect::<Vec<_>>().join(" | ")
                };
                let x = col_x[p.min(col_x.len() - 1)];
                if selected {
                    let dims = measure_text(&text, None, fs as u16, 1.0);
                    draw_rectangle_lines(x - 8.0, y - fs * 0.8, dims.width.max(60.0) + 16.0, fs * 1.1, 2.0, YELLOW);
                }
                let color = if selected && self.waiting {
                    if ((get_time() * 3.0) as i32) % 2 == 0 { YELLOW } else { ORANGE }
                } else {
                    WHITE
                };
                draw_text(&text, x, y, fs, color);
            }
        }

        let y_reset = top + (ACTIONS.len() + 2) as f32 * row_h;
        let y_back = y_reset + row_h;
        for (row, label, y) in [(ROW_RESET, "RESET TO config.ron", y_reset), (ROW_BACK, "BACK", y_back)] {
            let color = if self.row == row { YELLOW } else { WHITE };
            let text = if self.row == row { format!("> {label} <") } else { label.to_string() };
            center_text(&text, y, fs, color);
        }

        let help = if self.waiting {
            "Press the new key.  Esc - cancel"
        } else {
            "Arrows - select   Enter - change   Esc - back"
        };
        center_text(help, h * 0.86, 20.0, GRAY);

        if let Some((text, t)) = &self.message {
            if get_time() - t < MESSAGE_TIME {
                center_text(text, h * 0.91, 20.0, SKYBLUE);
            }
        }
    }
}
