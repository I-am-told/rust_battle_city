use macroquad::rand;
use crate::entities::enemy_kind::EnemyKind;

/// Параметры уровня. Пока карта одна (тестовая), меняются только
/// параметры спавна врагов. Когда появятся настоящие карты — добавим
/// сюда ссылку/индекс карты.
pub struct LevelConfig {
    pub total_enemies: u32,
    pub max_on_field: u32,
    pub spawn_interval: f32,
    pub kind_weights: [u32; 4],
}

impl LevelConfig {
    pub fn for_level(n: u32) -> Self {
        match n {
            1 => Self {
                total_enemies: 10,
                max_on_field: 3,
                spawn_interval: 3.0,
                kind_weights: [4, 1, 0, 0],
            },
            2 => Self {
                total_enemies: 15,
                max_on_field: 3,
                spawn_interval: 3.0,
                kind_weights: [3, 2, 1, 0],
            },
            3 => Self {
                total_enemies: 20,
                max_on_field: 4,
                spawn_interval: 2.8,
                kind_weights: [2, 2, 1, 1],
            },
            4 => Self {
                total_enemies: 20,
                max_on_field: 4,
                spawn_interval: 2.5,
                kind_weights: [2, 2, 1, 2],
            },
            5 => Self {
                total_enemies: 25,
                max_on_field: 5,
                spawn_interval: 2.5,
                kind_weights: [1, 2, 2, 2],
            },
            _ => Self {
                total_enemies: 25 + (n - 5) * 5,
                max_on_field: (5 + (n - 5) / 2).min(7),
                spawn_interval: (2.5 - (n - 5) as f32 * 0.1).max(1.2),
                kind_weights: [1, 1, 2, 3],
            },
        }
    }

    pub fn pick_kind(&self) -> EnemyKind {
        let total: u32 = self.kind_weights.iter().sum();
        if total == 0 {
            return EnemyKind::Basic;
        }
        let mut roll = rand::gen_range(0, total);
        for (i, w) in self.kind_weights.iter().enumerate() {
            if roll < *w {
                return match i {
                    0 => EnemyKind::Basic,
                    1 => EnemyKind::Fast,
                    2 => EnemyKind::Power,
                    _ => EnemyKind::Armored,
                };
            }
            roll -= *w;
        }
        EnemyKind::Basic
    }
}