use macroquad::prelude::KeyCode;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Одна клавиша или список клавиш для действия.
/// В config.ron можно писать и `fire: "Space"`, и `fire: ["Space", "J"]`.
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum KeyList {
    One(String),
    Many(Vec<String>),
}

impl KeyList {
    pub fn names(&self) -> Vec<String> {
        match self {
            KeyList::One(s) => vec![s.clone()],
            KeyList::Many(v) => v.clone(),
        }
    }

    fn many(names: &[&str]) -> Self {
        KeyList::Many(names.iter().map(|s| s.to_string()).collect())
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct PlayerKeys {
    pub up: KeyList,
    pub down: KeyList,
    pub left: KeyList,
    pub right: KeyList,
    pub fire: KeyList,
}

impl Default for PlayerKeys {
    /// Раскладка игрока 1 по умолчанию.
    fn default() -> Self {
        Self {
            up: KeyList::many(&["W"]),
            down: KeyList::many(&["S"]),
            left: KeyList::many(&["A"]),
            right: KeyList::many(&["D"]),
            fire: KeyList::many(&["Space"]),
        }
    }
}

impl PlayerKeys {
    /// Раскладка игрока 2 по умолчанию: стрелки + правый Ctrl или «/».
    pub fn player2_default() -> Self {
        Self {
            up: KeyList::many(&["Up"]),
            down: KeyList::many(&["Down"]),
            left: KeyList::many(&["Left"]),
            right: KeyList::many(&["Right"]),
            fire: KeyList::many(&["RCtrl", "Slash"]),
        }
    }
}

/// Общие клавиши (не привязаны к игроку).
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemKeys {
    /// Пауза во время игры / следующий уровень / новая игра (как Start на Dandy).
    pub start: KeyList,
    /// Выход из игры. Короткое нажатие — пауза, удержание — выход.
    pub quit: KeyList,
    /// Сколько секунд держать клавишу выхода. 0 — выход сразу по нажатию.
    pub quit_hold_seconds: f32,
}

impl Default for SystemKeys {
    fn default() -> Self {
        Self {
            start: KeyList::many(&["Enter", "P"]),
            quit: KeyList::many(&["Escape"]),
            quit_hold_seconds: 2.0,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
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
    /// Помощь при повороте: "off" | "jump" | "smooth".
    pub turn_assist: String,
    pub player1_keys: PlayerKeys,
    pub player2_keys: PlayerKeys,
    pub system_keys: SystemKeys,
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
            turn_assist: "smooth".into(),
            player1_keys: PlayerKeys::default(),
            player2_keys: PlayerKeys::player2_default(),
            system_keys: SystemKeys::default(),
        }
    }
}

const CONFIG_NAME: &str = "config.ron";

/// Где искать конфиг: текущая папка, затем папка с бинарником.
fn config_candidates() -> Vec<PathBuf> {
    let mut out = vec![PathBuf::from(CONFIG_NAME)];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(CONFIG_NAME);
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

/// Загружает конфиг. Поля, которых нет в файле, берутся по умолчанию.
pub fn load_config() -> Config {
    for path in config_candidates() {
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        return match ron::from_str::<Config>(&text) {
            Ok(cfg) => {
                println!("Конфиг загружен: {}", path.display());
                cfg
            }
            Err(e) => {
                eprintln!(
                    "Ошибка разбора {}: {e}\nИспользую значения по умолчанию.",
                    path.display()
                );
                Config::default()
            }
        };
    }
    eprintln!("{CONFIG_NAME} не найден (ни в текущей папке, ни рядом с программой). Использую значения по умолчанию.");
    Config::default()
}

/// Переводит имя клавиши из конфига в KeyCode. Регистр не важен.
/// Буквы: "A".."Z"; цифры: "0".."9"; цифровой блок: "Num0".."Num9",
/// "NumEnter", "NumPlus", "NumMinus", "NumMultiply", "NumDivide", "NumDot";
/// стрелки: "Up", "Down", "Left", "Right"; "F1".."F12";
/// "Space", "Enter", "Tab", "Backspace", "Escape"/"Esc", "Insert", "Delete",
/// "Home", "End", "PageUp", "PageDown", "CapsLock";
/// модификаторы: "LShift", "RShift", "LCtrl", "RCtrl", "LAlt", "RAlt"
/// (а также "Shift", "Ctrl", "Alt" = левые);
/// знаки: "Minus", "Equal", "LeftBracket", "RightBracket", "Semicolon",
/// "Apostrophe", "Comma", "Period", "Slash", "Backslash", "Grave".
pub fn parse_key(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    let n = name.trim().to_uppercase();
    let k = match n.as_str() {
        "A" => A, "B" => B, "C" => C, "D" => D, "E" => E, "F" => F, "G" => G,
        "H" => H, "I" => I, "J" => J, "K" => K, "L" => L, "M" => M, "N" => N,
        "O" => O, "P" => P, "Q" => Q, "R" => R, "S" => S, "T" => T, "U" => U,
        "V" => V, "W" => W, "X" => X, "Y" => Y, "Z" => Z,

        "0" => Key0, "1" => Key1, "2" => Key2, "3" => Key3, "4" => Key4,
        "5" => Key5, "6" => Key6, "7" => Key7, "8" => Key8, "9" => Key9,

        "NUM0" | "KP0" => Kp0, "NUM1" | "KP1" => Kp1, "NUM2" | "KP2" => Kp2,
        "NUM3" | "KP3" => Kp3, "NUM4" | "KP4" => Kp4, "NUM5" | "KP5" => Kp5,
        "NUM6" | "KP6" => Kp6, "NUM7" | "KP7" => Kp7, "NUM8" | "KP8" => Kp8,
        "NUM9" | "KP9" => Kp9,
        "NUMENTER" | "KPENTER" => KpEnter,
        "NUMPLUS" | "KPADD" => KpAdd,
        "NUMMINUS" | "KPSUBTRACT" => KpSubtract,
        "NUMMULTIPLY" | "KPMULTIPLY" => KpMultiply,
        "NUMDIVIDE" | "KPDIVIDE" => KpDivide,
        "NUMDOT" | "KPDECIMAL" => KpDecimal,

        "UP" => Up, "DOWN" => Down, "LEFT" => Left, "RIGHT" => Right,

        "F1" => F1, "F2" => F2, "F3" => F3, "F4" => F4, "F5" => F5, "F6" => F6,
        "F7" => F7, "F8" => F8, "F9" => F9, "F10" => F10, "F11" => F11, "F12" => F12,

        "SPACE" => Space,
        "ENTER" | "RETURN" => Enter,
        "TAB" => Tab,
        "BACKSPACE" => Backspace,
        "ESCAPE" | "ESC" => Escape,
        "INSERT" => Insert,
        "DELETE" => Delete,
        "HOME" => Home,
        "END" => End,
        "PAGEUP" => PageUp,
        "PAGEDOWN" => PageDown,
        "CAPSLOCK" => CapsLock,

        "SHIFT" | "LSHIFT" | "LEFTSHIFT" => LeftShift,
        "RSHIFT" | "RIGHTSHIFT" => RightShift,
        "CTRL" | "LCTRL" | "LEFTCTRL" | "LEFTCONTROL" => LeftControl,
        "RCTRL" | "RIGHTCTRL" | "RIGHTCONTROL" => RightControl,
        "ALT" | "LALT" | "LEFTALT" => LeftAlt,
        "RALT" | "RIGHTALT" => RightAlt,

        "MINUS" => Minus,
        "EQUAL" => Equal,
        "LEFTBRACKET" => LeftBracket,
        "RIGHTBRACKET" => RightBracket,
        "SEMICOLON" => Semicolon,
        "APOSTROPHE" => Apostrophe,
        "COMMA" => Comma,
        "PERIOD" => Period,
        "SLASH" => Slash,
        "BACKSLASH" => Backslash,
        "GRAVE" => GraveAccent,

        _ => return None,
    };
    Some(k)
}

/// Переводит список имён в список клавиш. Неизвестные имена пропускаются
/// с предупреждением; если не осталось ни одной — берутся `fallback`.
pub fn parse_keys(action: &str, list: &KeyList, fallback: &KeyList) -> Vec<KeyCode> {
    let mut out = Vec::new();
    for name in list.names() {
        match parse_key(&name) {
            Some(k) => {
                if !out.contains(&k) {
                    out.push(k);
                }
            }
            None => eprintln!("Неизвестная клавиша '{name}' для '{action}' — пропускаю."),
        }
    }
    if out.is_empty() {
        eprintln!("Для '{action}' не задано ни одной рабочей клавиши — беру по умолчанию.");
        out = fallback.names().iter().filter_map(|n| parse_key(n)).collect();
    }
    out
}
