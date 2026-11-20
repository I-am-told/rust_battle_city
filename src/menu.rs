use std::path::{Path, PathBuf};
use macroquad::prelude::*;
use crate::controls::{ControlsResult, ControlsScreen};
use crate::input::{any_pressed, KeyBindings, SystemBindings};

/// Что выбрали в меню.
pub enum MenuAction {
    /// Начать игру на N игроков.
    Start(usize),
}

/// Пункты меню (в порядке отображения).
const ITEMS: [&str; 3] = ["1 PLAYER", "2 PLAYERS", "CONTROLS"];
const ITEM_CONTROLS: usize = 2;

/// Титульный экран: выбор числа игроков, как в оригинале,
/// и переход к настройке клавиш.
pub struct Menu {
    pub selected: usize,
    /// Открыт экран CONTROLS.
    controls: Option<ControlsScreen>,
    keys_path: PathBuf,
}

impl Menu {
    pub fn new(selected: usize, keys_path: &Path) -> Self {
        Self { selected, controls: None, keys_path: keys_path.to_path_buf() }
    }

    /// Вверх/вниз — клавиши любого игрока или стрелки; выбор — Start
    /// (Enter/P) или огонь любого игрока.
    /// `players` — текущие клавиши (на экране CONTROLS меняются),
    /// `base` — клавиши из config.ron (для RESET).
    pub fn update(
        &mut self,
        players: &mut [KeyBindings],
        base: &[KeyBindings],
        sys: &SystemBindings,
    ) -> Option<MenuAction> {
        if let Some(c) = &mut self.controls {
            if let ControlsResult::Back = c.update(players, base, sys) {
                self.controls = None;
            }
            return None;
        }

        let up = players.iter().any(|k| k.up_pressed()) || is_key_pressed(KeyCode::Up);
        let down = players.iter().any(|k| k.down_pressed()) || is_key_pressed(KeyCode::Down);
        if up {
            self.selected = (self.selected + ITEMS.len() - 1) % ITEMS.len();
        }
        if down {
            self.selected = (self.selected + 1) % ITEMS.len();
        }
        if any_pressed(&sys.start) || players.iter().any(|k| k.fire_pressed()) {
            if self.selected == ITEM_CONTROLS {
                self.controls = Some(ControlsScreen::new(&self.keys_path));
                return None;
            }
            return Some(MenuAction::Start(self.selected + 1));
        }
        None
    }

    pub fn draw(&self, players: &[KeyBindings]) {
        if let Some(c) = &self.controls {
            c.draw(players);
            return;
        }
        clear_background(BLACK);
        let w = screen_width();
        let h = screen_height();

        let center_text = |text: &str, y: f32, size: f32, color: Color| {
            let dims = measure_text(text, None, size as u16, 1.0);
            draw_text(text, w / 2.0 - dims.width / 2.0, y, size, color);
        };

        center_text("BATTLE CITY", h * 0.28, 72.0, ORANGE);

        let item_size = 40.0;
        let first_y = h * 0.48;
        let step = 56.0;
        for (i, item) in ITEMS.iter().enumerate() {
            let y = first_y + i as f32 * step;
            center_text(item, y, item_size, WHITE);
            if i == self.selected {
                // Курсор — маленький танк слева от пункта.
                let dims = measure_text(ITEMS[1], None, item_size as u16, 1.0);
                let s = 28.0;
                let x = w / 2.0 - dims.width / 2.0 - s - 24.0;
                let ty = y - s + 4.0;
                draw_rectangle(x, ty, s, s, YELLOW);
                draw_rectangle_lines(x, ty, s, s, 2.0, WHITE);
                draw_line(x + s / 2.0, ty + s / 2.0, x + s, ty + s / 2.0, s * 0.2, WHITE);
            }
        }

        // Подсказка по управлению (берётся из конфига).
        let hint_y = h * 0.80;
        for (i, k) in players.iter().enumerate() {
            center_text(&format!("P{}: {}", i + 1, k.describe()), hint_y + i as f32 * 24.0, 18.0, GRAY);
        }
        center_text("ENTER / fire - start", hint_y + players.len() as f32 * 24.0 + 12.0, 18.0, GRAY);
    }
}
