use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Tile {
    Empty,
    Brick,
    Steel,
    Water,
}

pub struct GameMap {
    pub tiles: Vec<Tile>,
    pub width: i32,
    pub height: i32,
}

impl GameMap {
    pub fn new(width: i32, height: i32) -> Self {
        let mut tiles = vec![Tile::Empty; (width * height) as usize];

        for x in 0..width {
            tiles[(x + 0 * width) as usize] = Tile::Steel;
            tiles[(x + (height - 1) * width) as usize] = Tile::Steel;
        }
        for y in 0..height {
            tiles[(0 + y * width) as usize] = Tile::Steel;
            tiles[((width - 1) + y * width) as usize] = Tile::Steel;
        }

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

        Self { tiles, width, height }
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Tile {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return Tile::Steel;
        }
        self.tiles[(x + y * self.width) as usize]
    }

    pub fn is_blocked(&self, x: i32, y: i32) -> bool {
        matches!(self.get_tile(x, y), Tile::Brick | Tile::Steel | Tile::Water)
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
                }
            }
        }
    }
}