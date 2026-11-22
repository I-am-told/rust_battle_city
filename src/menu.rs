use std::path::{Path, PathBuf};
use macroquad::prelude::*;
use crate::ui_text::{draw_text, measure_text};
use crate::config::{Difficulty, HelperChoice};
use crate::entities::monkey::{HelperKind, Monkey};
use crate::controls::{ControlsResult, ControlsScreen};
use crate::input::{any_pressed, KeyBindings, SystemBindings};

/// Что выбрали в меню.
pub enum MenuAction {
    /// Начать игру на N игроков.
    Start(usize),
}

/// Пункты меню (в порядке отображения).
const ITEMS: [&str; 6] = ["1 PLAYER", "2 PLAYERS", "MODE", "STAGE", "HELPER", "CONTROLS"];
const ITEM_MODE: usize = 2;
const ITEM_STAGE: usize = 3;
const ITEM_HELPER: usize = 4;
const ITEM_CONTROLS: usize = 5;

/// Титульный экран: выбор числа игроков, как в оригинале,
/// и переход к настройке клавиш.
pub struct Menu {
    pub selected: usize,
    /// Открыт экран CONTROLS.
    controls: Option<ControlsScreen>,
    keys_path: PathBuf,
    hiscore: u32,
    /// Выбранная сложность (переключается пунктом MODE).
    pub difficulty: Difficulty,
    /// С какого уровня начинать (пункт STAGE). По умолчанию — где остановились.
    pub stage: u32,
    /// Сколько карт в папке levels (0 — встроенная тестовая).
    maps: u32,
    /// Помощник из бонуса «мама» (пункт HELPER).
    pub helper: HelperChoice,
}

impl Menu {
    pub fn new(selected: usize, keys_path: &Path, hiscore: u32, difficulty: Difficulty, stage: u32, maps: u32, helper: HelperChoice) -> Self {
        let mut m = Self { selected, controls: None, keys_path: keys_path.to_path_buf(), hiscore, difficulty, stage, maps, helper };
        m.stage = stage.clamp(1, m.max_stage());
        m
    }

    /// Выбрать можно все карты по два круга (второй — с умными врагами).
    fn max_stage(&self) -> u32 {
        let maps = if self.maps == 0 { 15 } else { self.maps };
        maps * 2
    }

    /// Уровень — уже второй круг карт (враги умнее).
    fn stage_is_smart(&self) -> bool {
        let maps = if self.maps == 0 { 15 } else { self.maps };
        self.stage > maps
    }

    fn change_stage(&mut self, delta: i32) {
        let max = self.max_stage() as i32;
        self.stage = ((self.stage as i32 - 1 + delta).rem_euclid(max) + 1) as u32;
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
        let left = players.iter().any(|k| k.left_pressed()) || is_key_pressed(KeyCode::Left);
        let right = players.iter().any(|k| k.right_pressed()) || is_key_pressed(KeyCode::Right);
        // На пункте MODE влево/вправо тоже переключают сложность.
        if self.selected == ITEM_MODE && (left || right) {
            self.difficulty = self.difficulty.toggled();
        }
        // STAGE: влево/вправо — уровень (по кругу).
        if self.selected == ITEM_STAGE {
            if left { self.change_stage(-1); }
            if right { self.change_stage(1); }
        }
        // HELPER: влево/вправо — кто прилетает по бонусу «мама».
        if self.selected == ITEM_HELPER {
            if left { self.helper = self.helper.step(-1); }
            if right { self.helper = self.helper.step(1); }
        }
        if any_pressed(&sys.start) || players.iter().any(|k| k.fire_pressed()) {
            if self.selected == ITEM_MODE {
                self.difficulty = self.difficulty.toggled();
                return None;
            }
            if self.selected == ITEM_STAGE {
                self.change_stage(1);
                return None;
            }
            if self.selected == ITEM_HELPER {
                self.helper = self.helper.step(1);
                return None;
            }
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

        center_text(&format!("HI-SCORE  {}", self.hiscore), h * 0.12, 28.0, LIGHTGRAY);
        center_text("BATTLE CITY", h * 0.28, 72.0, ORANGE);

        let item_size = 40.0;
        let first_y = h * 0.44;
        let step = 50.0;
        for (i, item) in ITEMS.iter().enumerate() {
            let y = first_y + i as f32 * step;
            if i == ITEM_MODE {
                let color = if self.difficulty == Difficulty::Easy { GREEN } else { ORANGE };
                center_text(&format!("MODE: {}", self.difficulty.label()), y, item_size, color);
            } else if i == ITEM_STAGE {
                let color = if self.stage_is_smart() { ORANGE } else { WHITE };
                center_text(&format!("STAGE < {} >", self.stage), y, item_size, color);
                if self.stage_is_smart() {
                    center_text("SMART ENEMIES", y + 20.0, 16.0, ORANGE);
                }
            } else if i == ITEM_HELPER {
                let text = format!("HELPER: {}", self.helper.label());
                center_text(&text, y, item_size, PINK);
                // Картинка помощника справа; у RANDOM — все по очереди.
                let kind = match self.helper {
                    HelperChoice::Fixed(k) => k,
                    HelperChoice::Random => HelperKind::ALL[(get_time() / 0.7) as usize % HelperKind::ALL.len()],
                };
                let dims = measure_text(&text, None, item_size as u16, 1.0);
                let icon = 44.0;
                Monkey::new(kind, vec2(w / 2.0 + dims.width / 2.0 + icon * 0.9, y - icon * 0.35), icon, 1.0).draw();
            } else {
                center_text(item, y, item_size, WHITE);
            }
            if i == self.selected {
                // Курсор — маленький танк слева от пункта.
                // По самой широкой надписи — чтобы курсор не наезжал на текст.
                let dims = measure_text("HELPER: MERMAID", None, item_size as u16, 1.0);
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
