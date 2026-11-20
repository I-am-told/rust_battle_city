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

impl From<&PlayerKeys> for KeyBindings {
    fn from(pk: &PlayerKeys) -> Self {
        let d = PlayerKeys::default();
        Self {
            up:    parse_keys("up", &pk.up, &d.up),
            down:  parse_keys("down", &pk.down, &d.down),
            left:  parse_keys("left", &pk.left, &d.left),
            right: parse_keys("right", &pk.right, &d.right),
            fire:  parse_keys("fire", &pk.fire, &d.fire),
        }
    }
}

impl KeyBindings {
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

    /// Все пары (действие, клавиша) — для поиска конфликтов.
    fn all(&self) -> Vec<(&'static str, KeyCode)> {
        let mut v = Vec::new();
        for (name, keys) in [
            ("up", &self.up), ("down", &self.down), ("left", &self.left),
            ("right", &self.right), ("fire", &self.fire),
        ] {
            for k in keys {
                v.push((name, *k));
            }
        }
        v
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
pub fn warn_conflicts(player: &KeyBindings, system: &SystemBindings) {
    let mut all = player.all();
    for k in &system.start { all.push(("start", *k)); }
    for k in &system.quit { all.push(("quit", *k)); }
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
