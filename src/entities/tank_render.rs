use macroquad::prelude::*;
use crate::entities::direction::Direction;
use crate::entities::enemy_kind::EnemyKind;
use crate::entities::tank::{Tank, Team, MAX_RANK};

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
    /// Три ствола (ранг 7 — три пули).
    triple: bool,
    /// Ширина гусениц.
    track_w: f32,
    /// Форма башни.
    turret: Turret,
    /// Задний спойлер во всю ширину (быстрый враг).
    spoiler: bool,
    /// Ящик на корме (мощный враг).
    ammo_box: bool,
    /// Бортовые экраны поверх гусениц и лобовой отвал (бронированный враг).
    skirts: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Turret {
    Round,
    /// Квадратная, со скосами — мощный враг.
    Square,
    /// Широкая «шайба» с люком — бронированный враг.
    Dome,
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
        triple: false,
        track_w: 0.22,
        turret: Turret::Round,
        spoiler: false,
        ammo_box: false,
        skirts: false,
    };
    match tank.kind {
        // Враги различаются силуэтом, не только цветом:
        // обычный — классический танк с круглой башней;
        // быстрый — узкий, тонкие гусеницы, скошенный нос, спойлер сзади;
        // мощный — квадратная башня, короткий толстый ствол с тормозом, ящик на корме;
        // бронированный — экраны на гусеницах, отвал спереди, большая башня-шайба.
        Some(EnemyKind::Basic) => base,
        Some(EnemyKind::Fast) => Look {
            hull_inset: 0.27, turret_r: 0.13, barrel_w: 0.08, track_w: 0.16,
            wedge: true, spoiler: true, ..base
        },
        Some(EnemyKind::Power) => Look {
            turret_r: 0.21, barrel_w: 0.2, barrel_tip: 0.08, muzzle: true,
            turret: Turret::Square, ammo_box: true, ..base
        },
        Some(EnemyKind::Armored) => Look {
            hull_inset: 0.17, turret_r: 0.26, barrel_w: 0.13, track_w: 0.24,
            turret: Turret::Dome, skirts: true, ..base
        },
        // Игрок меняется с каждой звездой:
        // 0 — лёгкий танк;
        // 1 — длинный ствол с дульным тормозом (быстрые пули);
        // 2 — бронеплиты и башня крупнее (две пули);
        // 3 — тяжёлый: шире корпус, толстый ствол (пробивает сталь);
        // 4 — тяжёлый + гоночные полосы и выхлоп (быстрее ездит);
        // 5 — + бронелисты (рисуются отдельно); 6 — шестая точка ранга зелёная;
        // 7 — + три ствола (три пули); 8 (максимум) — + золотая отделка.
        None => match tank.rank {
            0 => base,
            1 => Look { barrel_w: 0.13, barrel_tip: 0.0, muzzle: true, ..base },
            2 => Look { barrel_w: 0.14, barrel_tip: 0.0, muzzle: true, plates: true, turret_r: 0.21, ..base },
            3 => Look {
                hull_inset: 0.16, turret_r: 0.23, barrel_w: 0.16, barrel_tip: 0.0,
                muzzle: true, plates: true, ..base
            },
            r => Look {
                hull_inset: 0.16, turret_r: 0.23, barrel_w: 0.16, barrel_tip: 0.0,
                muzzle: true, plates: true, racer: true, triple: r >= 7, ..base
            },
        },
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
        // Максимальный ранг: танк отделан золотом (стволы тоже золотые).
        let gold = self.team == Team::Player && self.rank >= MAX_RANK;
        if gold {
            self.draw_gold_trim();
        }
        // Ранг 7: боковые стволы — поверх бронелистов, чтобы их было видно.
        if look_for(self).triple {
            self.draw_side_barrels(if gold { Color::new(0.93, 0.68, 0.12, 1.0) } else { color });
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
                // Шестая точка (ранг с телепортом) — зелёная, как портал.
                let c = if i == 5 { Color::new(0.3, 1.0, 0.3, 1.0) } else { WHITE };
                draw_circle(cx, y, r, c);
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
        let track_w = look.track_w;
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

        if look.skirts {
            // Бортовые экраны с болтами — закрывают гусеницы почти целиком.
            let steel = shade(color, 0.75);
            for x in [0.0, 1.0 - track_w] {
                pen.rect(x, 0.1, track_w, 0.8, darker);
                pen.rect(x + 0.02, 0.12, track_w - 0.04, 0.76, steel);
                for y in [0.2, 0.4, 0.6, 0.8] {
                    pen.circle(x + track_w / 2.0, y, 0.025, light);
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
        if look.spoiler {
            // Спойлер на стойках поперёк кормы.
            pen.rect(0.36, 0.84, 0.06, 0.08, darker);
            pen.rect(0.58, 0.84, 0.06, 0.08, darker);
            pen.rect(0.04, 0.9, 0.92, 0.07, dark);
            pen.rect(0.04, 0.9, 0.92, 0.025, light);
        }
        if look.ammo_box {
            pen.rect(0.3, 0.8, 0.4, 0.14, darker);
            pen.rect(0.33, 0.82, 0.34, 0.1, dark);
            pen.rect(0.48, 0.82, 0.04, 0.1, darker);
        }
        if look.skirts {
            // Лобовой отвал во всю ширину.
            pen.rect(0.02, 0.06, 0.96, 0.09, darker);
            pen.rect(0.04, 0.07, 0.92, 0.04, light);
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

        // Башня.
        let r = look.turret_r;
        match look.turret {
            Turret::Round => {
                pen.circle(0.5, 0.56, r, dark);
                pen.circle(0.5, 0.56, r * 0.82, color);
                pen.circle(0.46, 0.52, r * 0.35, light);
            }
            Turret::Square => {
                // Квадрат со срезанными углами (крест из двух прямоугольников).
                let c = r * 0.3;
                pen.rect(0.5 - r, 0.56 - r + c, 2.0 * r, 2.0 * (r - c), dark);
                pen.rect(0.5 - r + c, 0.56 - r, 2.0 * (r - c), 2.0 * r, dark);
                let i = r * 0.8;
                pen.rect(0.5 - i, 0.56 - i, 2.0 * i, 2.0 * i, color);
                pen.rect(0.5 - i, 0.56 - i, 2.0 * i, 0.04, light);
                pen.rect(0.5 - i, 0.56 - i, 0.04, 2.0 * i, light);
            }
            Turret::Dome => {
                pen.circle(0.5, 0.56, r, darker);
                pen.circle(0.5, 0.56, r * 0.86, dark);
                pen.circle(0.5, 0.56, r * 0.7, color);
                // Люк командира.
                pen.circle(0.5, 0.6, r * 0.3, dark);
                pen.circle(0.5, 0.6, r * 0.2, light);
            }
        }
    }

    /// Два боковых ствола 7-го ранга (три пули) — по бокам от главного.
    fn draw_side_barrels(&self, color: Color) {
        let pen = Pen { pos: self.pos, size: self.size, dir: self.direction };
        let (dark, light) = (shade(color, 0.35), tint(color, 0.5));
        for cx in [0.31, 0.69] {
            let (w, x) = (0.1, cx - 0.05);
            pen.rect(x, 0.0, w, 0.44, dark);
            pen.rect(x + 0.03, 0.02, 0.035, 0.4, light);
            pen.rect(x - 0.02, 0.0, w + 0.04, 0.07, dark);
        }
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

    /// Золотая отделка максимального ранга: башня, ствол, кромки гусениц
    /// и заклёпки на корпусе — в золоте, а сам корпус остаётся цвета игрока.
    /// По танку раз в пару секунд пробегает блик.
    fn draw_gold_trim(&self) {
        let pen = Pen { pos: self.pos, size: self.size, dir: self.direction };
        let look = look_for(self);
        let t = get_time() as f32;
        let gold = Color::new(0.93, 0.68, 0.12, 1.0);
        let dark = Color::new(0.6, 0.4, 0.05, 1.0);
        let light = Color::new(1.0, 0.95, 0.6, 1.0);

        // Кромки гусениц.
        let tw = look.track_w;
        for x in [0.0, 1.0 - tw] {
            pen.rect(x, 0.04, tw, 0.035, gold);
            pen.rect(x, 0.925, tw, 0.035, gold);
        }
        // Заклёпки по углам корпуса.
        let hx = look.hull_inset;
        for (x, y) in [(hx + 0.06, 0.22), (1.0 - hx - 0.06, 0.22), (hx + 0.06, 0.82), (1.0 - hx - 0.06, 0.82)] {
            pen.circle(x, y, 0.035, dark);
            pen.circle(x - 0.008, y - 0.008, 0.022, light);
        }
        // Ствол.
        let bw = look.barrel_w;
        let bx = 0.5 - bw / 2.0;
        pen.rect(bx, look.barrel_tip, bw, 0.5 - look.barrel_tip, dark);
        pen.rect(bx + bw * 0.2, look.barrel_tip, bw * 0.6, 0.5 - look.barrel_tip, gold);
        pen.rect(bx + bw * 0.3, look.barrel_tip, bw * 0.2, 0.5 - look.barrel_tip, light);
        pen.rect(bx - 0.03, look.barrel_tip, bw + 0.06, 0.06, dark);
        pen.rect(bx - 0.02, look.barrel_tip + 0.01, bw + 0.04, 0.03, gold);
        // Башня с бегущим по кругу бликом.
        let r = look.turret_r;
        pen.circle(0.5, 0.56, r, dark);
        pen.circle(0.5, 0.56, r * 0.82, gold);
        let a = t * 2.5;
        pen.circle(0.5 + a.cos() * r * 0.45, 0.56 + a.sin() * r * 0.45, r * 0.25, light);
        pen.circle(0.5, 0.56, r * 0.3, dark);
        // Блик-звёздочка пробегает по диагонали танка.
        let k = (t * 0.5).fract();
        if k < 0.35 {
            let p = self.pos + vec2(k, k) / 0.35 * self.size;
            let s = self.size * 0.1 * (1.0 - (k / 0.35 - 0.5).abs() * 2.0).max(0.2);
            draw_line(p.x - s, p.y, p.x + s, p.y, s * 0.3, light);
            draw_line(p.x, p.y - s, p.x, p.y + s, s * 0.3, light);
        }
    }

    /// Дымка вокруг врага, которого «усыпил» помощник (цвет — помощника).
    fn draw_gas(&self) {
        let c = self.pos + vec2(self.size, self.size) / 2.0;
        let t = get_time() as f32;
        for i in 0..7 {
            let a = i as f32 * std::f32::consts::TAU / 7.0 + t * 0.7;
            let d = self.size * (0.32 + 0.06 * (t * 2.0 + i as f32).sin());
            let p = c + vec2(a.cos(), a.sin()) * d;
            draw_circle(p.x, p.y, self.size * 0.24, Color { a: 0.35, ..self.gas_tint });
        }
        let g = self.gas_tint;
        let light = Color::new(g.r + (1.0 - g.r) * 0.6, g.g + (1.0 - g.g) * 0.6, g.b + (1.0 - g.b) * 0.6, 0.25);
        draw_circle(c.x, c.y, self.size * 0.4, light);
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
