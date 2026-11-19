use macroquad::prelude::*;

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
            BonusKind::Star    => "S",
            BonusKind::Life    => "L",
            BonusKind::Grenade => "G",
            BonusKind::Timer   => "T",
            BonusKind::Shovel  => "Sh",
        }
    }

    pub fn color(self) -> Color {
        match self {
            BonusKind::Star    => YELLOW,
            BonusKind::Life    => GREEN,
            BonusKind::Grenade => RED,
            BonusKind::Timer   => SKYBLUE,
            BonusKind::Shovel  => ORANGE,
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

    pub fn draw(&self) {
        if !self.alive { return; }

        // Мигание: 5 раз в секунду.
        let phase = ((get_time() * 5.0) as i32) % 2;
        if phase != 0 { return; }

        let c = self.kind.color();
        draw_rectangle(self.pos.x, self.pos.y, self.size, self.size, c);
        draw_rectangle_lines(self.pos.x, self.pos.y, self.size, self.size, 2.0, WHITE);

        // Символ по центру.
        let text = self.kind.symbol();
        let font_size = (self.size * 0.6) as u16;
        let dims = measure_text(text, None, font_size, 1.0);
        let tx = self.pos.x + self.size / 2.0 - dims.width / 2.0;
        let ty = self.pos.y + self.size / 2.0 + dims.height / 2.0;
        draw_text(text, tx, ty, font_size as f32, WHITE);
    }
}