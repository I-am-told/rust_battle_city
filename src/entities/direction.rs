use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn vector(self) -> IVec2 {
        match self {
            Direction::Up    => ivec2( 0, -1),
            Direction::Down  => ivec2( 0,  1),
            Direction::Left  => ivec2(-1,  0),
            Direction::Right => ivec2( 1,  0),
        }
    }

    pub fn all() -> [Direction; 4] {
        [Direction::Up, Direction::Down, Direction::Left, Direction::Right]
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::Up    => Direction::Down,
            Direction::Down  => Direction::Up,
            Direction::Left  => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}