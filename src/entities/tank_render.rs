use macroquad::prelude::*;
use crate::entities::tank::{Tank, Team};

impl Tank {
    pub fn draw(&self) {
        if !self.alive { return; }

        // Мигание при неуязвимости.
        if self.invulnerable {
            let phase = ((get_time() * 10.0) as i32) % 2;
            if phase != 0 { return; }
        }

        // Цвет корпуса.
        let color = match self.team {
            Team::Player => GREEN,
            Team::Enemy => {
                if self.is_bonus {
                    let phase = ((get_time() * 5.0) as i32) % 2;
                    if phase == 0 { PURPLE } else { RED }
                } else {
                    RED
                }
            }
        };

        draw_rectangle(self.pos.x, self.pos.y, self.size, self.size, color);
        draw_rectangle_lines(self.pos.x, self.pos.y, self.size, self.size, 4.0, WHITE);

        // Ствол.
        let center = self.pos + vec2(self.size / 2.0, self.size / 2.0);
        let v = self.direction.vector();
        let tip = center + vec2(v.x as f32, v.y as f32) * (self.size / 2.0);
        draw_line(center.x, center.y, tip.x, tip.y, self.size * 0.2, WHITE);

        // Маркеры ранга у игрока: маленькие белые кружки вдоль верха танка.
        if self.team == Team::Player && self.rank > 0 {
            let n = self.rank as i32;
            let r = self.size * 0.07;
            let margin = self.size * 0.12;
            let gap = self.size * 0.10;
            let total_w = n as f32 * (2.0 * r) + (n as f32 - 1.0) * gap;
            let start_x = self.pos.x + (self.size - total_w) / 2.0;
            let y = self.pos.y + margin;

            for i in 0..n {
                let cx = start_x + i as f32 * (2.0 * r + gap) + r;
                draw_circle(cx, y, r, WHITE);
            }
        }
    }
}