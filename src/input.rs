use macroquad::prelude::KeyCode;
use crate::config::{PlayerKeys, parse_key};

#[derive(Clone, Copy)]
pub struct KeyBindings {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub fire: KeyCode,
}

impl From<&PlayerKeys> for KeyBindings {
    fn from(pk: &PlayerKeys) -> Self {
        Self {
            up:    parse_key(&pk.up),
            down:  parse_key(&pk.down),
            left:  parse_key(&pk.left),
            right: parse_key(&pk.right),
            fire:  parse_key(&pk.fire),
        }
    }
}