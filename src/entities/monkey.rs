//! Бонус «мама»: прилетает помощник и стреляет во врагов — обезьянка
//! облачками-пуками, русалка водой, фея волшебными облачками, щенок лает
//! звуковой волной. Враг, в которого попал снаряд, навсегда замирает в
//! дымке (не ездит и не стреляет), пока его не уничтожат.
//! Кто прилетит — выбирается в меню (HELPER), по умолчанию случайно.

use macroquad::prelude::*;
use crate::sound::Sfx;
use std::f32::consts::TAU;

/// Кто из помощников прилетел.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HelperKind {
    Monkey,
    Mermaid,
    Fairy,
    Puppy,
}

impl HelperKind {
    pub const ALL: [HelperKind; 4] = [HelperKind::Monkey, HelperKind::Mermaid, HelperKind::Fairy, HelperKind::Puppy];

    pub fn label(self) -> &'static str {
        match self {
            HelperKind::Monkey => "MONKEY",
            HelperKind::Mermaid => "MERMAID",
            HelperKind::Fairy => "FAIRY",
            HelperKind::Puppy => "PUPPY",
        }
    }

    /// Цвет дымки вокруг «усыплённого» врага.
    pub fn haze(self) -> Color {
        match self {
            HelperKind::Monkey => Color::new(0.55, 0.85, 0.45, 1.0),
            HelperKind::Mermaid => Color::new(0.35, 0.65, 1.0, 1.0),
            HelperKind::Fairy => Color::new(0.95, 0.5, 0.95, 1.0),
            HelperKind::Puppy => Color::new(1.0, 0.85, 0.35, 1.0),
        }
    }

    /// Звук выстрела.
    pub fn sfx(self) -> Sfx {
        match self {
            HelperKind::Monkey => Sfx::Fart,
            HelperKind::Mermaid => Sfx::Splash,
            HelperKind::Fairy => Sfx::Magic,
            HelperKind::Puppy => Sfx::Bark,
        }
    }

    /// Размер снаряда (радиус) в долях размера помощника.
    fn shot_radius(self) -> f32 {
        match self {
            HelperKind::Monkey => 0.3,
            HelperKind::Mermaid => 0.2,
            HelperKind::Fairy => 0.28,
            HelperKind::Puppy => 0.34,
        }
    }
}

/// Снаряд помощника: летит сквозь стены и сам наводится на цель,
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
    /// Чей снаряд — от этого зависит рисунок и цвет дымки.
    pub kind: HelperKind,
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

    pub fn draw(&self) {
        if !self.alive {
            return;
        }
        match self.kind {
            HelperKind::Monkey => self.draw_puff(Color::new(0.55, 0.85, 0.45, 0.85), Color::new(0.9, 1.0, 0.85, 0.9)),
            HelperKind::Fairy => {
                self.draw_puff(Color::new(0.85, 0.5, 0.95, 0.8), Color::new(1.0, 0.85, 1.0, 0.9));
                // Искорки-звёздочки вокруг.
                for i in 0..5 {
                    let a = self.age * 3.0 + i as f32 * TAU / 5.0;
                    let p = self.pos + vec2(a.cos(), a.sin()) * self.radius * 1.15;
                    let tw = 0.5 + 0.5 * (self.age * 12.0 + i as f32 * 1.7).sin();
                    sparkle(p, self.radius * 0.3 * (0.6 + 0.4 * tw), Color::new(1.0, 1.0, 0.6, 0.6 + 0.4 * tw));
                }
            }
            HelperKind::Mermaid => self.draw_water(),
            HelperKind::Puppy => self.draw_bark(),
        }
    }

    /// Пушистое облачко: несколько кругов, чуть клубится.
    fn draw_puff(&self, body: Color, light: Color) {
        let t = self.age * 6.0;
        let r = self.radius;
        let puffs = [(0.0, 0.0, 0.75), (-0.45, 0.15, 0.55), (0.45, 0.1, 0.55), (0.1, -0.4, 0.5), (-0.2, 0.4, 0.45)];
        for (i, (dx, dy, k)) in puffs.iter().enumerate() {
            let wob = (t + i as f32).sin() * 0.08;
            let c = self.pos + vec2(dx + wob, dy - wob) * r;
            draw_circle(c.x, c.y, r * k, body);
            draw_circle(c.x - r * 0.12, c.y - r * 0.12, r * k * 0.6, light);
        }
    }

    /// Струя воды: капля с бликом и брызги позади.
    fn draw_water(&self) {
        let r = self.radius;
        let back = -self.vel.normalize_or(vec2(0.0, 1.0));
        let side = vec2(-back.y, back.x);
        for i in 1..=4 {
            let k = i as f32;
            let wob = (self.age * 20.0 + k).sin() * r * 0.3;
            let p = self.pos + back * r * 0.9 * k + side * wob;
            draw_circle(p.x, p.y, r * (0.55 - 0.1 * k), Color::new(0.45, 0.75, 1.0, 0.8 - 0.15 * k));
        }
        draw_circle(self.pos.x, self.pos.y, r, Color::new(0.2, 0.5, 0.95, 0.9));
        draw_circle(self.pos.x, self.pos.y, r * 0.75, Color::new(0.45, 0.75, 1.0, 0.95));
        draw_circle(self.pos.x - r * 0.3, self.pos.y - r * 0.3, r * 0.25, Color::new(1.0, 1.0, 1.0, 0.9));
    }

    /// «Гав!» — три дуги звуковой волны по направлению полёта, пульсируют.
    fn draw_bark(&self) {
        let r = self.radius;
        let dir = self.vel.normalize_or(vec2(0.0, -1.0));
        let ang = dir.y.atan2(dir.x).to_degrees();
        let back = self.pos - dir * r * 0.6;
        for i in 0..3 {
            let k = i as f32;
            let pulse = ((self.age * 10.0 - k).sin() * 0.5 + 0.5) * 0.4 + 0.6;
            let col = Color::new(1.0, 0.9 - 0.15 * k, 0.4, pulse);
            draw_arc(back.x, back.y, 24, r * (0.45 + 0.35 * k), ang - 45.0, r * 0.16, 90.0, col);
        }
    }
}

/// Звёздочка из четырёх лучей.
fn sparkle(p: Vec2, s: f32, col: Color) {
    draw_line(p.x - s, p.y, p.x + s, p.y, s * 0.35, col);
    draw_line(p.x, p.y - s, p.x, p.y + s, s * 0.35, col);
    draw_circle(p.x, p.y, s * 0.3, WHITE);
}

/// Помощник. Летает вокруг сына (оберегает его) и стреляет во врагов.
/// Название `Monkey` осталось с тех пор, как помощник был один.
pub struct Monkey {
    pub kind: HelperKind,
    /// Центр помощника.
    pub pos: Vec2,
    /// Размер (как танк).
    pub size: f32,
    /// Сколько ещё действует, секунд.
    pub time_left: f32,
    /// До следующего выстрела.
    pub cooldown: f32,
    /// Направление последнего выстрела — для рисунка.
    pub aim: Vec2,
    /// Сколько прошло с последнего выстрела (анимация).
    pub since_shot: f32,
    age: f32,
    /// Угол на круге вокруг охраняемого игрока, радиан.
    orbit: f32,
}

impl Monkey {
    pub fn new(kind: HelperKind, pos: Vec2, size: f32, time: f32) -> Self {
        Self { kind, pos, size, time_left: time, cooldown: 0.3, aim: vec2(0.0, -1.0), since_shot: 10.0, age: 0.0, orbit: std::f32::consts::PI }
    }

    /// Облёт вокруг `center` по кругу радиуса `radius` со скоростью `turns`
    /// оборотов в секунду. Помощник плавно догоняет свою точку на круге —
    /// если сын уехал или переместился, он «подлетает» к нему, а не прыгает.
    pub fn orbit_around(&mut self, center: Vec2, radius: f32, turns: f32, dt: f32) {
        self.orbit = (self.orbit + TAU * turns * dt) % TAU;
        let target = center + vec2(self.orbit.cos(), self.orbit.sin()) * radius;
        self.pos += (target - self.pos) * (dt * 6.0).min(1.0);
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

    /// Выстрел в сторону `target`: новый снаряд.
    pub fn shoot(&mut self, target: Vec2, target_id: u32, speed: f32, interval: f32) -> Cloud {
        let dir = (target - self.pos).normalize_or(vec2(0.0, -1.0));
        self.aim = dir;
        self.cooldown = interval;
        self.since_shot = 0.0;
        Cloud {
            pos: self.pos + dir * self.size * 0.45,
            vel: dir * speed,
            radius: self.size * self.kind.shot_radius(),
            age: 0.0,
            alive: true,
            target: Some(target_id),
            kind: self.kind,
        }
    }

    pub fn draw(&self) {
        // Подпрыгивает, а после выстрела — подскакивает выше.
        let s = self.size;
        let hop = (self.age * 8.0).sin().abs() * s * 0.05 + (1.0 - (self.since_shot * 4.0).min(1.0)) * s * 0.1;
        let c = self.pos - vec2(0.0, hop);
        match self.kind {
            HelperKind::Monkey => self.draw_monkey(c),
            HelperKind::Mermaid => self.draw_mermaid(c),
            HelperKind::Fairy => self.draw_fairy(c),
            HelperKind::Puppy => self.draw_puppy(c),
        }
    }

    /// Глазки и улыбка (общие для всех).
    fn face(&self, h: Vec2, line: Color) {
        let s = self.size;
        for dx in [-0.06, 0.06] {
            draw_circle(h.x + dx * s, h.y - s * 0.02, s * 0.03, WHITE);
            draw_circle(h.x + dx * s, h.y - s * 0.015, s * 0.017, BLACK);
        }
        draw_line(h.x - s * 0.06, h.y + s * 0.07, h.x, h.y + s * 0.09, s * 0.02, line);
        draw_line(h.x, h.y + s * 0.09, h.x + s * 0.06, h.y + s * 0.07, s * 0.02, line);
    }

    fn draw_monkey(&self, c: Vec2) {
        let s = self.size;
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
        self.face(h + vec2(0.0, s * 0.02), dark);
        // Лапы.
        draw_circle(c.x - s * 0.22, c.y + s * 0.38, s * 0.07, fur);
        draw_circle(c.x + s * 0.22, c.y + s * 0.38, s * 0.07, fur);
    }

    /// Русалка: рыжие волосы, ракушки, бирюзовый хвост машет плавником.
    fn draw_mermaid(&self, c: Vec2) {
        let s = self.size;
        let skin = Color::new(0.98, 0.82, 0.68, 1.0);
        let hair = Color::new(0.9, 0.3, 0.12, 1.0);
        let tail = Color::new(0.1, 0.7, 0.62, 1.0);
        let tail_light = Color::new(0.45, 0.9, 0.8, 1.0);
        let shell = Color::new(0.75, 0.4, 0.85, 1.0);
        let h = c - vec2(0.0, s * 0.2);

        // Волосы сзади — длинные, до пояса.
        for i in 0..5 {
            let k = i as f32 / 4.0;
            let sway = (self.age * 3.0 + k * 2.0).sin() * s * 0.02;
            for dx in [-1.0, 1.0] {
                draw_circle(h.x + dx * s * (0.14 + 0.03 * k) + sway, h.y + s * (0.02 + 0.12 * k), s * 0.08, hair);
            }
        }
        // Хвост: изгибается и машет.
        let wave = (self.age * 5.0).sin();
        for i in 0..7 {
            let k = i as f32 / 6.0;
            let p = c + vec2(wave * s * 0.12 * k * k, s * (0.08 + 0.36 * k));
            draw_circle(p.x, p.y, s * (0.15 - 0.08 * k), tail);
            draw_circle(p.x - s * 0.03, p.y, s * (0.07 - 0.04 * k), tail_light);
        }
        // Плавник.
        let end = c + vec2(wave * s * 0.12, s * 0.45);
        let tilt = wave * 0.3;
        for side in [-1.0f32, 1.0] {
            let tip = end + vec2(side * s * 0.17 + tilt * s * 0.1, s * 0.07);
            draw_triangle(end, tip, end + vec2(side * s * 0.05, s * 0.12), tail);
        }
        // Тело, ракушки, руки.
        draw_circle(c.x, c.y + s * 0.02, s * 0.14, skin);
        draw_circle(c.x - s * 0.06, c.y - s * 0.01, s * 0.05, shell);
        draw_circle(c.x + s * 0.06, c.y - s * 0.01, s * 0.05, shell);
        draw_circle(c.x - s * 0.17, c.y + s * 0.06, s * 0.05, skin);
        draw_circle(c.x + s * 0.17, c.y + s * 0.06, s * 0.05, skin);
        // Голова, чёлка, морская звезда в волосах.
        draw_circle(h.x, h.y, s * 0.15, skin);
        draw_circle(h.x - s * 0.07, h.y - s * 0.12, s * 0.08, hair);
        draw_circle(h.x + s * 0.05, h.y - s * 0.13, s * 0.08, hair);
        sparkle(h + vec2(s * 0.13, -s * 0.1), s * 0.05, Color::new(1.0, 0.75, 0.2, 1.0));
        self.face(h + vec2(0.0, s * 0.02), Color::new(0.7, 0.25, 0.3, 1.0));
    }

    /// Фея: машет прозрачными крылышками, розовое платье, палочка со звездой.
    fn draw_fairy(&self, c: Vec2) {
        let s = self.size;
        let skin = Color::new(0.98, 0.85, 0.72, 1.0);
        let hair = Color::new(1.0, 0.85, 0.3, 1.0);
        let dress = Color::new(1.0, 0.5, 0.75, 1.0);
        let dress_light = Color::new(1.0, 0.75, 0.88, 1.0);
        let wing = Color::new(0.7, 0.9, 1.0, 0.55);
        let h = c - vec2(0.0, s * 0.18);

        // Крылья — четыре овала, «машут» (сужаются и расширяются).
        let flap = 0.55 + 0.45 * (self.age * 16.0).sin().abs();
        for side in [-1.0f32, 1.0] {
            draw_ellipse(c.x + side * s * 0.2 * flap, c.y - s * 0.05, s * 0.2 * flap, s * 0.13, side * 25.0, wing);
            draw_ellipse(c.x + side * s * 0.15 * flap, c.y + s * 0.13, s * 0.13 * flap, s * 0.09, -side * 25.0, wing);
        }
        // Платье-колокольчик.
        draw_triangle(c + vec2(0.0, -s * 0.06), c + vec2(-s * 0.18, s * 0.3), c + vec2(s * 0.18, s * 0.3), dress);
        draw_triangle(c + vec2(0.0, s * 0.02), c + vec2(-s * 0.1, s * 0.3), c + vec2(s * 0.1, s * 0.3), dress_light);
        // Ножки.
        draw_circle(c.x - s * 0.06, c.y + s * 0.34, s * 0.035, skin);
        draw_circle(c.x + s * 0.06, c.y + s * 0.34, s * 0.035, skin);
        // Палочка — в сторону последнего выстрела, звезда мерцает.
        let hand = c + vec2(s * 0.12, s * 0.04);
        let tip = hand + self.aim * s * 0.28;
        draw_line(hand.x, hand.y, tip.x, tip.y, s * 0.025, Color::new(0.6, 0.4, 0.2, 1.0));
        let tw = if self.since_shot < 0.3 { 1.6 } else { 1.0 + 0.2 * (self.age * 10.0).sin() };
        sparkle(tip, s * 0.08 * tw, Color::new(1.0, 0.95, 0.4, 1.0));
        draw_circle(hand.x, hand.y, s * 0.04, skin);
        // Голова с пучком.
        draw_circle(h.x, h.y - s * 0.12, s * 0.07, hair);
        draw_circle(h.x, h.y, s * 0.14, hair);
        draw_circle(h.x, h.y + s * 0.02, s * 0.12, skin);
        self.face(h + vec2(0.0, s * 0.03), Color::new(0.8, 0.3, 0.4, 1.0));
    }

    /// Щенок: висячие уши, виляет хвостом, при «гав» открывает пасть.
    fn draw_puppy(&self, c: Vec2) {
        let s = self.size;
        let fur = Color::new(0.85, 0.6, 0.3, 1.0);
        let dark = Color::new(0.5, 0.3, 0.12, 1.0);
        let white = Color::new(0.98, 0.95, 0.88, 1.0);
        let h = c - vec2(0.0, s * 0.14);
        let barking = self.since_shot < 0.25;

        // Хвост виляет.
        let wag = (self.age * 18.0).sin() * 0.6;
        let tb = c + vec2(s * 0.2, s * 0.2);
        let tt = tb + vec2((0.6 + wag).cos(), -(0.6 + wag).sin().abs() - 0.3).normalize() * s * 0.2;
        draw_line(tb.x, tb.y, tt.x, tt.y, s * 0.07, fur);
        // Тело, лапы, грудка.
        draw_ellipse(c.x, c.y + s * 0.2, s * 0.24, s * 0.17, 0.0, fur);
        draw_circle(c.x, c.y + s * 0.18, s * 0.1, white);
        for dx in [-0.16, -0.06, 0.06, 0.16] {
            draw_circle(c.x + dx * s, c.y + s * 0.36, s * 0.05, white);
        }
        // Уши висят по бокам головы, подпрыгивают при лае.
        let ear_up = if barking { -s * 0.04 } else { 0.0 };
        for side in [-1.0f32, 1.0] {
            draw_ellipse(h.x + side * s * 0.17, h.y + s * 0.05 + ear_up, s * 0.07, s * 0.14, side * 15.0, dark);
        }
        // Голова и мордочка.
        draw_circle(h.x, h.y, s * 0.17, fur);
        draw_circle(h.x, h.y + s * 0.07, s * 0.09, white);
        for dx in [-0.06, 0.06] {
            draw_circle(h.x + dx * s, h.y - s * 0.04, s * 0.03, WHITE);
            draw_circle(h.x + dx * s, h.y - s * 0.035, s * 0.018, BLACK);
        }
        draw_circle(h.x, h.y + s * 0.03, s * 0.035, BLACK);
        if barking {
            // Открытая пасть: «Гав!»
            draw_ellipse(h.x, h.y + s * 0.11, s * 0.05, s * 0.04, 0.0, Color::new(0.55, 0.1, 0.15, 1.0));
        } else {
            // Язычок.
            draw_ellipse(h.x, h.y + s * 0.13, s * 0.025, s * 0.035, 0.0, Color::new(1.0, 0.45, 0.55, 1.0));
        }
    }
}
