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
    /// Размер тайла в пикселях — внутреннее разрешение отрисовки.
    /// Танк = 2x2 тайла. На геймплей не влияет: скорости и размеры
    /// ниже заданы в тайлах.
    pub tile_size: f32,
    /// Масштаб окна относительно поля (поле = grid * tile_size пикселей).
    pub window_scale: f32,
    pub grid_width: i32,
    pub grid_height: i32,
    /// Скорость танка игрока, тайлов в секунду.
    pub tank_speed: f32,
    /// Скорость снаряда, тайлов в секунду.
    pub bullet_speed: f32,
    /// Размер снаряда, в тайлах.
    pub bullet_size: f32,
    pub player1_keys: PlayerKeys,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tile_size: 32.0,
            window_scale: 1.0,
            grid_width: 26,
            grid_height: 26,
            tank_speed: 4.0,
            bullet_speed: 10.0,
            bullet_size: 0.2,
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