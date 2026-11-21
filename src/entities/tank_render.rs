use macroquad::prelude::*;
use crate::entities::direction::Direction;
use crate::entities::enemy_kind::EnemyKind;
use crate::entities::tank::{Tank, Team};

/// Рисует фигуры танка в «нормальных» координатах: танк смотрит вверх,
/// (0,0) — левый верхний угол, (1,1) — правый нижний. Для других
/// направлений фигуры поворачиваются на 90°/180°/270° вокруг центра —
/// прямоугольники при этом остаются прямоугольниками.
struct Pen {
    pos: Vec2,
    size: f32,
    dir: Direction,
}

impl Pen {
    /// Прямоугольник (x, y, w, h) в нормальных координатах «вверх».
    fn rect(&self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let (rx, ry, rw, rh) = match self.dir {
            Direction::Up => (x, y, w, h),
            Direction::Right => (1.0 - (y + h), x, h, w),
            Direction::Down => (1.0 - (x + w), 1.0 - (y + h), w, h),
            Direction::Left => (y, 1.0 - (x + w), h, w),
        };
        let s = self.size;
        draw_rectangle(self.pos.x + rx * s, self.pos.y + ry * s, rw * s, rh * s, color);
    }

    fn point(&self, x: f32, y: f32) -> Vec2 {
        let (px, py) = match self.dir {
            Direction::Up => (x, y),
            Direction::Right => (1.0 - y, x),
            Direction::Down => (1.0 - x, 1.0 - y),
            Direction::Left => (y, 1.0 - x),
        };
        self.pos + vec2(px, py) * self.size
    }

    fn circle(&self, x: f32, y: f32, r: f32, color: Color) {
        let p = self.point(x, y);
        draw_circle(p.x, p.y, r * self.size, color);
    }
}

fn shade(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

fn tint(c: Color, k: f32) -> Color {
    Color::new(c.r + (1.0 - c.r) * k, c.g + (1.0 - c.g) * k, c.b + (1.0 - c.b) * k, c.a)
}

/// Пропорции корпуса для разных типов танков.
struct Look {
    /// Отступ корпуса от краёв (между гусеницами).
    hull_inset: f32,
    turret_r: f32,
    barrel_w: f32,
    /// Где кончается ствол (0 — у самого края танка).
    barrel_tip: f32,
    /// Дополнительные бронеплиты по бокам корпуса.
    plates: bool,
    /// Скошенный нос (быстрый танк).
    wedge: bool,
    /// Дульный тормоз на конце ствола (мощный танк).
    muzzle: bool,
    /// Гоночные полосы на корпусе и выхлопные трубы (ранг 4 — быстрый).
    racer: bool,
}

fn look_for(tank: &Tank) -> Look {
    let base = Look {
        hull_inset: 0.2,
        turret_r: 0.19,
        barrel_w: 0.12,
        barrel_tip: 0.02,
        plates: false,
        wedge: false,
        muzzle: false,
        racer: false,
    };
    match tank.kind {
        Some(EnemyKind::Fast) => Look { hull_inset: 0.25, turret_r: 0.15, barrel_w: 0.09, wedge: true, ..base },
        Some(EnemyKind::Power) => Look { barrel_w: 0.15, barrel_tip: 0.0, muzzle: true, ..base },
        Some(EnemyKind::Armored) => Look { hull_inset: 0.17, turret_r: 0.23, barrel_w: 0.14, plates: true, ..base },
        // Игрок меняется с каждой звездой:
        // 0 — лёгкий танк;
        // 1 — длинный ствол с дульным тормозом (быстрые пули);
        // 2 — бронеплиты и башня крупнее (две пули);
        // 3 — тяжёлый: шире корпус, толстый ствол (пробивает сталь);
        // 4 — тяжёлый + гоночные полосы и выхлоп (быстрее ездит).
        None => match tank.rank {
            0 => base,
            1 => Look { barrel_w: 0.13, barrel_tip: 0.0, muzzle: true, ..base },
            2 => Look { barrel_w: 0.14, barrel_tip: 0.0, muzzle: true, plates: true, turret_r: 0.21, ..base },
            3 => Look {
                hull_inset: 0.16, turret_r: 0.23, barrel_w: 0.16, barrel_tip: 0.0,
                muzzle: true, plates: true, ..base
            },
            _ => Look {
                hull_inset: 0.16, turret_r: 0.23, barrel_w: 0.16, barrel_tip: 0.0,
                muzzle: true, plates: true, racer: true, ..base
            },
        },
        _ => base,
    }
}

impl Tank {
    pub fn draw(&self) {
        if !self.alive { return; }

        let color = match self.team {
            // Как в оригинале: P1 — жёлтый, P2 — зелёный.
            Team::Player => if self.player_slot == 0 { YELLOW } else { GREEN },
            Team::Enemy => {
                if self.frozen {
                    SKYBLUE
                } else if self.is_bonus {
                    // Бонусный мигает фиолетовым поверх базового цвета.
                    let phase = ((get_time() * 5.0) as i32) % 2;
                    if phase == 0 { PURPLE } else { self.base_enemy_color() }
                } else {
                    self.base_enemy_color()
                }
            }
        };

        self.draw_body(color);

        // Ранг 5+: стальные бронелисты (пока броня цела).
        if self.team == Team::Player && self.armor > 0 {
            self.draw_armor();
        }
        // Ранг 6: золотая рамка.
        if self.team == Team::Player && self.rank >= 6 {
            self.draw_gold_frame();
        }

        // Маркеры ранга у игрока.
        if self.team == Team::Player && self.rank > 0 {
            let n = self.rank as i32;
            let r = self.size * 0.06;
            let gap = self.size * 0.08;
            let total_w = n as f32 * (2.0 * r) + (n as f32 - 1.0) * gap;
            let start_x = self.pos.x + (self.size - total_w) / 2.0;
            let y = self.pos.y - r * 1.5;
            for i in 0..n {
                let cx = start_x + i as f32 * (2.0 * r + gap) + r;
                draw_circle(cx, y, r, WHITE);
            }
        }

        // Индикатор HP у врагов с max_hp > 1 (бронированный).
        if self.team == Team::Enemy && self.max_hp > 1 {
            let hp = self.hp.min(self.max_hp);
            let r = self.size * 0.06;
            let gap = self.size * 0.05;
            let total_w = hp as f32 * (2.0 * r) + (hp as f32 - 1.0) * gap;
            let start_x = self.pos.x + (self.size - total_w) / 2.0;
            let y = self.pos.y + self.size + r * 1.5;
            for i in 0..hp {
                let cx = start_x + i as f32 * (2.0 * r + gap) + r;
                draw_circle(cx, y, r, WHITE);
            }
        }

        if self.invulnerable {
            self.draw_shield();
        }
        if self.gassed {
            self.draw_gas();
        }
    }

    fn draw_body(&self, color: Color) {
        let pen = Pen { pos: self.pos, size: self.size, dir: self.direction };
        let look = look_for(self);
        let dark = shade(color, 0.45);
        let darker = shade(color, 0.3);
        let light = tint(color, 0.45);

        // Гусеницы с траками. Траки сдвигаются с пройденным путём.
        let track_w = 0.22;
        let phase = (self.odometer / self.size * 4.0).fract();
        for x in [0.0, 1.0 - track_w] {
            pen.rect(x, 0.04, track_w, 0.92, darker);
            let links = 7;
            for i in 0..=links {
                let y = 0.04 + (i as f32 + phase) / links as f32 * 0.92 - 0.06;
                let (y0, y1) = (y.max(0.04), (y + 0.06).min(0.96));
                if y1 > y0 {
                    pen.rect(x + 0.02, y0, track_w - 0.04, y1 - y0, dark);
                }
            }
        }

        // Корпус: основная заливка, светлая кромка сверху/слева, тень снизу.
        let hx = look.hull_inset;
        let hw = 1.0 - 2.0 * hx;
        let (hy, hh) = if look.wedge { (0.22, 0.66) } else { (0.16, 0.72) };
        pen.rect(hx, hy, hw, hh, color);
        pen.rect(hx, hy, hw, 0.05, light);
        pen.rect(hx, hy, 0.04, hh, light);
        pen.rect(hx, hy + hh - 0.07, hw, 0.07, dark);
        if look.wedge {
            // Скошенный нос: две ступеньки.
            pen.rect(hx + 0.06, hy - 0.06, hw - 0.12, 0.06, color);
            pen.rect(hx + 0.12, hy - 0.11, hw - 0.24, 0.05, light);
        }
        if look.plates {
            pen.rect(hx - 0.02, 0.3, 0.08, 0.4, light);
            pen.rect(1.0 - hx - 0.06, 0.3, 0.08, 0.4, light);
        }

        if look.racer {
            // Две гоночные полосы вдоль корпуса.
            let stripe = Color::new(0.85, 0.1, 0.1, 1.0);
            pen.rect(0.36, hy + 0.05, 0.07, hh - 0.12, stripe);
            pen.rect(0.57, hy + 0.05, 0.07, hh - 0.12, stripe);
            // Выхлопные трубы сзади; когда танк едет — пламя.
            pen.rect(hx + 0.06, 0.88, 0.1, 0.08, darker);
            pen.rect(1.0 - hx - 0.16, 0.88, 0.1, 0.08, darker);
            let moving = (self.pos - self.prev_pos).length_squared() > 0.0001;
            if moving && ((get_time() * 16.0) as i32) % 2 == 0 {
                let fire = Color::new(1.0, 0.6, 0.1, 0.9);
                pen.rect(hx + 0.07, 0.96, 0.08, 0.04, fire);
                pen.rect(1.0 - hx - 0.15, 0.96, 0.08, 0.04, fire);
            }
        }

        // Ствол (под башней, чтобы башня перекрыла основание).
        let bx = 0.5 - look.barrel_w / 2.0;
        pen.rect(bx, look.barrel_tip, look.barrel_w, 0.5 - look.barrel_tip, dark);
        pen.rect(bx + look.barrel_w * 0.25, look.barrel_tip, look.barrel_w * 0.3, 0.5 - look.barrel_tip, light);
        if look.muzzle {
            pen.rect(bx - 0.04, look.barrel_tip, look.barrel_w + 0.08, 0.08, dark);
        }

        // Башня: круг с бликом.
        pen.circle(0.5, 0.56, look.turret_r, dark);
        pen.circle(0.5, 0.56, look.turret_r * 0.82, color);
        pen.circle(0.46, 0.52, look.turret_r * 0.35, light);
    }

    /// Бронелисты: стальные экраны поверх гусениц и лобовая плита с заклёпками.
    fn draw_armor(&self) {
        let pen = Pen { pos: self.pos, size: self.size, dir: self.direction };
        let steel = Color::new(0.72, 0.74, 0.8, 1.0);
        let edge = Color::new(0.42, 0.44, 0.5, 1.0);
        let rivet = Color::new(0.95, 0.95, 1.0, 1.0);
        for x in [-0.03, 0.85] {
            pen.rect(x, 0.12, 0.18, 0.76, edge);
            pen.rect(x + 0.02, 0.14, 0.14, 0.72, steel);
            for y in [0.22, 0.5, 0.78] {
                pen.circle(x + 0.09, y, 0.025, rivet);
            }
        }
        // Лобовая плита (у основания ствола).
        pen.rect(0.26, 0.12, 0.48, 0.08, edge);
        pen.rect(0.28, 0.13, 0.44, 0.05, steel);
    }

    /// Золотая рамка 6-го ранга: переливается, с ромбиками по углам.
    fn draw_gold_frame(&self) {
        let t = get_time() as f32;
        let shine = 0.75 + 0.25 * (t * 6.0).sin();
        let gold = Color::new(1.0 * shine, 0.8 * shine, 0.15, 1.0);
        let light = Color::new(1.0, 0.97, 0.6, 1.0);
        let m = self.size * 0.06;
        let (x, y, s) = (self.pos.x - m, self.pos.y - m, self.size + 2.0 * m);
        draw_rectangle_lines(x, y, s, s, self.size * 0.07, gold);
        for (cx, cy) in [(x, y), (x + s, y), (x, y + s), (x + s, y + s)] {
            let r = self.size * 0.09;
            draw_triangle(vec2(cx - r, cy), vec2(cx, cy - r), vec2(cx + r, cy), light);
            draw_triangle(vec2(cx - r, cy), vec2(cx, cy + r), vec2(cx + r, cy), light);
        }
        // Бегущий блик по рамке.
        let k = (t * 0.8).fract() * 4.0;
        let (bx, by) = match k as i32 {
            0 => (x + s * k.fract(), y),
            1 => (x + s, y + s * k.fract()),
            2 => (x + s * (1.0 - k.fract()), y + s),
            _ => (x, y + s * (1.0 - k.fract())),
        };
        draw_circle(bx, by, self.size * 0.07, light);
    }

    /// Зелёная дымка вокруг врага, попавшего под облачко «мамы».
    fn draw_gas(&self) {
        let c = self.pos + vec2(self.size, self.size) / 2.0;
        let t = get_time() as f32;
        for i in 0..7 {
            let a = i as f32 * std::f32::consts::TAU / 7.0 + t * 0.7;
            let d = self.size * (0.32 + 0.06 * (t * 2.0 + i as f32).sin());
            let p = c + vec2(a.cos(), a.sin()) * d;
            draw_circle(p.x, p.y, self.size * 0.24, Color::new(0.55, 0.85, 0.45, 0.35));
        }
        draw_circle(c.x, c.y, self.size * 0.4, Color::new(0.8, 1.0, 0.75, 0.25));
    }

    /// Защитное поле после появления: мерцающее кольцо с искрами.
    fn draw_shield(&self) {
        let c = self.pos + vec2(self.size, self.size) / 2.0;
        let t = get_time() as f32;
        let flick = ((t * 20.0) as i32 % 2) as f32;
        let r = self.size * (0.62 + 0.04 * flick);
        let col = Color::new(1.0, 1.0, 1.0, 0.75);
        draw_circle_lines(c.x, c.y, r, self.size * 0.05, col);
        for i in 0..8 {
            let a = t * 6.0 + i as f32 * std::f32::consts::TAU / 8.0;
            let p0 = c + vec2(a.cos(), a.sin()) * r;
            let p1 = c + vec2(a.cos(), a.sin()) * (r + self.size * 0.1);
            draw_line(p0.x, p0.y, p1.x, p1.y, self.size * 0.04, col);
        }
    }

    fn base_enemy_color(&self) -> Color {
        match self.kind {
            Some(k) => enemy_color(k),
            None => RED,
        }
    }
}

/// Цвет врага по типу (и для значков на экране подсчёта).
pub fn enemy_color(kind: EnemyKind) -> Color {
    match kind {
        EnemyKind::Basic   => Color::new(0.85, 0.25, 0.2, 1.0),
        EnemyKind::Fast    => ORANGE,
        // Не зелёный — чтобы не путать с P2.
        EnemyKind::Power   => Color::new(0.95, 0.4, 0.7, 1.0),
        EnemyKind::Armored => Color::new(0.55, 0.55, 0.6, 1.0),
    }
}
