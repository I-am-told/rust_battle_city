//! «Чёрный ящик» для редких багов (например, танк не едет, хотя путь
//! свободен): игра постоянно помнит последние ~3 секунды — что зажато на
//! клавиатуре у каждого игрока и где был танк, — а по Ctrl+I сохраняет это
//! вместе с полным состоянием (карта с выбитыми кирпичами, все танки,
//! пули) в файл debug_<время>.txt рядом с config.ron. По файлу ситуацию
//! можно воспроизвести точь-в-точь.

use std::collections::VecDeque;
use std::fmt::Write as _;
use macroquad::prelude::*;
use crate::entities::direction::Direction;
use crate::entities::tank::Tank;
use crate::input::any_down;
use crate::map::Tile;
use crate::ui_text::draw_text;
use super::state::Game;

/// Сколько кадров помнить (при 60 FPS — 3 секунды).
const TRACE_FRAMES: usize = 180;

/// Память последних кадров и надпись «сохранено».
#[derive(Default)]
pub struct BlackBox {
    frame: u64,
    lines: VecDeque<String>,
    /// Сколько ещё показывать надпись о сохранении, секунд, и её текст.
    note: Option<(f32, String)>,
}

impl BlackBox {
    pub fn next_frame(&mut self) {
        self.frame += 1;
    }

    fn push(&mut self, line: String) {
        if self.lines.len() >= TRACE_FRAMES * 2 {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn tick(&mut self, dt: f32) {
        if let Some((t, _)) = &mut self.note {
            *t -= dt;
            if *t <= 0.0 {
                self.note = None;
            }
        }
    }

    /// Надпись внизу экрана после сохранения.
    pub fn draw_note(&self) {
        if let Some((_, text)) = &self.note {
            draw_text(text, 12.0, screen_height() - 12.0, 16.0, YELLOW);
        }
    }
}

fn tank_line(t: &Tank, ts: f32) -> String {
    format!(
        "id={} alive={} pos=({:.3},{:.3})t dir={:?} slide={:?} speed={:.1} rank={} armor={} gassed={} team={:?}",
        t.id,
        t.alive,
        t.pos.x / ts,
        t.pos.y / ts,
        t.direction,
        t.slide_target.map(|v| ((v.x / ts * 1000.0).round() / 1000.0, (v.y / ts * 1000.0).round() / 1000.0)),
        t.speed,
        t.rank,
        t.armor,
        t.gassed,
        t.team,
    )
}

impl Game {
    /// Записать кадр игрока `i`: какие направления зажаты, что выбрано и
    /// где танк до и после хода. Вызывается из update для каждого игрока.
    pub(crate) fn trace_player(&mut self, i: usize, desired: Option<Direction>, before: Vec2) {
        let ts = self.tile_size;
        let p = &self.players[i];
        let mut held: String = Direction::all()
            .iter()
            .filter(|d| any_down(p.keys.keys_for(**d)))
            .map(|d| format!("{d:?} "))
            .collect();
        if any_down(&p.keys.fire) {
            held.push_str("Fire");
        }
        let t = &p.tank;
        let line = format!(
            "f{} P{} keys=[{}] desired={:?} ({:.3},{:.3})->({:.3},{:.3}) dir={:?} slide={}",
            self.blackbox.frame,
            i + 1,
            held.trim_end(),
            desired,
            before.x / ts,
            before.y / ts,
            t.pos.x / ts,
            t.pos.y / ts,
            t.direction,
            t.slide_target.is_some(),
        );
        self.blackbox.push(line);
    }

    /// Полное состояние + последние кадры — текстом.
    pub fn debug_dump(&self) -> String {
        let ts = self.tile_size;
        let mut s = String::new();
        let _ = writeln!(s, "Battle City debug dump");
        let _ = writeln!(
            s,
            "level={} difficulty={:?} players={} lives={} paused={} freeze={:.2} state={:?} frame={}",
            self.current_level,
            self.difficulty,
            self.players.len(),
            self.lives,
            self.paused,
            self.freeze_timer,
            self.state,
            self.blackbox.frame,
        );
        let _ = writeln!(s, "\n[players]");
        for (i, p) in self.players.iter().enumerate() {
            let _ = writeln!(s, "P{} {} invuln={:.2} star={}", i + 1, tank_line(&p.tank, ts), p.invuln_timer, p.star.is_some());
        }
        // Клавиши: что назначено и что зажато прямо сейчас (для «не стреляет»).
        let dir = crate::config::config_dir();
        let dir = std::fs::canonicalize(if dir.as_os_str().is_empty() { std::path::Path::new(".") } else { &dir })
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| dir.display().to_string());
        let _ = writeln!(s, "\n[keys] config dir: {dir}");
        let _ = writeln!(s, "exe: {}", std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default());
        for (i, p) in self.players.iter().enumerate() {
            let k = &p.keys;
            let held: Vec<String> = [&k.up, &k.down, &k.left, &k.right, &k.fire]
                .iter()
                .flat_map(|v| v.iter())
                .filter(|c| is_key_down(**c))
                .map(|c| format!("{c:?}"))
                .collect();
            let _ = writeln!(
                s,
                "P{} up={:?} down={:?} left={:?} right={:?} fire={:?} held_now={:?}",
                i + 1, k.up, k.down, k.left, k.right, k.fire, held
            );
        }
        let _ = writeln!(s, "\n[enemies]");
        for e in &self.enemies {
            let _ = writeln!(s, "{}", tank_line(e, ts));
        }
        let _ = writeln!(s, "\n[spawn stars]");
        for (t, _) in &self.pending {
            let _ = writeln!(s, "{}", tank_line(t, ts));
        }
        let _ = writeln!(s, "\n[bullets]");
        for b in &self.bullets {
            let _ = writeln!(
                s,
                "owner={} team={:?} pos=({:.3},{:.3})t dir={:?} alive={}",
                b.owner_id, b.owner_team, b.pos.x / ts, b.pos.y / ts, b.direction, b.alive
            );
        }
        // Карта: символы как в levels/*.txt; кирпич с выбитыми
        // четвертинками — шестнадцатеричная маска целых (0..e; f = целый = #).
        let _ = writeln!(s, "\n[map] . # @ ~ % - : E(штаб), кирпич с выбоинами — маска 0..e");
        for y in 0..self.map.height {
            let mut row = String::new();
            for x in 0..self.map.width {
                let idx = (y * self.map.width + x) as usize;
                let c = match self.map.tiles[idx] {
                    Tile::Empty => '.',
                    Tile::Brick => match self.map.brick_mask[idx] {
                        0b1111 => '#',
                        m => char::from_digit(m as u32, 16).unwrap_or('?'),
                    },
                    Tile::Steel => '@',
                    Tile::Water => '~',
                    Tile::Eagle => 'E',
                    Tile::Forest => '%',
                    Tile::Ice => '-',
                    Tile::Sand => ':',
                };
                row.push(c);
            }
            let _ = writeln!(s, "{y:2} {row}");
        }
        let _ = writeln!(s, "\n[last frames, координаты в тайлах]");
        for l in &self.blackbox.lines {
            let _ = writeln!(s, "{l}");
        }
        s
    }

    /// Сохранить дамп в `dir`; на экране — надпись, куда сохранён.
    pub fn save_debug_dump(&mut self, dir: &std::path::Path) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("debug_{secs}.txt"));
        let text = match std::fs::write(&path, self.debug_dump()) {
            Ok(()) => format!("DEBUG SAVED: {}", path.display()),
            Err(e) => format!("DEBUG NOT SAVED: {e}"),
        };
        println!("{text}");
        self.blackbox.note = Some((4.0, text));
    }
}
