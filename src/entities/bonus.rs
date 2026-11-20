use macroquad::prelude::*;
use crate::ui_text::{draw_text, measure_text};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BonusKind {
    Star,
    Life,
    Grenade,
    Timer,
    Shovel,
}

impl BonusKind {
    pub fn random() -> Self {
        match rand::gen_range(0, 5) {
            0 => BonusKind::Star,
            1 => BonusKind::Life,
            2 => BonusKind::Grenade,
            3 => BonusKind::Timer,
            _ => BonusKind::Shovel,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            BonusKind::Star    => "*",
            BonusKind::Life    => "+",
            BonusKind::Grenade => "G",
            BonusKind::Timer   => "T",
            BonusKind::Shovel  => "Sh",
        }
    }

    /// Яркая рамка (акцент).
    pub fn accent_color(self) -> Color {
        match self {
            BonusKind::Star    => YELLOW,
            BonusKind::Life    => GREEN,
            BonusKind::Grenade => RED,
            BonusKind::Timer   => SKYBLUE,
            BonusKind::Shovel  => ORANGE,
        }
    }

    /// Тёмный фон (для контраста).
    pub fn background_color(self) -> Color {
        match self {
            BonusKind::Star    => Color::new(0.25, 0.20, 0.00, 1.0),
            BonusKind::Life    => Color::new(0.00, 0.20, 0.00, 1.0),
            BonusKind::Grenade => Color::new(0.25, 0.00, 0.00, 1.0),
            BonusKind::Timer   => Color::new(0.00, 0.10, 0.25, 1.0),
            BonusKind::Shovel  => Color::new(0.25, 0.12, 0.00, 1.0),
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

        // Мигание: 3 раза в секунду, "видимая" фаза длиннее, чтобы символ читался.
        let phase = ((get_time() * 3.0) as i32) % 2;
        if phase != 0 { return; }

        let bg = self.kind.background_color();
        let accent = self.kind.accent_color();

        // Фон.
        draw_rectangle(self.pos.x, self.pos.y, self.size, self.size, bg);
        // Толстая яркая рамка.
        draw_rectangle_lines(
            self.pos.x + 1.0,
            self.pos.y + 1.0,
            self.size - 2.0,
            self.size - 2.0,
            3.0,
            accent,
        );

        // Символ по центру, крупный и белый.
        let text = self.kind.symbol();
        let font_size = (self.size * 0.7) as u16;
        let dims = measure_text(text, None, font_size, 1.0);
        let tx = self.pos.x + self.size / 2.0 - dims.width / 2.0;
        let ty = self.pos.y + self.size / 2.0 + dims.height / 2.0;
        draw_text(text, tx, ty, font_size as f32, WHITE);
    }
}