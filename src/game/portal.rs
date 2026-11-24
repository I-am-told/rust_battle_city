//! Телепорт-защитник (ранг 6) с порталами: зелёный портал раскрывается
//! на танке и в точке, куда его переносит, танк «проваливается» в первый
//! и выезжает из второго, порталы закрываются. Пока это идёт, вся игра
//! замедлена (по умолчанию втрое) — чтобы перенос не был внезапным.
//! Весь процесс — PORTAL_TIME секунд реального времени.
//!
//! Перенос в логике происходит сразу (танк уже стоит на месте и
//! принимает пулю на себя), а анимация только показывает, как он туда
//! попал. Пока она идёт, этот танк не ездит и не стреляет.

use macroquad::prelude::*;
use std::f32::consts::TAU;
use super::state::Game;

/// Длительность всего телепорта в реальных секундах.
pub const PORTAL_TIME: f32 = 1.3;
/// Доли времени: раскрытие порталов + «проваливание» танка в первый
/// (0..DIVE), выезд из второго (DIVE..EMERGE), закрытие порталов (EMERGE..1).
const DIVE: f32 = 0.35;
const EMERGE: f32 = 0.65;
/// Во сколько раз замедляется игра во время телепорта.
const SLOWDOWN: f32 = 3.0;

/// Идущий телепорт.
#[derive(Clone, Debug)]
pub struct PortalJump {
    /// Кого переносят (индекс игрока).
    pub player: usize,
    /// Центры порталов: откуда и куда.
    pub from: Vec2,
    pub to: Vec2,
    /// Сколько прошло, реальных секунд.
    pub t: f32,
}

impl PortalJump {
    fn k(&self) -> f32 {
        (self.t / PORTAL_TIME).clamp(0.0, 1.0)
    }

    /// Множитель скорости игры: 1/SLOWDOWN, в последние 15% плавно к 1.
    pub fn time_scale(&self) -> f32 {
        let slow = 1.0 / SLOWDOWN;
        let k = self.k();
        if k < 0.85 { slow } else { slow + (1.0 - slow) * (k - 0.85) / 0.15 }
    }

    /// Раскрытие портала 0..1: быстро раскрывается, держится, закрывается.
    fn open(&self) -> f32 {
        let k = self.k();
        if k < 0.2 {
            k / 0.2
        } else if k > EMERGE {
            1.0 - (k - EMERGE) / (1.0 - EMERGE)
        } else {
            1.0
        }
    }
}

/// Портал «как в Рике и Морти»: кислотно-зелёный круг со светящимся
/// краем и закручивающейся спиралью внутри. `open` — 0..1.
pub fn draw_portal(c: Vec2, size: f32, open: f32, t: f32) {
    if open <= 0.0 {
        return;
    }
    let r = size * 0.72 * open;
    // Свечение.
    draw_circle(c.x, c.y, r * 1.25, Color::new(0.4, 1.0, 0.3, 0.18));
    draw_circle(c.x, c.y, r * 1.1, Color::new(0.45, 1.0, 0.35, 0.3));
    // Кромка и тело.
    draw_circle(c.x, c.y, r, Color::new(0.55, 1.0, 0.25, 1.0));
    draw_circle(c.x, c.y, r * 0.88, Color::new(0.15, 0.7, 0.15, 1.0));
    draw_circle(c.x, c.y, r * 0.65, Color::new(0.3, 0.85, 0.25, 1.0));
    // Спираль из трёх рукавов, крутится.
    for arm in 0..3 {
        for i in 0..14 {
            let k = i as f32 / 13.0;
            let a = -t * 5.0 + arm as f32 * TAU / 3.0 + k * 4.0;
            let rr = r * (0.85 - 0.75 * k);
            let p = c + vec2(a.cos(), a.sin()) * rr;
            draw_circle(p.x, p.y, r * (0.11 - 0.06 * k), Color::new(0.75, 1.0, 0.55, 0.85));
        }
    }
    // Светлая серединка.
    draw_circle(c.x, c.y, r * 0.22, Color::new(0.85, 1.0, 0.75, 1.0));
    // Искры по краю.
    for i in 0..6 {
        let a = t * 3.0 + i as f32 * TAU / 6.0;
        let p = c + vec2(a.cos(), a.sin()) * r * (1.05 + 0.08 * (t * 13.0 + i as f32).sin());
        draw_circle(p.x, p.y, r * 0.06, Color::new(0.9, 1.0, 0.7, 0.9));
    }
}

impl Game {
    /// Начать телепорт игрока `gi` из `from` в `to` (центры).
    pub(crate) fn start_portal(&mut self, gi: usize, from: Vec2, to: Vec2) {
        self.portal = Some(PortalJump { player: gi, from, to, t: 0.0 });
    }

    /// Продвинуть анимацию на `real_dt` и вернуть множитель скорости игры.
    pub(crate) fn portal_tick(&mut self, real_dt: f32) -> f32 {
        let Some(p) = &mut self.portal else { return 1.0 };
        p.t += real_dt;
        if p.t >= PORTAL_TIME {
            self.portal = None;
            return 1.0;
        }
        p.time_scale()
    }

    /// Игрок `i` сейчас «в портале» (не ездит, не стреляет, рисуется анимацией).
    pub(crate) fn in_portal(&self, i: usize) -> bool {
        self.portal.as_ref().is_some_and(|p| p.player == i)
    }

    /// Нарисовать порталы и «проваливающийся»/выезжающий танк.
    pub(crate) fn draw_portal_jump(&self) {
        let Some(p) = &self.portal else { return };
        let size = self.tank_size;
        let t = p.t;
        let open = p.open();
        draw_portal(p.from, size, open, t);
        draw_portal(p.to, size, open, t + 1.0);
        let k = p.k();
        // Танк уменьшается и крутится в первом портале, потом вырастает из второго.
        let (center, scale, spin) = if k < DIVE {
            (p.from, 1.0 - k / DIVE, k / DIVE)
        } else if k < EMERGE {
            (p.to, (k - DIVE) / (EMERGE - DIVE), 1.0 - (k - DIVE) / (EMERGE - DIVE))
        } else {
            (p.to, 1.0, 0.0)
        };
        if scale <= 0.02 {
            return;
        }
        let mut ghost = self.players[p.player].tank.clone();
        ghost.size = size * scale;
        ghost.pos = center - vec2(ghost.size, ghost.size) / 2.0;
        // Пока крутится — меняем направление по кругу.
        if spin > 0.05 {
            use crate::entities::direction::Direction;
            let dirs = [Direction::Up, Direction::Right, Direction::Down, Direction::Left];
            let start = dirs.iter().position(|d| *d == ghost.direction).unwrap_or(0);
            ghost.direction = dirs[(start + (spin * 6.0) as usize) % 4];
        }
        ghost.draw();
    }
}
