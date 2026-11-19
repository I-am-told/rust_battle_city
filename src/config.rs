use macroquad::prelude::KeyCode;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone)]
pub struct PlayerKeys {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub fire: String,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub tile_size: f32,
    pub grid_width: i32,
    pub grid_height: i32,
    pub tank_speed: f32,
    pub player1_keys: PlayerKeys,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tile_size: 32.0,
            grid_width: 13,
            grid_height: 13,
            tank_speed: 120.0,
            player1_keys: PlayerKeys {
                up: "W".into(),
                down: "S".into(),
                left: "A".into(),
                right: "D".into(),
                fire: "Space".into(),
            },
        }
    }
}

pub fn load_config() -> Config {
    match fs::read_to_string("config.ron") {
        Ok(text) => match ron::from_str::<Config>(&text) {
            Ok(cfg) => {
                println!("Конфиг загружен из config.ron");
                cfg
            }
            Err(e) => {
                eprintln!("Ошибка разбора config.ron: {e}\nИспользую значения по умолчанию.");
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("Не удалось прочитать config.ron: {e}\nИспользую значения по умолчанию.");
            Config::default()
        }
    }
}

pub fn parse_key(name: &str) -> KeyCode {
    match name.to_uppercase().as_str() {
        "W" => KeyCode::W,
        "A" => KeyCode::A,
        "S" => KeyCode::S,
        "D" => KeyCode::D,
        "SPACE" => KeyCode::Space,
        "UP" => KeyCode::Up,
        "DOWN" => KeyCode::Down,
        "LEFT" => KeyCode::Left,
        "RIGHT" => KeyCode::Right,
        "ENTER" => KeyCode::Enter,
        other => {
            eprintln!("Неизвестная клавиша '{other}', использую W.");
            KeyCode::W
        }
    }
}