use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BonusKind {
    /// Звезда — ранг танка +1.
    Star,
    /// Танк — +1 жизнь.
    Life,
    /// Граната — взрывает всех врагов на поле.
    Grenade,
    /// Часы — враги замирают.
    Timer,
    /// Лопата — сталь вокруг штаба.
    Shovel,
    /// Каска — защитное поле на время.
    Helmet,
}

impl BonusKind {
    pub fn random() -> Self {
        match rand::gen_range(0, 6) {
            0 => BonusKind::Star,
            1 => BonusKind::Life,
            2 => BonusKind::Grenade,
            3 => BonusKind::Timer,
            4 => BonusKind::Shovel,
            _ => BonusKind::Helmet,
        }
    }

    /// Цвет рамки значка.
    pub fn accent_color(self) -> Color {
        match self {
            BonusKind::Star    => YELLOW,
            BonusKind::Life    => GREEN,
            BonusKind::Grenade => RED,
            BonusKind::Timer   => SKYBLUE,
            BonusKind::Shovel  => ORANGE,
            BonusKind::Helmet  => WHITE,
        }
    }
}

pub struct Bonus {
    pub pos: Vec2,
    pub size: f32,
    pub kind: BonusKind,
    pub lifetime: f32,
    pub alive: bool,
}

impl Bonus {
    pub fn new(pos: Vec2, size: f32, lifetime: f32) -> Self {
        Self {
            pos,
            size,
            kind: BonusKind::random(),
            lifetime,
            alive: true,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.alive { return; }
        self.lifetime -= dt;
        if self.lifetime <= 0.0 {
            self.alive = false;
        }
    }

    pub fn try_pickup(&mut self, tank_pos: Vec2, tank_size: f32) -> bool {
        if !self.alive { return false; }
        if tank_pos.x < self.pos.x + self.size
            && tank_pos.x + tank_size > self.pos.x
            && tank_pos.y < self.pos.y + self.size
            && tank_pos.y + tank_size > self.pos.y
        {
            self.alive = false;
            true
        } else {
            false
        }
    }

    pub fn draw(&self) {
        if !self.alive { return; }

        // Мигание: 3 раза в секунду; за 3 секунды до исчезновения — чаще.
        let rate = if self.lifetime < 3.0 { 8.0 } else { 3.0 };
        if ((get_time() * rate) as i32) % 2 != 0 { return; }

        let (x, y, s) = (self.pos.x, self.pos.y, self.size);
        draw_rectangle(x, y, s, s, Color::new(0.05, 0.05, 0.1, 0.9));
        draw_rectangle_lines(x + 1.0, y + 1.0, s - 2.0, s - 2.0, s * 0.05, self.kind.accent_color());
        draw_bonus_icon(self.kind, x + s * 0.12, y + s * 0.12, s * 0.76);
    }
}

/// Картинка бонуса в квадрате (x, y, s).
pub fn draw_bonus_icon(kind: BonusKind, x: f32, y: f32, s: f32) {
    let p = |u: f32, v: f32| vec2(x + u * s, y + v * s);
    let white = Color::new(0.95, 0.95, 0.95, 1.0);
    let grey = Color::new(0.6, 0.6, 0.65, 1.0);
    let dark = Color::new(0.3, 0.3, 0.35, 1.0);
    match kind {
        BonusKind::Star => {
            // Пятиконечная звезда: центр + 5 треугольников-лучей.
            let c = p(0.5, 0.54);
            let (r_out, r_in) = (s * 0.48, s * 0.2);
            let pt = |i: i32, r: f32| {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                c + vec2(a.cos(), a.sin()) * r
            };
            for i in 0..5 {
                let tip = pt(i * 2, r_out);
                draw_triangle(pt(i * 2 - 1, r_in), pt(i * 2 + 1, r_in), tip, YELLOW);
                draw_triangle(c, pt(i * 2 - 1, r_in), pt(i * 2 + 1, r_in), YELLOW);
            }
            draw_circle(c.x - s * 0.06, c.y - s * 0.06, s * 0.06, Color::new(1.0, 1.0, 0.8, 1.0));
        }
        BonusKind::Life => {
            // Маленький танк.
            let g = GREEN;
            let gd = Color::new(0.0, 0.45, 0.15, 1.0);
            draw_rectangle(x + 0.08 * s, y + 0.2 * s, 0.2 * s, 0.72 * s, gd);
            draw_rectangle(x + 0.72 * s, y + 0.2 * s, 0.2 * s, 0.72 * s, gd);
            draw_rectangle(x + 0.24 * s, y + 0.3 * s, 0.52 * s, 0.56 * s, g);
            draw_rectangle(x + 0.45 * s, y + 0.02 * s, 0.1 * s, 0.42 * s, gd);
            draw_circle(x + 0.5 * s, y + 0.58 * s, 0.17 * s, gd);
            draw_circle(x + 0.5 * s, y + 0.58 * s, 0.13 * s, g);
        }
        BonusKind::Grenade => {
            // Граната: ребристый корпус, скоба и кольцо.
            let body = Color::new(0.35, 0.5, 0.25, 1.0);
            draw_circle(x + 0.5 * s, y + 0.62 * s, 0.33 * s, body);
            for k in [0.45, 0.62, 0.79] {
                draw_rectangle(x + 0.2 * s, y + k * s, 0.6 * s, 0.035 * s, Color::new(0.2, 0.3, 0.15, 1.0));
            }
            draw_rectangle(x + 0.4 * s, y + 0.18 * s, 0.2 * s, 0.14 * s, grey);
            draw_rectangle(x + 0.56 * s, y + 0.14 * s, 0.26 * s, 0.07 * s, grey);
            draw_circle_lines(x + 0.3 * s, y + 0.16 * s, 0.1 * s, s * 0.04, white);
        }
        BonusKind::Timer => {
            // Часы: циферблат со стрелками и кнопкой сверху.
            let c = p(0.5, 0.56);
            draw_rectangle(x + 0.42 * s, y + 0.02 * s, 0.16 * s, 0.1 * s, grey);
            draw_circle(c.x, c.y, 0.42 * s, grey);
            draw_circle(c.x, c.y, 0.34 * s, white);
            for i in 0..12 {
                let a = i as f32 * std::f32::consts::TAU / 12.0;
                let d = vec2(a.cos(), a.sin());
                let q = c + d * 0.29 * s;
                draw_circle(q.x, q.y, s * 0.02, dark);
            }
            draw_line(c.x, c.y, c.x, c.y - 0.24 * s, s * 0.05, BLACK);
            draw_line(c.x, c.y, c.x + 0.17 * s, c.y + 0.05 * s, s * 0.05, RED);
            draw_circle(c.x, c.y, s * 0.04, BLACK);
        }
        BonusKind::Shovel => {
            // Лопата: ручка, черенок и штык.
            let wood = Color::new(0.7, 0.45, 0.2, 1.0);
            draw_rectangle(x + 0.36 * s, y + 0.02 * s, 0.28 * s, 0.08 * s, wood);
            draw_rectangle(x + 0.45 * s, y + 0.02 * s, 0.1 * s, 0.5 * s, wood);
            draw_rectangle(x + 0.28 * s, y + 0.5 * s, 0.44 * s, 0.3 * s, grey);
            draw_triangle(p(0.28, 0.8), p(0.72, 0.8), p(0.5, 0.98), grey);
            draw_rectangle(x + 0.32 * s, y + 0.52 * s, 0.08 * s, 0.26 * s, white);
        }
        BonusKind::Helmet => {
            // Каска: купол, козырёк и звёздочка.
            let shell = Color::new(0.3, 0.55, 0.3, 1.0);
            draw_circle(x + 0.5 * s, y + 0.62 * s, 0.4 * s, shell);
            draw_rectangle(x + 0.08 * s, y + 0.62 * s, 0.84 * s, 0.36 * s, Color::new(0.05, 0.05, 0.1, 0.9));
            draw_rectangle(x + 0.04 * s, y + 0.6 * s, 0.92 * s, 0.1 * s, Color::new(0.2, 0.4, 0.2, 1.0));
            draw_circle(x + 0.36 * s, y + 0.4 * s, 0.07 * s, Color::new(0.5, 0.75, 0.5, 1.0));
            // Звёздочка на лбу.
            let c = p(0.5, 0.44);
            for i in 0..5 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 5.0;
                let tip = c + vec2(a.cos(), a.sin()) * 0.12 * s;
                draw_triangle(c, tip, c + vec2((a + 1.2).cos(), (a + 1.2).sin()) * 0.05 * s, RED);
                draw_triangle(c, tip, c + vec2((a - 1.2).cos(), (a - 1.2).sin()) * 0.05 * s, RED);
            }
        }
    }
}
