use macroquad::prelude::*;
use crate::map::{GameMap, WallHit};

/// Чем закончился полёт пули (для звука).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BulletHit {
    /// Край поля.
    Edge,
    Tank,
    Wall(WallHit),
    /// Встречная пуля.
    Bullet,
}
use crate::entities::tank::{Direction, Tank, Team};

pub struct Bullet {
    pub pos: Vec2,
    pub direction: Direction,
    pub speed: f32,
    pub size: f32,
    pub owner_id: u32,
    pub owner_team: Team,
    pub alive: bool,
    pub pierces_steel: bool,
    /// Во что попала (когда alive стал false).
    pub hit: Option<BulletHit>,
}

impl Bullet {
    pub fn new(
        pos: Vec2,
        direction: Direction,
        speed: f32,
        size: f32,
        owner_id: u32,
        owner_team: Team,
    ) -> Self {
        Self {
            pos,
            direction,
            speed,
            size,
            owner_id,
            owner_team,
            alive: true,
            pierces_steel: false,
            hit: None,
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        map: &mut GameMap,
        tanks: &[Tank],
        tile_size: f32,
    ) -> Option<u32> {
        if !self.alive { return None; }

        let v = self.direction.vector();
        let step = vec2(v.x as f32, v.y as f32) * self.speed * dt;

        let max_step = tile_size * 0.5;
        let total = step.length();
        let steps = (total / max_step).ceil().max(1.0) as i32;
        let sub_step = step / steps as f32;

        for _ in 0..steps {
            let next = self.pos + sub_step;

            if self.out_of_bounds(next, map, tile_size) {
                self.alive = false;
                self.hit = Some(BulletHit::Edge);
                return None;
            }

            if let Some(hit_id) = self.hits_tank(next, tanks) {
                self.alive = false;
                self.hit = Some(BulletHit::Tank);
                return Some(hit_id);
            }

            if self.collides_tile(next, map, tile_size) {
                let w = self.damage_hit_tile(next, map, tile_size);
                self.alive = false;
                self.hit = Some(BulletHit::Wall(w));
                return None;
            }

            self.pos = next;
        }

        None
    }

    fn out_of_bounds(&self, pos: Vec2, map: &GameMap, tile_size: f32) -> bool {
        let playfield_w = map.width as f32 * tile_size;
        let playfield_h = map.height as f32 * tile_size;
        pos.x < 0.0
            || pos.y < 0.0
            || pos.x + self.size > playfield_w
            || pos.y + self.size > playfield_h
    }

    fn hits_tank(&self, pos: Vec2, tanks: &[Tank]) -> Option<u32> {
        for t in tanks {
            if !t.alive { continue; }
            if t.team == self.owner_team { continue; }
            if t.invulnerable { continue; }
            if pos.x < t.pos.x + t.size
                && pos.x + self.size > t.pos.x
                && pos.y < t.pos.y + t.size
                && pos.y + self.size > t.pos.y
            {
                return Some(t.id);
            }
        }
        None
    }

    fn collides_tile(&self, pos: Vec2, map: &GameMap, tile_size: f32) -> bool {
        // Точность — четвертинка тайла: в пробитую дыру пуля пролетает.
        map.rect_blocked(pos, vec2(self.size, self.size), tile_size, true)
    }

    fn damage_hit_tile(&self, pos: Vec2, map: &mut GameMap, tile_size: f32) -> WallHit {
        map.bullet_hit(pos, self.size, self.direction.vector(), tile_size, self.pierces_steel)
    }

    pub fn resolve_mutual_destruction(bullets: &mut [Bullet]) {
        let n = bullets.len();
        for i in 0..n {
            if !bullets[i].alive { continue; }
            for j in (i + 1)..n {
                if !bullets[j].alive { continue; }
                if bullets[i].owner_team == bullets[j].owner_team { continue; }

                if Self::aabb_overlap(&bullets[i], &bullets[j]) {
                    bullets[i].alive = false;
                    bullets[j].alive = false;
                    bullets[i].hit = Some(BulletHit::Bullet);
                    bullets[j].hit = Some(BulletHit::Bullet);
                    break;
                }
            }
        }
    }

    fn aabb_overlap(a: &Bullet, b: &Bullet) -> bool {
        a.pos.x < b.pos.x + b.size
            && a.pos.x + a.size > b.pos.x
            && a.pos.y < b.pos.y + b.size
            && a.pos.y + a.size > b.pos.y
    }

    pub fn draw(&self) {
        if !self.alive { return; }
        let color = match self.owner_team {
            Team::Player => YELLOW,
            Team::Enemy  => LIGHTGRAY,
        };
        draw_rectangle(self.pos.x, self.pos.y, self.size, self.size, color);
    }
}