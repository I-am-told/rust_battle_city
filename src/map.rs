use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tile {
    Empty,
    Brick,
    Steel,
    Water,
    Eagle,
}

/// Во что попала пуля (для звука).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WallHit {
    Brick,
    Steel,
    Base,
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
    /// Карта из раскладки (тайлы поля построчно, длина width*height).
    /// None — встроенная тестовая раскладка (если в папке levels нет карт). Штаб и кирпичная стена вокруг
    /// него ставятся всегда, поверх раскладки.
    pub fn with_layout(width: i32, height: i32, layout: Option<&[Tile]>) -> Self {
        let mut tiles = match layout {
            Some(l) if l.len() == (width * height) as usize => l.to_vec(),
            _ => vec![Tile::Empty; (width * height) as usize],
        };
        let builtin = layout.is_none();

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
        if builtin {
            fill(10, 8, 6, 4, Tile::Brick);
            fill(6, 16, 4, 4, Tile::Water);
            fill(16, 4, 4, 4, Tile::Steel);
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

    /// Очистить прямоугольник тайлов (точки появления танков).
    /// Возвращает true, если там что-то было.
    pub fn clear_area(&mut self, x0: i32, y0: i32, w: i32, h: i32) -> bool {
        let mut cleared = false;
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                if self.in_bounds(x, y) && self.get_tile(x, y) != Tile::Empty {
                    self.set_tile(x, y, Tile::Empty);
                    cleared = true;
                }
            }
        }
        cleared
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
    pub fn bullet_hit(&mut self, pos: Vec2, size: f32, dir: IVec2, tile_size: f32, pierce: bool) -> WallHit {
        let q = tile_size / 2.0;
        let (x0, x1, y0, y1) = Self::quarter_range(pos, vec2(size, size), tile_size);

        // Штаб: любое касание уничтожает его.
        for qy in y0..=y1 {
            for qx in x0..=x1 {
                if self.get_tile(qx.div_euclid(2), qy.div_euclid(2)) == Tile::Eagle {
                    self.destroy_base();
                    return WallHit::Base;
                }
            }
        }

        // Первый задетый ряд четвертинок по ходу полёта.
        let solid: Vec<IVec2> = (y0..=y1)
            .flat_map(|qy| (x0..=x1).map(move |qx| ivec2(qx, qy)))
            .filter(|c| self.quarter_solid(c.x, c.y, true))
            .collect();
        if solid.is_empty() {
            return WallHit::Steel;
        }
        // Звук: если задет хоть один кирпич — «кирпич», иначе «сталь».
        let kind = if solid
            .iter()
            .any(|c| self.get_tile(c.x.div_euclid(2), c.y.div_euclid(2)) == Tile::Brick)
        {
            WallHit::Brick
        } else {
            WallHit::Steel
        };
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
        kind
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
                    Tile::Steel => draw_steel_tile(px, py, tile_size),
                    Tile::Water => draw_water_tile(x, y, px, py, tile_size),
                    // Штаб рисуется целиком ниже (он 2x2 тайла).
                    Tile::Eagle => {}
                }
            }
        }

        if let Some(b) = self.base_origin {
            let (bx, by) = (b.x as f32 * tile_size, b.y as f32 * tile_size);
            if self.base_destroyed {
                draw_base_ruins(bx, by, tile_size * 2.0);
            } else {
                draw_eagle(bx, by, tile_size * 2.0);
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

/// Бетон (сталь): светлая плита с фаской — светлый край сверху-слева,
/// тёмный снизу-справа и блестящий квадрат в центре, как на Dandy.
fn draw_steel_tile(px: f32, py: f32, ts: f32) {
    let edge = ts * 0.14;
    draw_rectangle(px, py, ts, ts, Color::new(0.62, 0.62, 0.66, 1.0));
    // Тень снизу и справа.
    draw_rectangle(px, py + ts - edge, ts, edge, Color::new(0.35, 0.35, 0.4, 1.0));
    draw_rectangle(px + ts - edge, py, edge, ts, Color::new(0.35, 0.35, 0.4, 1.0));
    // Блик сверху и слева.
    draw_rectangle(px, py, ts - edge, edge, Color::new(0.88, 0.88, 0.92, 1.0));
    draw_rectangle(px, py, edge, ts - edge, Color::new(0.88, 0.88, 0.92, 1.0));
    // Центральная плитка.
    let m = ts * 0.32;
    draw_rectangle(px + m, py + m, ts - 2.0 * m, ts - 2.0 * m, Color::new(0.95, 0.95, 1.0, 1.0));
}

/// Вода: синяя гладь и светлые «барашки» волн, которые переливаются
/// (два кадра, как в оригинале). Соседние тайлы сдвинуты по фазе, чтобы
/// волны не стояли ровными рядами.
fn draw_water_tile(x: i32, y: i32, px: f32, py: f32, ts: f32) {
    draw_rectangle(px, py, ts, ts, Color::new(0.08, 0.25, 0.75, 1.0));
    let frame = ((get_time() * 1.6) as i32 + x + y) % 2;
    let wave = Color::new(0.55, 0.78, 1.0, 1.0);
    let dark = Color::new(0.04, 0.16, 0.55, 1.0);
    let h = (ts / 16.0).max(1.0) * 1.5;
    let shift = if frame == 0 { 0.0 } else { ts * 0.25 };
    for (row, y_k) in [(0, 0.25), (1, 0.7)] {
        let off = if row == 0 { shift } else { ts * 0.25 - shift };
        let yy = py + ts * y_k;
        // Волна: короткая светлая черта и под ней тёмная тень.
        for k in [0.0, 0.5] {
            let xx = px + off + ts * k;
            let w = ts * 0.28;
            let w = w.min(px + ts - xx);
            if w > 0.0 {
                draw_rectangle(xx, yy, w, h, wave);
                draw_rectangle(xx + h, yy + h, (w - h).max(0.0), h, dark);
            }
        }
    }
}

/// Штаб — орёл на чёрном фоне (`s` — сторона квадрата 2x2 тайла).
fn draw_eagle(x: f32, y: f32, s: f32) {
    let light = Color::new(0.82, 0.82, 0.86, 1.0);
    let mid = Color::new(0.6, 0.6, 0.66, 1.0);
    let dark = Color::new(0.38, 0.38, 0.44, 1.0);
    let p = |u: f32, v: f32| vec2(x + u * s, y + v * s);
    let tri = |a: Vec2, b: Vec2, c: Vec2, col: Color| draw_triangle(a, b, c, col);

    draw_rectangle(x, y, s, s, BLACK);

    // Крылья: три слоя перьев с каждой стороны.
    for (side, sign) in [(0.5, -1.0f32), (0.5, 1.0)] {
        let u = |d: f32| side + sign * d;
        tri(p(u(0.06), 0.28), p(u(0.46), 0.12), p(u(0.40), 0.40), light);
        tri(p(u(0.06), 0.38), p(u(0.44), 0.32), p(u(0.36), 0.56), mid);
        tri(p(u(0.06), 0.48), p(u(0.38), 0.52), p(u(0.28), 0.68), dark);
    }
    // Тело.
    draw_rectangle(x + 0.40 * s, y + 0.26 * s, 0.20 * s, 0.46 * s, light);
    draw_rectangle(x + 0.44 * s, y + 0.30 * s, 0.12 * s, 0.38 * s, mid);
    // Голова и клюв.
    draw_circle(x + 0.5 * s, y + 0.22 * s, 0.10 * s, light);
    tri(p(0.5, 0.26), p(0.62, 0.28), p(0.52, 0.34), Color::new(1.0, 0.75, 0.2, 1.0));
    draw_circle(x + 0.47 * s, y + 0.19 * s, 0.025 * s, BLACK);
    // Хвост веером.
    tri(p(0.42, 0.70), p(0.58, 0.70), p(0.32, 0.92), mid);
    tri(p(0.42, 0.70), p(0.58, 0.70), p(0.68, 0.92), mid);
    tri(p(0.44, 0.70), p(0.56, 0.70), p(0.5, 0.96), light);
    // Лапы.
    draw_rectangle(x + 0.40 * s, y + 0.72 * s, 0.05 * s, 0.10 * s, Color::new(1.0, 0.75, 0.2, 1.0));
    draw_rectangle(x + 0.55 * s, y + 0.72 * s, 0.05 * s, 0.10 * s, Color::new(1.0, 0.75, 0.2, 1.0));
}

/// Разрушенный штаб: груда обломков и сломанные крылья.
fn draw_base_ruins(x: f32, y: f32, s: f32) {
    let p = |u: f32, v: f32| vec2(x + u * s, y + v * s);
    draw_rectangle(x, y, s, s, BLACK);
    let ash = Color::new(0.3, 0.28, 0.28, 1.0);
    let stone = Color::new(0.45, 0.43, 0.42, 1.0);
    // Поникшие крылья.
    draw_triangle(p(0.1, 0.75), p(0.45, 0.55), p(0.35, 0.85), ash);
    draw_triangle(p(0.9, 0.75), p(0.55, 0.55), p(0.65, 0.85), ash);
    // Обломки.
    for (u, v, w, h) in [
        (0.12, 0.78, 0.22, 0.14),
        (0.36, 0.70, 0.28, 0.22),
        (0.66, 0.80, 0.20, 0.12),
        (0.28, 0.86, 0.14, 0.10),
        (0.58, 0.66, 0.10, 0.10),
    ] {
        draw_rectangle(x + u * s, y + v * s, w * s, h * s, stone);
    }
    // Дымок.
    let t = get_time() as f32;
    for i in 0..3 {
        let k = ((t * 0.6 + i as f32 / 3.0) % 1.0).abs();
        let c = p(0.5 + 0.08 * (i as f32 - 1.0), 0.6 - 0.5 * k);
        draw_circle(c.x, c.y, s * (0.06 + 0.08 * k), Color::new(0.5, 0.5, 0.5, 0.6 * (1.0 - k)));
    }
}
