use macroquad::prelude::KeyCode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Одна клавиша или список клавиш для действия.
/// В config.ron можно писать и `fire: "Space"`, и `fire: ["Space", "J"]`.
#[derive(Deserialize, Serialize, Clone, Debug)]
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

    pub fn many(names: &[&str]) -> Self {
        KeyList::Many(names.iter().map(|s| s.to_string()).collect())
    }
}

/// Клавиши игрока. Действие, которого нет в файле (None), берётся
/// из раскладки по умолчанию ЭТОГО игрока (см. KeyBindings::from_config),
/// а не из раскладки P1.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default)]
pub struct PlayerKeys {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<KeyList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<KeyList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<KeyList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<KeyList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fire: Option<KeyList>,
}

impl PlayerKeys {
    /// Раскладка игрока 1 по умолчанию: WASD + левый Ctrl.
    pub fn player1_default() -> Self {
        Self {
            up: Some(KeyList::many(&["W"])),
            down: Some(KeyList::many(&["S"])),
            left: Some(KeyList::many(&["A"])),
            right: Some(KeyList::many(&["D"])),
            fire: Some(KeyList::many(&["LCtrl"])),
        }
    }

    /// Раскладка игрока 2 по умолчанию: стрелки + пробел.
    pub fn player2_default() -> Self {
        Self {
            up: Some(KeyList::many(&["Up"])),
            down: Some(KeyList::many(&["Down"])),
            left: Some(KeyList::many(&["Left"])),
            right: Some(KeyList::many(&["Right"])),
            fire: Some(KeyList::many(&["Space"])),
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

/// Настройки игрового процесса (времена, множители, очки).
/// Всё в секундах, если не сказано иное.
#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(default)]
pub struct Gameplay {
    /// Запасных жизней на старте — на каждого игрока (запас общий).
    pub start_lives_per_player: u32,
    /// Пауза между гибелью игрока и звёздочкой возрождения.
    pub respawn_delay: f32,
    /// Сколько горит звёздочка перед появлением танка.
    pub spawn_star_time: f32,
    /// Защитное поле после появления игрока.
    pub spawn_shield: f32,
    /// Защитное поле от бонуса-каски.
    pub helmet_shield: f32,
    /// Сколько стоят враги после бонуса-часов.
    pub freeze_time: f32,
    /// Сколько держится сталь вокруг штаба после бонуса-лопаты.
    pub shovel_time: f32,
    /// Сколько бонус лежит на поле, пока не исчезнет.
    pub bonus_lifetime: f32,
    /// Сколько игра идёт после гибели последнего врага (успеть взять бонус).
    pub victory_delay: f32,
    /// За сколько секунд танк доезжает до точки поворота (turn_assist "smooth").
    pub turn_slide_time: f32,
    /// Сколько тайлов танк игрока скользит по льду, если отпустить кнопку.
    pub ice_slide_tiles: f32,
    /// Во сколько раз быстрее танк игрока на 4-м ранге.
    pub rank4_speed_mult: f32,
    /// Скорость любого танка на песке (0.5 — вдвое медленнее).
    pub sand_speed_mult: f32,
    /// Короткий щит после того, как броня (ранг 5+) приняла попадание.
    pub armor_hit_shield: f32,
    /// Перезарядка телепорта-защитника (ранг 6).
    pub guardian_cooldown: f32,
    /// Сколько секунд действует бонус «мама» (обезьянка стреляет облачками).
    pub mama_time: f32,
    /// Как часто обезьянка выпускает облачко, секунд.
    pub monkey_fire_interval: f32,
    /// Скорость облачка, тайлов в секунду.
    pub cloud_speed: f32,
    /// Насколько «умны» враги со второго круга карт: шанс, что враг поедет
    /// к цели (игрок/штаб) и развернётся к ней, если она на одной линии.
    /// 0 — враги всегда ездят случайно; 1 — всегда к цели.
    pub smart_ai_bias: f32,
    /// Обезьянка «мама»: радиус круга вокруг сына (в размерах танка)
    /// и скорость облёта, оборотов в секунду.
    pub monkey_orbit_radius: f32,
    pub monkey_orbit_speed: f32,
    /// Скорость врагов относительно tank_speed (у быстрых — ещё ×2).
    pub enemy_speed_mult: f32,
    /// Скорость вражеских пуль относительно bullet_speed.
    pub enemy_bullet_speed_mult: f32,
    /// Каждые столько очков — дополнительная жизнь.
    pub extra_life_every: u32,
    /// Очки за подобранный бонус.
    pub bonus_points: u32,
}

impl Default for Gameplay {
    fn default() -> Self {
        Self {
            start_lives_per_player: 2,
            respawn_delay: 1.0,
            spawn_star_time: 1.0,
            spawn_shield: 3.0,
            helmet_shield: 10.0,
            freeze_time: 10.0,
            shovel_time: 10.0,
            bonus_lifetime: 15.0,
            victory_delay: 5.0,
            turn_slide_time: 0.1,
            ice_slide_tiles: 1.0,
            rank4_speed_mult: 1.3,
            sand_speed_mult: 0.5,
            armor_hit_shield: 1.0,
            guardian_cooldown: 5.0,
            mama_time: 10.0,
            monkey_fire_interval: 0.6,
            cloud_speed: 7.0,
            smart_ai_bias: 0.6,
            monkey_orbit_radius: 1.0,
            monkey_orbit_speed: 0.35,
            enemy_speed_mult: 0.75,
            enemy_bullet_speed_mult: 0.75,
            extra_life_every: 20_000,
            bonus_points: 500,
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
    /// Громкость звука 0.0..1.0 (0 — без звука).
    pub sound_volume: f32,
    /// Гул мотора, пока танк игрока едет.
    pub engine_sound: bool,
    /// Настройки игрового процесса.
    pub gameplay: Gameplay,
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
            sound_volume: 0.6,
            engine_sound: true,
            gameplay: Gameplay::default(),
            player1_keys: PlayerKeys::player1_default(),
            player2_keys: PlayerKeys::player2_default(),
            system_keys: SystemKeys::default(),
        }
    }
}

const CONFIG_NAME: &str = "config.ron";

/// Папка, где лежит найденный config.ron (или текущая, если его нет).
/// Там же хранится keys.ron с клавишами, назначенными в меню.
pub fn config_dir() -> PathBuf {
    for path in config_candidates() {
        if path.is_file() {
            if let Some(dir) = path.parent() {
                return dir.to_path_buf();
            }
        }
    }
    PathBuf::from(".")
}

pub const KEYS_FILE_NAME: &str = "keys.ron";

/// Клавиши, назначенные в меню CONTROLS. Если файл есть, его клавиши
/// главнее клавиш из config.ron (сам config.ron не перезаписывается).
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default)]
pub struct KeysFile {
    pub player1_keys: Option<PlayerKeys>,
    pub player2_keys: Option<PlayerKeys>,
}

/// RON-настройки для config.ron и keys.ron: `Some(...)` писать не нужно
/// (implicit_some), чтобы файлы было удобно править руками.
fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
}

pub fn keys_file_path(dir: &Path) -> PathBuf {
    let p = dir.join(KEYS_FILE_NAME);
    // "./keys.ron" в сообщениях выглядит хуже, чем "keys.ron".
    p.strip_prefix(".").map(|x| x.to_path_buf()).unwrap_or(p)
}

/// Читает keys.ron. Нет файла — None; ошибка разбора — предупреждение и None.
pub fn load_keys_file(path: &Path) -> Option<KeysFile> {
    let text = fs::read_to_string(path).ok()?;
    match ron_options().from_str::<KeysFile>(&text) {
        Ok(k) => {
            println!("Клавиши из меню загружены: {}", path.display());
            Some(k)
        }
        Err(e) => {
            eprintln!("Ошибка разбора {}: {e}\nИгнорирую, беру клавиши из config.ron.", path.display());
            None
        }
    }
}

/// Записывает keys.ron (через временный файл, чтобы не оставить битый файл).
pub fn save_keys_file(path: &Path, keys: &KeysFile) -> Result<(), String> {
    let pretty = ron::ser::PrettyConfig::new().compact_arrays(true);
    let body = ron_options()
        .to_string_pretty(keys, pretty)
        .map_err(|e| e.to_string())?;
    let text = format!(
        "// Клавиши, назначенные в меню CONTROLS. Главнее, чем в config.ron.\n\
         // Удалите этот файл (или нажмите RESET в меню), чтобы вернуть клавиши из config.ron.\n{body}\n"
    );
    let tmp = path.with_extension("ron.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

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

/// Разбирает текст config.ron.
pub fn parse_config(text: &str) -> Result<Config, ron::error::SpannedError> {
    ron_options().from_str::<Config>(text)
}

/// Загружает конфиг. Поля, которых нет в файле, берутся по умолчанию.
pub fn load_config() -> Config {
    for path in config_candidates() {
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        return match parse_config(&text) {
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

/// Каноничные имена клавиш: так они пишутся в keys.ron и показываются
/// в меню. В config.ron можно писать и эти имена, и синонимы (см. ALIASES).
/// Регистр не важен.
const KEY_TABLE: &[(&str, KeyCode)] = {
    use KeyCode::*;
    &[
        ("A", A), ("B", B), ("C", C), ("D", D), ("E", E), ("F", F), ("G", G),
        ("H", H), ("I", I), ("J", J), ("K", K), ("L", L), ("M", M), ("N", N),
        ("O", O), ("P", P), ("Q", Q), ("R", R), ("S", S), ("T", T), ("U", U),
        ("V", V), ("W", W), ("X", X), ("Y", Y), ("Z", Z),
        ("0", Key0), ("1", Key1), ("2", Key2), ("3", Key3), ("4", Key4),
        ("5", Key5), ("6", Key6), ("7", Key7), ("8", Key8), ("9", Key9),
        ("Num0", Kp0), ("Num1", Kp1), ("Num2", Kp2), ("Num3", Kp3), ("Num4", Kp4),
        ("Num5", Kp5), ("Num6", Kp6), ("Num7", Kp7), ("Num8", Kp8), ("Num9", Kp9),
        ("NumEnter", KpEnter), ("NumPlus", KpAdd), ("NumMinus", KpSubtract),
        ("NumMultiply", KpMultiply), ("NumDivide", KpDivide), ("NumDot", KpDecimal),
        ("Up", Up), ("Down", Down), ("Left", Left), ("Right", Right),
        ("F1", F1), ("F2", F2), ("F3", F3), ("F4", F4), ("F5", F5), ("F6", F6),
        ("F7", F7), ("F8", F8), ("F9", F9), ("F10", F10), ("F11", F11), ("F12", F12),
        ("Space", Space), ("Enter", Enter), ("Tab", Tab), ("Backspace", Backspace),
        ("Escape", Escape), ("Insert", Insert), ("Delete", Delete),
        ("Home", Home), ("End", End), ("PageUp", PageUp), ("PageDown", PageDown),
        ("CapsLock", CapsLock),
        ("LShift", LeftShift), ("RShift", RightShift),
        ("LCtrl", LeftControl), ("RCtrl", RightControl),
        ("LAlt", LeftAlt), ("RAlt", RightAlt),
        ("Minus", Minus), ("Equal", Equal), ("LeftBracket", LeftBracket),
        ("RightBracket", RightBracket), ("Semicolon", Semicolon),
        ("Apostrophe", Apostrophe), ("Comma", Comma), ("Period", Period),
        ("Slash", Slash), ("Backslash", Backslash), ("Grave", GraveAccent),
    ]
};

/// Синонимы (только для чтения конфига).
const ALIASES: &[(&str, KeyCode)] = {
    use KeyCode::*;
    &[
        ("Kp0", Kp0), ("Kp1", Kp1), ("Kp2", Kp2), ("Kp3", Kp3), ("Kp4", Kp4),
        ("Kp5", Kp5), ("Kp6", Kp6), ("Kp7", Kp7), ("Kp8", Kp8), ("Kp9", Kp9),
        ("KpEnter", KpEnter), ("KpAdd", KpAdd), ("KpSubtract", KpSubtract),
        ("KpMultiply", KpMultiply), ("KpDivide", KpDivide), ("KpDecimal", KpDecimal),
        ("Return", Enter), ("Esc", Escape),
        ("Shift", LeftShift), ("LeftShift", LeftShift), ("RightShift", RightShift),
        ("Ctrl", LeftControl), ("LeftCtrl", LeftControl), ("LeftControl", LeftControl),
        ("RightCtrl", RightControl), ("RightControl", RightControl),
        ("Alt", LeftAlt), ("LeftAlt", LeftAlt), ("RightAlt", RightAlt),
    ]
};

/// Переводит имя клавиши из конфига в KeyCode. Регистр не важен.
/// Полный список имён — в KEY_TABLE и ALIASES (и в комментарии в config.ron).
pub fn parse_key(name: &str) -> Option<KeyCode> {
    let n = name.trim();
    KEY_TABLE
        .iter()
        .chain(ALIASES.iter())
        .find(|(k, _)| k.eq_ignore_ascii_case(n))
        .map(|(_, code)| *code)
}

/// Каноничное имя клавиши (для keys.ron и меню). None — клавиша не поддерживается.
pub fn key_name(code: KeyCode) -> Option<&'static str> {
    KEY_TABLE.iter().find(|(_, c)| *c == code).map(|(n, _)| *n)
}

/// Переводит список имён в список клавиш. Нет списка (None) — берётся
/// `fallback`. Неизвестные имена пропускаются с предупреждением;
/// если не осталось ни одной — тоже берётся `fallback`.
pub fn parse_keys(action: &str, list: Option<&KeyList>, fallback: Option<&KeyList>) -> Vec<KeyCode> {
    let fallback_keys = || -> Vec<KeyCode> {
        fallback.map(|f| f.names().iter().filter_map(|n| parse_key(n)).collect()).unwrap_or_default()
    };
    let Some(list) = list else { return fallback_keys() };
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
        out = fallback_keys();
    }
    out
}

/// Файл рекорда (рядом с config.ron).
pub fn hiscore_path(dir: &Path) -> PathBuf {
    let p = dir.join("hiscore.txt");
    p.strip_prefix(".").map(|x| x.to_path_buf()).unwrap_or(p)
}

/// Рекорд из файла; нет файла или мусор — 0.
pub fn load_hiscore(path: &Path) -> u32 {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0)
}

pub fn save_hiscore(path: &Path, value: u32) {
    if let Err(e) = fs::write(path, format!("{value}\n")) {
        eprintln!("Не удалось сохранить рекорд в {}: {e}", path.display());
    }
}

/// Сложность. Выбирается в меню и запоминается в difficulty.txt.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Difficulty {
    /// Лёгкая: помощь второму игроку (сыну) — бонусы рядом с ним и
    /// «умные», граната не трогает бонусного врага, в спину не убить,
    /// звезда P1 — и ему тоже, его танк виден в лесу.
    Easy,
    /// Обычная: как в оригинале.
    Normal,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "EASY",
            Difficulty::Normal => "NORMAL",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Difficulty::Easy => Difficulty::Normal,
            Difficulty::Normal => Difficulty::Easy,
        }
    }
}

pub fn difficulty_path(dir: &Path) -> PathBuf {
    let p = dir.join("difficulty.txt");
    p.strip_prefix(".").map(|x| x.to_path_buf()).unwrap_or(p)
}

/// Сложность из файла; нет файла или непонятное значение — лёгкая.
pub fn load_difficulty(path: &Path) -> Difficulty {
    match fs::read_to_string(path).map(|t| t.trim().to_lowercase()) {
        Ok(t) if t == "normal" => Difficulty::Normal,
        _ => Difficulty::Easy,
    }
}

pub fn save_difficulty(path: &Path, d: Difficulty) {
    if let Err(e) = fs::write(path, format!("{}\n", d.label().to_lowercase())) {
        eprintln!("Не удалось сохранить сложность в {}: {e}", path.display());
    }
}
