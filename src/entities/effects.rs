//! Взрывы (как на Dandy): маленький — когда пуля во что-то попала,
//! большой — когда уничтожен танк или штаб.

use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EffectKind {
    /// Попадание пули: ~0.2 с, размером с танк.
    Small,
    /// Гибель танка или штаба: ~0.5 с, в два раза больше танка.
    Big,
}

pub struct Effect {
    /// Центр взрыва.
    pub center: Vec2,
    pub kind: EffectKind,
    /// Сколько прошло, секунд.
    t: f32,
    duration: f32,
    /// Базовый размер (диаметр в пикселях) — размер танка.
    size: f32,
    /// Для «случайного» рисунка осколков: у каждого взрыва свой.
    seed: u32,
}

impl Effect {
    pub fn new(center: Vec2, kind: EffectKind, tank_size: f32) -> Self {
        let duration = match kind {
            EffectKind::Small => 0.2,
            EffectKind::Big => 0.5,
        };
        Self {
            center,
            kind,
            t: 0.0,
            duration,
            size: tank_size,
            seed: rand::gen_range(0u32, 1_000_000),
        }
    }

    /// Возвращает false, когда взрыв закончился.
    pub fn update(&mut self, dt: f32) -> bool {
        self.t += dt;
        self.t < self.duration
    }

    pub fn alive(&self) -> bool {
        self.t < self.duration
    }

    pub fn draw(&self) {
        if !self.alive() {
            return;
        }
        let k = (self.t / self.duration).clamp(0.0, 1.0);
        match self.kind {
            EffectKind::Small => self.draw_small(k),
            EffectKind::Big => self.draw_big(k),
        }
    }

    /// Псевдослучайное число 0..1 для i-го осколка (одинаковое каждый кадр).
    fn noise(&self, i: u32) -> f32 {
        let mut x = self.seed.wrapping_mul(747_796_405).wrapping_add(i.wrapping_mul(2_891_336_453));
        x ^= x >> 16;
        x = x.wrapping_mul(0x045d_9f3b);
        x ^= x >> 16;
        (x % 10_000) as f32 / 10_000.0
    }

    /// Огненный шар: слои от тёмно-красного снаружи до белого в центре.
    fn fireball(&self, r: f32, alpha: f32) {
        let c = self.center;
        let layers = [
            (1.00, Color::new(0.75, 0.10, 0.05, alpha)),
            (0.78, Color::new(1.00, 0.45, 0.05, alpha)),
            (0.55, Color::new(1.00, 0.85, 0.20, alpha)),
            (0.30, Color::new(1.00, 1.00, 0.90, alpha)),
        ];
        for (f, color) in layers {
            draw_circle(c.x, c.y, r * f, color);
        }
    }

    fn draw_small(&self, k: f32) {
        // Быстро раздувается, в конце гаснет.
        let grow = (k * 3.0).min(1.0);
        let r = self.size * 0.5 * (0.35 + 0.65 * grow);
        let alpha = if k > 0.7 { 1.0 - (k - 0.7) / 0.3 } else { 1.0 };
        self.fireball(r, alpha);
        // Несколько искр.
        let c = self.center;
        for i in 0..6 {
            let a = self.noise(i) * std::f32::consts::TAU;
            let len = r * (1.1 + 0.5 * self.noise(i + 10));
            let p = c + vec2(a.cos(), a.sin()) * len;
            draw_circle(p.x, p.y, self.size * 0.05, Color::new(1.0, 0.9, 0.4, alpha));
        }
    }

    fn draw_big(&self, k: f32) {
        let c = self.center;
        // Растёт до 0.6, потом сжимается и гаснет.
        let grow = if k < 0.6 { k / 0.6 } else { 1.0 - (k - 0.6) / 0.4 * 0.4 };
        let r = self.size * (0.3 + 0.7 * grow);
        let alpha = if k > 0.75 { 1.0 - (k - 0.75) / 0.25 } else { 1.0 };

        // Лучи.
        let rays = 10;
        for i in 0..rays {
            let a = (i as f32 + self.noise(i) * 0.6) / rays as f32 * std::f32::consts::TAU;
            let len = r * (1.15 + 0.45 * self.noise(i + 20));
            let tip = c + vec2(a.cos(), a.sin()) * len;
            draw_line(c.x, c.y, tip.x, tip.y, self.size * 0.12, Color::new(1.0, 0.55, 0.1, alpha));
        }

        // Несколько «клубов» вокруг центра — взрыв неровный.
        for i in 0..5 {
            let a = self.noise(i + 40) * std::f32::consts::TAU;
            let d = r * 0.45 * self.noise(i + 50);
            let p = c + vec2(a.cos(), a.sin()) * d;
            draw_circle(p.x, p.y, r * 0.55, Color::new(0.85, 0.2, 0.05, alpha));
        }
        self.fireball(r * 0.85, alpha);

        // Осколки разлетаются.
        for i in 0..12 {
            let a = self.noise(i + 60) * std::f32::consts::TAU;
            let d = r * (0.6 + 1.0 * k) * (0.8 + 0.4 * self.noise(i + 70));
            let p = c + vec2(a.cos(), a.sin()) * d;
            let s = self.size * 0.07;
            draw_rectangle(p.x - s / 2.0, p.y - s / 2.0, s, s, Color::new(1.0, 0.95, 0.6, alpha));
        }
    }
}

/// Сколько секунд мерцает звёздочка перед появлением танка.
pub const SPAWN_STAR_TIME: f32 = 1.0;

/// Мерцающая звёздочка появления (как на Dandy): четырёхлучевая звезда,
/// которая пульсирует — сжимается и разгорается — пока не появится танк.
/// `center` — центр точки появления, `size` — размер танка,
/// `elapsed` — сколько секунд звезда уже горит.
pub fn draw_spawn_star(center: Vec2, size: f32, elapsed: f32) {
    // 4 пульса за время звезды.
    let pulse = (elapsed / SPAWN_STAR_TIME * 4.0 * std::f32::consts::TAU).sin() * 0.5 + 0.5;
    let r = size * (0.18 + 0.32 * pulse);
    let thin = r * 0.22;
    let c = center;
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    let glow = Color::new(1.0, 0.95, 0.6, 0.9);

    // Длинные лучи (вверх-вниз-влево-вправо).
    for (dx, dy) in [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
        let tip = c + vec2(dx, dy) * r;
        let side = vec2(-dy, dx) * thin;
        draw_triangle(c + side, c - side, tip, glow);
    }
    // Короткие диагональные лучи.
    let d = r * 0.55;
    for (dx, dy) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let dir = vec2(dx, dy).normalize();
        let tip = c + dir * d;
        let side = vec2(-dir.y, dir.x) * thin * 0.7;
        draw_triangle(c + side, c - side, tip, white);
    }
    draw_circle(c.x, c.y, thin * 1.3, white);
}
