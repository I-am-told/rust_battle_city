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
    pub base_origin: Option<IVec2>,
    pub base_destroyed: bool,
}

impl GameMap {
    pub fn new(width: i32, height: i32) -> Self {
        let mut tiles = vec![Tile::Empty; (width * height) as usize];

        for x in 5..8 {
            for y in 4..6 {
                tiles[(x + y * width) as usize] = Tile::Brick;
            }
        }

        for x in 3..5 {
            for y in 8..10 {
                tiles[(x + y * width) as usize] = Tile::Water;
            }
        }

        // Тестовые стальные блоки.
        for x in 8..10 {
            for y in 2..4 {
                tiles[(x + y * width) as usize] = Tile::Steel;
            }
        }

        let base_x = (width - 2) / 2;
        let base_y = height - 2;
        for dx in 0..2 {
            for dy in 0..2 {
                tiles[((base_x + dx) + (base_y + dy) * width) as usize] = Tile::Eagle;
            }
        }

        let top_y = base_y - 1;
        for x in (base_x - 1)..=(base_x + 2) {
            if x >= 0 && x < width && top_y >= 0 {
                tiles[(x + top_y * width) as usize] = Tile::Brick;
            }
        }
        for y in base_y..(base_y + 2) {
            let lx = base_x - 1;
            if lx >= 0 && y >= 0 && y < height {
                tiles[(lx + y * width) as usize] = Tile::Brick;
            }
        }
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

    /// Временная стальная стена вокруг базы (эффект "лопата").
    /// Возвращает список тайлов, где была поставлена сталь.
    pub fn apply_shovel(&mut self) -> Vec<IVec2> {
        let mut tiles_changed: Vec<IVec2> = Vec::new();
        let base = match self.base_origin {
            Some(b) => b,
            None => return tiles_changed,
        };
        let base_x = base.x;
        let base_y = base.y;

        let mut set_tile = |map: &mut GameMap, x: i32, y: i32, out: &mut Vec<IVec2>| {
            if x < 0 || x >= map.width || y < 0 || y >= map.height {
                return;
            }
            map.tiles[(x + y * map.width) as usize] = Tile::Steel;
            out.push(ivec2(x, y));
        };

        let top_y = base_y - 1;
        for x in (base_x - 1)..=(base_x + 2) {
            set_tile(self, x, top_y, &mut tiles_changed);
        }
        for y in base_y..(base_y + 2) {
            set_tile(self, base_x - 1, y, &mut tiles_changed);
        }
        for y in base_y..(base_y + 2) {
            set_tile(self, base_x + 2, y, &mut tiles_changed);
        }

        tiles_changed
    }

    /// Возвращает кирпичи на места, где была сталь после лопаты.
    /// Только если там всё ещё Steel (если сталь разрушили — оставляем как есть).
    /// Возвращает кирпичи на места, где была сталь после лопаты.
    /// Восстанавливает стену ПОЛНОСТЬЮ, даже если сталь была разрушена —
    /// это соответствует поведению лопаты в оригинале ("чинит" стену).
    pub fn remove_shovel(&mut self, tiles: &[IVec2]) {
        for t in tiles {
            if t.x < 0 || t.x >= self.width || t.y < 0 || t.y >= self.height {
                continue;
            }
            self.tiles[(t.x + t.y * self.width) as usize] = Tile::Brick;
        }
    }

    pub fn random_bonus_spawn(&self, forbidden: &[IVec2]) -> Option<IVec2> {
        let mut candidates: Vec<IVec2> = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get_tile(x, y) != Tile::Empty {
                    continue;
                }
                let p = ivec2(x, y);
                if forbidden.iter().any(|f| *f == p) {
                    continue;
                }
                candidates.push(p);
            }
        }
        if candidates.is_empty() {
            return None;
        }
        let idx = rand::gen_range(0, candidates.len());
        Some(candidates[idx])
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