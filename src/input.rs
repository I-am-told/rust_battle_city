use macroquad::prelude::{is_key_down, is_key_pressed, KeyCode};
use crate::config::{key_name, parse_keys, KeyList, PlayerKeys, SystemKeys};
use crate::entities::direction::Direction;

pub fn any_down(keys: &[KeyCode]) -> bool {
    keys.iter().any(|k| is_key_down(*k))
}

pub fn any_pressed(keys: &[KeyCode]) -> bool {
    keys.iter().any(|k| is_key_pressed(*k))
}

/// Названия действий игрока — в порядке `KeyBindings::action`.
pub const ACTIONS: [&str; 5] = ["UP", "DOWN", "LEFT", "RIGHT", "FIRE"];

/// Имя клавиши для показа: каноничное из таблицы или Debug-имя.
pub fn display_name(k: KeyCode) -> String {
    key_name(k).map(|s| s.to_string()).unwrap_or_else(|| format!("{k:?}"))
}

/// Клавиши одного игрока. На каждое действие — список клавиш.
#[derive(Clone, Debug)]
pub struct KeyBindings {
    pub up: Vec<KeyCode>,
    pub down: Vec<KeyCode>,
    pub left: Vec<KeyCode>,
    pub right: Vec<KeyCode>,
    pub fire: Vec<KeyCode>,
}

impl KeyBindings {
    /// Разбирает клавиши игрока из конфига. `who` — для сообщений ("P1"),
    /// `d` — раскладка по умолчанию на случай кривых клавиш.
    pub fn from_config(who: &str, pk: &PlayerKeys, d: &PlayerKeys) -> Self {
        Self {
            up:    parse_keys(&format!("{who} up"), pk.up.as_ref(), d.up.as_ref()),
            down:  parse_keys(&format!("{who} down"), pk.down.as_ref(), d.down.as_ref()),
            left:  parse_keys(&format!("{who} left"), pk.left.as_ref(), d.left.as_ref()),
            right: parse_keys(&format!("{who} right"), pk.right.as_ref(), d.right.as_ref()),
            fire:  parse_keys(&format!("{who} fire"), pk.fire.as_ref(), d.fire.as_ref()),
        }
    }

    fn keys_for(&self, d: Direction) -> &[KeyCode] {
        match d {
            Direction::Up => &self.up,
            Direction::Down => &self.down,
            Direction::Left => &self.left,
            Direction::Right => &self.right,
        }
    }

    pub fn fire_pressed(&self) -> bool {
        any_pressed(&self.fire)
    }

    /// Клавиши действия по номеру (0..5, см. ACTIONS).
    pub fn action(&self, i: usize) -> &Vec<KeyCode> {
        match i {
            0 => &self.up,
            1 => &self.down,
            2 => &self.left,
            3 => &self.right,
            _ => &self.fire,
        }
    }

    pub fn action_mut(&mut self, i: usize) -> &mut Vec<KeyCode> {
        match i {
            0 => &mut self.up,
            1 => &mut self.down,
            2 => &mut self.left,
            3 => &mut self.right,
            _ => &mut self.fire,
        }
    }

    /// Обратно в формат конфига (для сохранения в keys.ron).
    pub fn to_player_keys(&self) -> PlayerKeys {
        let names = |keys: &[KeyCode]| {
            Some(KeyList::Many(keys.iter().filter_map(|k| key_name(*k)).map(String::from).collect()))
        };
        PlayerKeys {
            up: names(&self.up),
            down: names(&self.down),
            left: names(&self.left),
            right: names(&self.right),
            fire: names(&self.fire),
        }
    }

    pub fn up_pressed(&self) -> bool {
        any_pressed(&self.up)
    }

    pub fn down_pressed(&self) -> bool {
        any_pressed(&self.down)
    }

    pub fn left_pressed(&self) -> bool {
        any_pressed(&self.left)
    }

    pub fn right_pressed(&self) -> bool {
        any_pressed(&self.right)
    }

    /// Все пары (действие, клавиша) — для поиска конфликтов.
    fn all(&self, who: &str) -> Vec<(String, KeyCode)> {
        let mut v = Vec::new();
        for (name, keys) in [
            ("up", &self.up), ("down", &self.down), ("left", &self.left),
            ("right", &self.right), ("fire", &self.fire),
        ] {
            for k in keys {
                v.push((format!("{who} {name}"), *k));
            }
        }
        v
    }

    /// Короткое описание для экрана меню: "W/S/A/D, fire: Space".
    pub fn describe(&self) -> String {
        let names = |keys: &[KeyCode]| {
            keys.iter().map(|k| display_name(*k)).collect::<Vec<_>>().join("|")
        };
        format!(
            "{}/{}/{}/{}  fire: {}",
            names(&self.up), names(&self.down), names(&self.left),
            names(&self.right), names(&self.fire)
        )
    }
}

/// Общие клавиши: старт/пауза и выход.
#[derive(Clone, Debug)]
pub struct SystemBindings {
    pub start: Vec<KeyCode>,
    pub quit: Vec<KeyCode>,
    pub quit_hold_seconds: f32,
}

impl From<&SystemKeys> for SystemBindings {
    fn from(sk: &SystemKeys) -> Self {
        let d = SystemKeys::default();
        Self {
            start: parse_keys("start", Some(&sk.start), Some(&d.start)),
            quit: parse_keys("quit", Some(&sk.quit), Some(&d.quit)),
            quit_hold_seconds: sk.quit_hold_seconds.max(0.0),
        }
    }
}

/// Предупреждает, если одна клавиша назначена на несколько действий.
pub fn warn_conflicts(players: &[KeyBindings], system: &SystemBindings) {
    let mut all: Vec<(String, KeyCode)> = Vec::new();
    for (i, p) in players.iter().enumerate() {
        all.extend(p.all(&format!("P{}", i + 1)));
    }
    for k in &system.start { all.push(("start".into(), *k)); }
    for k in &system.quit { all.push(("quit".into(), *k)); }
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            if all[i].1 == all[j].1 && all[i].0 != all[j].0 {
                eprintln!(
                    "Внимание: клавиша {:?} назначена и на '{}', и на '{}'.",
                    all[i].1, all[i].0, all[j].0
                );
            }
        }
    }
}

/// Направление движения с правилом «последняя нажатая клавиша главная».
///
/// Ребёнок часто держит две стрелки сразу: танк едет туда, куда нажали
/// последним, а когда её отпускают — снова туда, что ещё зажато.
#[derive(Default, Clone, Debug)]
pub struct DirectionInput {
    /// Зажатые направления в порядке нажатия (последнее — в конце).
    held: Vec<Direction>,
}

impl DirectionInput {
    pub fn update(&mut self, keys: &KeyBindings) -> Option<Direction> {
        let dirs = Direction::all();
        let down = dirs.map(|d| any_down(keys.keys_for(d)));
        let pressed = dirs.map(|d| any_pressed(keys.keys_for(d)));
        self.step(down, pressed)
    }

    /// Логика без опроса клавиатуры (индексы — как в `Direction::all()`).
    fn step(&mut self, down: [bool; 4], pressed: [bool; 4]) -> Option<Direction> {
        let dirs = Direction::all();
        self.held.retain(|d| down[dirs.iter().position(|x| x == d).unwrap()]);
        for (i, d) in dirs.into_iter().enumerate() {
            if pressed[i] && down[i] {
                // Нажали заново — переносим в конец.
                self.held.retain(|x| *x != d);
                self.held.push(d);
            } else if down[i] && !self.held.contains(&d) {
                // Была зажата до начала отслеживания (напр. после паузы).
                self.held.push(d);
            }
        }
        self.held.last().copied()
    }
}
