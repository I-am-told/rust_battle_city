//! Бонус «мама»: на поле появляется обезьянка и пукает облачками в
//! врагов. Враг, в которого попало облачко, навсегда замирает в зелёной
//! дымке (не ездит и не стреляет), пока его не уничтожат.

use macroquad::prelude::*;

/// Облачко: летит сквозь стены (это же газ) и само наводится на цель,
/// чтобы не промахнуться по едущему врагу.
pub struct Cloud {
    pub pos: Vec2,
    pub vel: Vec2,
    /// Радиус в пикселях.
    pub radius: f32,
    pub age: f32,
    pub alive: bool,
    /// За кем летит (id врага).
    pub target: Option<u32>,
}

impl Cloud {
    /// Сколько облачко живёт, если ни в кого не попало.
    pub const MAX_AGE: f32 = 6.0;

    /// Повернуть полёт на точку `to` (скорость та же).
    pub fn steer(&mut self, to: Vec2) {
        let speed = self.vel.length();
        self.vel = (to - self.pos).normalize_or(self.vel / speed.max(0.001)) * speed;
    }

    pub fn update(&mut self, dt: f32) {
        self.pos += self.vel * dt;
        self.age += dt;
        if self.age > Self::MAX_AGE {
            self.alive = false;
        }
    }

    /// Задевает ли облачко прямоугольник (танк).
    pub fn touches(&self, rect_pos: Vec2, rect_size: f32) -> bool {
        let nearest = vec2(
            self.pos.x.clamp(rect_pos.x, rect_pos.x + rect_size),
            self.pos.y.clamp(rect_pos.y, rect_pos.y + rect_size),
        );
        (nearest - self.pos).length() < self.radius
    }

    /// Пушистое облачко: несколько кругов, бело-зелёное, чуть клубится.
    pub fn draw(&self) {
        if !self.alive {
            return;
        }
        let t = self.age * 6.0;
        let r = self.radius;
        let puffs = [(0.0, 0.0, 0.75), (-0.45, 0.15, 0.55), (0.45, 0.1, 0.55), (0.1, -0.4, 0.5), (-0.2, 0.4, 0.45)];
        for (i, (dx, dy, k)) in puffs.iter().enumerate() {
            let wob = (t + i as f32).sin() * 0.08;
            let c = self.pos + vec2(dx + wob, dy - wob) * r;
            draw_circle(c.x, c.y, r * k, Color::new(0.55, 0.85, 0.45, 0.85));
            draw_circle(c.x - r * 0.12, c.y - r * 0.12, r * k * 0.6, Color::new(0.9, 1.0, 0.85, 0.9));
        }
    }
}

/// Обезьянка. Стоит рядом с точкой появления P1 и стреляет облачками.
pub struct Monkey {
    /// Центр обезьянки.
    pub pos: Vec2,
    /// Размер (как танк).
    pub size: f32,
    /// Сколько ещё действует, секунд.
    pub time_left: f32,
    /// До следующего облачка.
    pub cooldown: f32,
    /// Куда смотрит попа (направление последнего выстрела) — для рисунка.
    pub aim: Vec2,
    /// Сколько прошло с последнего выстрела (анимация «пука»).
    pub since_shot: f32,
    age: f32,
}

impl Monkey {
    pub fn new(pos: Vec2, size: f32, time: f32) -> Self {
        Self { pos, size, time_left: time, cooldown: 0.3, aim: vec2(0.0, -1.0), since_shot: 10.0, age: 0.0 }
    }

    pub fn tick(&mut self, dt: f32) {
        self.time_left -= dt;
        self.cooldown -= dt;
        self.since_shot += dt;
        self.age += dt;
    }

    pub fn alive(&self) -> bool {
        self.time_left > 0.0
    }

    /// Пукнуть в сторону `target`: новое облачко.
    pub fn shoot(&mut self, target: Vec2, target_id: u32, speed: f32, interval: f32) -> Cloud {
        let dir = (target - self.pos).normalize_or(vec2(0.0, -1.0));
        self.aim = dir;
        self.cooldown = interval;
        self.since_shot = 0.0;
        Cloud {
            pos: self.pos + dir * self.size * 0.45,
            vel: dir * speed,
            radius: self.size * 0.3,
            age: 0.0,
            alive: true,
            target: Some(target_id),
        }
    }

    pub fn draw(&self) {
        let s = self.size;
        // Подпрыгивает, а после «пука» — подскакивает выше.
        let hop = (self.age * 8.0).sin().abs() * s * 0.05 + (1.0 - (self.since_shot * 4.0).min(1.0)) * s * 0.1;
        let c = self.pos - vec2(0.0, hop);
        let fur = Color::new(0.55, 0.33, 0.15, 1.0);
        let dark = Color::new(0.38, 0.22, 0.1, 1.0);
        let skin = Color::new(0.96, 0.8, 0.62, 1.0);

        // Хвост закорючкой — с противоположной от выстрела стороны.
        let back = self.aim;
        for i in 0..6 {
            let k = i as f32 / 5.0;
            let a = k * 3.5;
            let p = c + back * s * (0.3 + 0.15 * k) + vec2(a.cos(), a.sin()) * s * 0.12 * k;
            draw_circle(p.x, p.y, s * 0.05, dark);
        }
        // Тело и живот.
        draw_circle(c.x, c.y + s * 0.15, s * 0.28, fur);
        draw_circle(c.x, c.y + s * 0.18, s * 0.17, skin);
        // Голова, уши, мордочка.
        let h = c - vec2(0.0, s * 0.2);
        draw_circle(h.x - s * 0.22, h.y, s * 0.1, fur);
        draw_circle(h.x + s * 0.22, h.y, s * 0.1, fur);
        draw_circle(h.x - s * 0.22, h.y, s * 0.06, skin);
        draw_circle(h.x + s * 0.22, h.y, s * 0.06, skin);
        draw_circle(h.x, h.y, s * 0.2, fur);
        draw_circle(h.x, h.y + s * 0.04, s * 0.14, skin);
        // Глаза и улыбка.
        for dx in [-0.06, 0.06] {
            draw_circle(h.x + dx * s, h.y - s * 0.02, s * 0.03, WHITE);
            draw_circle(h.x + dx * s, h.y - s * 0.015, s * 0.017, BLACK);
        }
        draw_line(h.x - s * 0.06, h.y + s * 0.09, h.x, h.y + s * 0.11, s * 0.02, dark);
        draw_line(h.x, h.y + s * 0.11, h.x + s * 0.06, h.y + s * 0.09, s * 0.02, dark);
        // Лапы.
        draw_circle(c.x - s * 0.22, c.y + s * 0.38, s * 0.07, fur);
        draw_circle(c.x + s * 0.22, c.y + s * 0.38, s * 0.07, fur);
    }
}
