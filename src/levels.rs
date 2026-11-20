//! Карты уровней из текстовых файлов (папка `levels` рядом с config.ron).
//!
//! Формат — см. levels/README.md. Коротко: одна строка файла = один ряд
//! карты, один символ = одна клетка:
//!   `.` пусто   `#` кирпич   `@` сталь   `~` вода
//! Размер — либо 13x13 «блоков» (блок = клетка размером с танк, как в
//! оригинале), либо 26x26 тайлов (тайл = половина танка — для тонких стен).
//! Строки, начинающиеся с `;`, и пустые строки пропускаются.
//! Штаб, стена вокруг него и точки появления танков ставятся/расчищаются
//! игрой автоматически.

use std::fs;
use std::path::Path;
use crate::map::Tile;

/// Одна загруженная карта.
pub struct LevelMap {
    /// Имя файла (для сообщений).
    pub name: String,
    /// Тайлы поля построчно, длина grid_w * grid_h.
    pub tiles: Vec<Tile>,
}

/// Все карты из папки levels, по порядку имён файлов.
#[derive(Default)]
pub struct LevelSet {
    pub maps: Vec<LevelMap>,
}

impl LevelSet {
    /// Раскладка для уровня `level` (с 1). Карты идут по кругу:
    /// если карт 5, то 6-й уровень снова на первой (но враги сильнее).
    /// None — карт нет, будет встроенная тестовая.
    pub fn layout_for(&self, level: u32) -> Option<&[Tile]> {
        if self.maps.is_empty() {
            return None;
        }
        let i = (level.max(1) as usize - 1) % self.maps.len();
        Some(&self.maps[i].tiles)
    }
}

fn tile_for(c: char) -> Option<Tile> {
    match c {
        '.' => Some(Tile::Empty),
        '#' => Some(Tile::Brick),
        '@' => Some(Tile::Steel),
        '~' => Some(Tile::Water),
        _ => None,
    }
}

/// Разбирает текст карты в тайлы поля `grid_w` x `grid_h`.
/// Ошибка — строка с понятным описанием (номер строки и столбца).
pub fn parse_level(text: &str, grid_w: i32, grid_h: i32) -> Result<Vec<Tile>, String> {
    // (номер строки в файле, символы)
    let rows: Vec<(usize, Vec<char>)> = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_end()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with(';'))
        .map(|(i, l)| (i, l.chars().collect()))
        .collect();

    let h = rows.len() as i32;
    // Масштаб: 1 — клетка = тайл, 2 — клетка = блок 2x2 тайла.
    let scale = if h == grid_h {
        1
    } else if h * 2 == grid_h {
        2
    } else {
        return Err(format!(
            "строк карты {h}, а нужно {grid_h} (по тайлам) или {} (по блокам)",
            grid_h / 2
        ));
    };
    let need_w = grid_w / scale;

    let mut tiles = vec![Tile::Empty; (grid_w * grid_h) as usize];
    for (r, (line_no, chars)) in rows.iter().enumerate() {
        if chars.len() as i32 != need_w {
            return Err(format!(
                "строка {line_no}: {} символов, а нужно {need_w}",
                chars.len()
            ));
        }
        for (c, ch) in chars.iter().enumerate() {
            let tile = tile_for(*ch).ok_or_else(|| {
                format!(
                    "строка {line_no}, столбец {}: неизвестный символ '{ch}' (можно . # @ ~)",
                    c + 1
                )
            })?;
            for dy in 0..scale {
                for dx in 0..scale {
                    let x = c as i32 * scale + dx;
                    let y = r as i32 * scale + dy;
                    tiles[(x + y * grid_w) as usize] = tile;
                }
            }
        }
    }
    Ok(tiles)
}

/// Загружает все `*.txt` из папки `dir` по порядку имён.
/// Файлы с ошибками пропускаются с сообщением в консоль.
pub fn load_levels(dir: &Path, grid_w: i32, grid_h: i32) -> LevelSet {
    let mut files: Vec<_> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("txt")))
            .collect(),
        Err(_) => {
            eprintln!(
                "Папка с картами {} не найдена — играю на встроенной тестовой карте.",
                dir.display()
            );
            return LevelSet::default();
        }
    };
    files.sort();

    let mut set = LevelSet::default();
    for path in files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Карта {name}: не удалось прочитать ({e}) — пропускаю.");
                continue;
            }
        };
        match parse_level(&text, grid_w, grid_h) {
            Ok(tiles) => set.maps.push(LevelMap { name, tiles }),
            Err(e) => eprintln!("Карта {name}: {e} — пропускаю."),
        }
    }
    if set.maps.is_empty() {
        eprintln!("В {} нет годных карт — играю на встроенной тестовой карте.", dir.display());
    } else {
        let names: Vec<&str> = set.maps.iter().map(|m| m.name.as_str()).collect();
        println!("Карты: {}", names.join(", "));
    }
    set
}
