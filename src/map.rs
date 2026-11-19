use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Tile {
    Empty,
    Brick,
    Steel,
    Water,
    Eagle,
}

pub struct GameMap {
    pub tiles: Vec<Tile>,
    pub width: i32,
    pub height: i32,
    /// Позиция базы (левый верхний тайл 2×2). None — если базы нет.
    pub base_origin: Option<IVec2>,
    /// Разрушена ли база.
    pub base_destroyed: bool,
}

impl GameMap {
    pub fn new(width: i32, height: i32) -> Self {
        let mut tiles = vec![Tile::Empty; (width * height) as usize];

        // Кирпичный блок в центре (как было).
        for x in 5..8 {
            for y in 4..6 {
                tiles[(x + y * width) as usize] = Tile::Brick;
            }
        }

        // Водоём (как было).
        for x in 3..5 {
            for y in 8..10 {
                tiles[(x + y * width) as usize] = Tile::Water;
            }
        }

        // База 2×2 внизу по центру.
        let base_x = (width - 2) / 2;
        let base_y = height - 2;
        for dx in 0..2 {
            for dy in 0..2 {
                tiles[((base_x + dx) + (base_y + dy) * width) as usize] = Tile::Eagle;
            }
        }

        // Кирпичная защита вокруг базы (сверху и по бокам).
        // Сверху: y = base_y - 1, x = base_x - 1 .. base_x + 2 (4 тайла)
        let top_y = base_y - 1;
        for x in (base_x - 1)..=(base_x + 2) {
            if x >= 0 && x < width && top_y >= 0 {
                tiles[(x + top_y * width) as usize] = Tile::Brick;
            }
        }
        // Слева: x = base_x - 1, y = base_y .. base_y + 1
        for y in base_y..(base_y + 2) {
            let lx = base_x - 1;
            if lx >= 0 && y >= 0 && y < height {
                tiles[(lx + y * width) as usize] = Tile::Brick;
            }
        }
        // Справа: x = base_x + 2, y = base_y .. base_y + 1
        for y in base_y..(base_y + 2) {
            let rx = base_x + 2;
            if rx >= 0 && rx < width && y >= 0 && y < height {
                tiles[(rx + y * width) as usize] = Tile::Brick;
            }
        }

        Self {
            tiles,
            width,
            height,
            base_origin: Some(ivec2(base_x, base_y)),
            base_destroyed: false,
        }
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Tile {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return Tile::Empty;
        }
        self.tiles[(x + y * self.width) as usize]
    }

    pub fn is_blocked_for_tank(&self, x: i32, y: i32) -> bool {
        matches!(
            self.get_tile(x, y),
            Tile::Brick | Tile::Steel | Tile::Water | Tile::Eagle
        )
    }

    pub fn is_blocked_for_bullet(&self, x: i32, y: i32) -> bool {
        matches!(
            self.get_tile(x, y),
            Tile::Brick | Tile::Steel | Tile::Eagle
        )
    }

    /// Пытается повредить тайл в (x, y).
    /// Кирпич разрушается, сталь — только с pierce_steel, база — разрушается
    /// (и помечает base_destroyed). Вода не повреждается.
    /// Возвращает true, если тайл изменился.
    pub fn damage_at(&mut self, x: i32, y: i32, pierce_steel: bool) -> bool {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return false;
        }
        let idx = (x + y * self.width) as usize;
        match self.tiles[idx] {
            Tile::Brick => {
                self.tiles[idx] = Tile::Empty;
                true
            }
            Tile::Steel if pierce_steel => {
                self.tiles[idx] = Tile::Empty;
                true
            }
            Tile::Eagle => {
                // Разрушаем базу — все 4 тайла становятся пустыми.
                self.destroy_base();
                true
            }
            _ => false,
        }
    }

    fn destroy_base(&mut self) {
        if let Some(origin) = self.base_origin {
            for dx in 0..2 {
                for dy in 0..2 {
                    let x = origin.x + dx;
                    let y = origin.y + dy;
                    if x >= 0 && x < self.width && y >= 0 && y < self.height {
                        self.tiles[(x + y * self.width) as usize] = Tile::Empty;
                    }
                }
            }
        }
        self.base_destroyed = true;
    }

    pub fn draw(&self, tile_size: f32) {
        for y in 0..self.height {
            for x in 0..self.width {
                let tile = self.get_tile(x, y);
                let px = x as f32 * tile_size;
                let py = y as f32 * tile_size;

                match tile {
                    Tile::Empty => {}
                    Tile::Brick => {
                        draw_rectangle(px, py, tile_size, tile_size, ORANGE);
                        draw_rectangle_lines(px, py, tile_size, tile_size, 1.0, DARKBROWN);
                    }
                    Tile::Steel => {
                        draw_rectangle(px, py, tile_size, tile_size, GRAY);
                        draw_rectangle_lines(px, py, tile_size, tile_size, 1.0, DARKGRAY);
                    }
                    Tile::Water => {
                        draw_rectangle(px, py, tile_size, tile_size, BLUE);
                    }
                    Tile::Eagle => {
                        draw_rectangle(px, py, tile_size, tile_size, DARKPURPLE);
                        draw_rectangle_lines(px, py, tile_size, tile_size, 1.0, BLACK);
                    }
                }
            }
        }
    }
}