use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tile {
    Empty,
    Brick,
    Steel,
    Water,
    Eagle,
}

/// Все 4 четвертинки кирпичного тайла на месте.
pub const BRICK_FULL: u8 = 0b1111;

/// Карта. Тайл = половина танка. Кирпичный тайл дополнительно делится
/// на 4 четвертинки (как на Dandy): пуля выбивает их полосой.
pub struct GameMap {
    pub tiles: Vec<Tile>,
    /// Для кирпичных тайлов — какие четвертинки целы (биты:
    /// 0 — левая верхняя, 1 — правая верхняя, 2 — левая нижняя,
    /// 3 — правая нижняя). Для остальных тайлов не используется.
    /// Менять тайлы только через `set_tile`, чтобы маска не разъехалась.
    pub brick_mask: Vec<u8>,
    pub width: i32,
    pub height: i32,
    pub base_origin: Option<IVec2>,
    pub base_destroyed: bool,
}

impl GameMap {
    pub fn new(width: i32, height: i32) -> Self {
        let mut tiles = vec![Tile::Empty; (width * height) as usize];

        // Запись тайла с проверкой границ: тестовая раскладка рассчитана
        // на 26x26, при другом размере поля лишнее просто отбрасывается.
        let mut fill = |x0: i32, y0: i32, w: i32, h: i32, tile: Tile| {
            for y in y0..y0 + h {
                for x in x0..x0 + w {
                    if x >= 0 && x < width && y >= 0 && y < height {
                        tiles[(x + y * width) as usize] = tile;
                    }
                }
            }
        };

        // Тестовая раскладка (координаты в тайлах, танк = 2x2).
        fill(10, 8, 6, 4, Tile::Brick);
        fill(6, 16, 4, 4, Tile::Water);
        fill(16, 4, 4, 4, Tile::Steel);

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

        let brick_mask = tiles
            .iter()
            .map(|t| if *t == Tile::Brick { BRICK_FULL } else { 0 })
            .collect();

        Self {
            tiles,
            brick_mask,
            width,
            height,
            base_origin: Some(ivec2(base_x, base_y)),
            base_destroyed: false,
        }
    }

    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height
    }

    /// Поставить тайл (кирпич — целым). За пределами поля — ничего.
    pub fn set_tile(&mut self, x: i32, y: i32, tile: Tile) {
        if !self.in_bounds(x, y) {
            return;
        }
        let idx = (x + y * self.width) as usize;
        self.tiles[idx] = tile;
        self.brick_mask[idx] = if tile == Tile::Brick { BRICK_FULL } else { 0 };
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Tile {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return Tile::Empty;
        }
        self.tiles[(x + y * self.width) as usize]
    }

    /// Твёрдая ли четвертинка (qx, qy) — координаты в половинах тайла.
    /// За пределами поля — нет (границы поля проверяются отдельно).
    fn quarter_solid(&self, qx: i32, qy: i32, for_bullet: bool) -> bool {
        let (tx, ty) = (qx.div_euclid(2), qy.div_euclid(2));
        if !self.in_bounds(tx, ty) {
            return false;
        }
        let idx = (tx + ty * self.width) as usize;
        match self.tiles[idx] {
            Tile::Empty => false,
            Tile::Brick => {
                let bit = qx.rem_euclid(2) + 2 * qy.rem_euclid(2);
                self.brick_mask[idx] & (1 << bit) != 0
            }
            Tile::Steel | Tile::Eagle => true,
            // По воде танки не ездят, а пули над ней летят.
            Tile::Water => !for_bullet,
        }
    }

    /// Диапазон четвертинок, которые задевает прямоугольник.
    fn quarter_range(pos: Vec2, size: Vec2, tile_size: f32) -> (i32, i32, i32, i32) {
        let q = tile_size / 2.0;
        let eps = 0.001;
        (
            (pos.x / q).floor() as i32,
            ((pos.x + size.x - eps) / q).floor() as i32,
            (pos.y / q).floor() as i32,
            ((pos.y + size.y - eps) / q).floor() as i32,
        )
    }

    /// Задевает ли прямоугольник (пиксели) что-то твёрдое.
    /// Танкам мешают кирпич, сталь, вода и штаб; пулям — всё, кроме воды.
    /// Кирпич проверяется по четвертинкам: в пробитую дыру можно проехать.
    pub fn rect_blocked(&self, pos: Vec2, size: Vec2, tile_size: f32, for_bullet: bool) -> bool {
        let (x0, x1, y0, y1) = Self::quarter_range(pos, size, tile_size);
        (y0..=y1).any(|qy| (x0..=x1).any(|qx| self.quarter_solid(qx, qy, for_bullet)))
    }

    /// Попадание пули в препятствие (пуля стоит в `pos` и уже что-то задела).
    ///
    /// Как на Dandy: пуля выбивает в кирпиче полосу шириной в танк
    /// (4 четвертинки поперёк полёта) и глубиной в одну четвертинку —
    /// первый ряд, который она задела. Пуля «трёх звёзд» (`pierce`) бьёт
    /// на целый тайл в глубину и так же полосой разрушает сталь.
    /// Попадание в штаб — штаб уничтожен.
    pub fn bullet_hit(&mut self, pos: Vec2, size: f32, dir: IVec2, tile_size: f32, pierce: bool) {
        let q = tile_size / 2.0;
        let (x0, x1, y0, y1) = Self::quarter_range(pos, vec2(size, size), tile_size);

        // Штаб: любое касание уничтожает его.
        for qy in y0..=y1 {
            for qx in x0..=x1 {
                if self.get_tile(qx.div_euclid(2), qy.div_euclid(2)) == Tile::Eagle {
                    self.destroy_base();
                    return;
                }
            }
        }

        // Первый задетый ряд четвертинок по ходу полёта.
        let solid: Vec<IVec2> = (y0..=y1)
            .flat_map(|qy| (x0..=x1).map(move |qx| ivec2(qx, qy)))
            .filter(|c| self.quarter_solid(c.x, c.y, true))
            .collect();
        if solid.is_empty() {
            return;
        }
        let vertical = dir.y != 0;
        let along = |c: &IVec2| if vertical { c.y } else { c.x };
        // Летим вверх/влево — первым задеваем ряд с большим индексом.
        let first = if dir.x + dir.y < 0 {
            solid.iter().map(along).max().unwrap()
        } else {
            solid.iter().map(along).min().unwrap()
        };

        // Полоса поперёк полёта: 4 четвертинки (ширина танка) по центру пули.
        let center = if vertical { pos.x + size / 2.0 } else { pos.y + size / 2.0 };
        let start = ((center - tile_size) / q).round() as i32;
        let depth = if pierce { 2 } else { 1 };
        // Ряды в глубину: от первого задетого дальше по ходу полёта.
        let step = if dir.x + dir.y < 0 { -1 } else { 1 };

        for d in 0..depth {
            let row = first + d * step;
            for across in start..start + 4 {
                let (qx, qy) = if vertical { (across, row) } else { (row, across) };
                self.break_quarter(qx, qy, pierce);
            }
        }
    }

    /// Разрушить четвертинку: кирпич — всегда, сталь — только `pierce`
    /// (сталь разрушается целым тайлом).
    fn break_quarter(&mut self, qx: i32, qy: i32, pierce: bool) {
        let (tx, ty) = (qx.div_euclid(2), qy.div_euclid(2));
        if !self.in_bounds(tx, ty) {
            return;
        }
        let idx = (tx + ty * self.width) as usize;
        match self.tiles[idx] {
            Tile::Brick => {
                let bit = qx.rem_euclid(2) + 2 * qy.rem_euclid(2);
                self.brick_mask[idx] &= !(1 << bit);
                if self.brick_mask[idx] == 0 {
                    self.tiles[idx] = Tile::Empty;
                }
            }
            Tile::Steel if pierce => self.set_tile(tx, ty, Tile::Empty),
            _ => {}
        }
    }

    fn destroy_base(&mut self) {
        if let Some(origin) = self.base_origin {
            for dx in 0..2 {
                for dy in 0..2 {
                    self.set_tile(origin.x + dx, origin.y + dy, Tile::Empty);
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
            map.set_tile(x, y, Tile::Steel);
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
            self.set_tile(t.x, t.y, Tile::Brick);
        }
    }

    /// Случайное место для бонуса размером 2x2 тайла (как танк).
    /// Все 4 тайла должны быть пустыми, а область не должна пересекаться
    /// с областями 2x2 из `forbidden` (точки появления, другие бонусы).
    pub fn random_bonus_spawn(&self, forbidden: &[IVec2]) -> Option<IVec2> {
        let mut candidates: Vec<IVec2> = Vec::new();
        for y in 0..(self.height - 1) {
            for x in 0..(self.width - 1) {
                let all_empty = (0..2).all(|dy| {
                    (0..2).all(|dx| self.get_tile(x + dx, y + dy) == Tile::Empty)
                });
                if !all_empty {
                    continue;
                }
                let p = ivec2(x, y);
                if forbidden.iter().any(|f| (f.x - p.x).abs() < 2 && (f.y - p.y).abs() < 2) {
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
                        let mask = self.brick_mask[(x + y * self.width) as usize];
                        draw_brick_tile(px, py, tile_size, mask);
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

/// Кирпичный тайл: рисуются только целые четвертинки, каждая —
/// как два ряда кирпичей со швами.
fn draw_brick_tile(px: f32, py: f32, tile_size: f32, mask: u8) {
    let q = tile_size / 2.0;
    let mortar = DARKBROWN;
    let line = (tile_size / 16.0).max(1.0);
    for bit in 0..4 {
        if mask & (1 << bit) == 0 {
            continue;
        }
        let x = px + (bit % 2) as f32 * q;
        let y = py + (bit / 2) as f32 * q;
        draw_rectangle(x, y, q, q, ORANGE);
        // Горизонтальный шов посередине и снизу.
        draw_rectangle(x, y + q / 2.0 - line, q, line, mortar);
        draw_rectangle(x, y + q - line, q, line, mortar);
        // Вертикальные швы со сдвигом в соседних рядах.
        draw_rectangle(x + q / 2.0 - line / 2.0, y, line, q / 2.0, mortar);
        draw_rectangle(x, y + q / 2.0, line, q / 2.0, mortar);
    }
}
