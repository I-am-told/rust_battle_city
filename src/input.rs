use macroquad::prelude::{is_key_down, is_key_pressed, KeyCode};
use crate::config::{parse_keys, PlayerKeys, SystemKeys};
use crate::entities::direction::Direction;

pub fn any_down(keys: &[KeyCode]) -> bool {
    keys.iter().any(|k| is_key_down(*k))
}

pub fn any_pressed(keys: &[KeyCode]) -> bool {
    keys.iter().any(|k| is_key_pressed(*k))
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
            up:    parse_keys(&format!("{who} up"), &pk.up, &d.up),
            down:  parse_keys(&format!("{who} down"), &pk.down, &d.down),
            left:  parse_keys(&format!("{who} left"), &pk.left, &d.left),
            right: parse_keys(&format!("{who} right"), &pk.right, &d.right),
            fire:  parse_keys(&format!("{who} fire"), &pk.fire, &d.fire),
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

    pub fn up_pressed(&self) -> bool {
        any_pressed(&self.up)
    }

    pub fn down_pressed(&self) -> bool {
        any_pressed(&self.down)
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
            keys.iter().map(|k| format!("{k:?}")).collect::<Vec<_>>().join("|")
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
            start: parse_keys("start", &sk.start, &d.start),
            quit: parse_keys("quit", &sk.quit, &d.quit),
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
