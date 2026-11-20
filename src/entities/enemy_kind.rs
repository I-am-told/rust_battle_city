#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EnemyKind {
    Basic,
    Fast,
    Power,       // быстрая пуля
    Armored,     // 4 HP
}

impl EnemyKind {
    pub fn all() -> [EnemyKind; 4] {
        [EnemyKind::Basic, EnemyKind::Fast, EnemyKind::Power, EnemyKind::Armored]
    }

    /// Множитель скорости танка относительно базовой скорости врага.
    pub fn speed_mult(self) -> f32 {
        match self {
            EnemyKind::Basic  => 1.0,
            EnemyKind::Fast   => 2.0,
            EnemyKind::Power  => 1.0,
            EnemyKind::Armored => 1.0,
        }
    }

    /// Множитель скорости пули относительно базовой скорости пули врага.
    pub fn bullet_speed_mult(self) -> f32 {
        match self {
            EnemyKind::Basic  => 1.0,
            EnemyKind::Fast   => 1.0,
            EnemyKind::Power  => 1.5,
            EnemyKind::Armored => 1.0,
        }
    }

    pub fn max_hp(self) -> u8 {
        match self {
            EnemyKind::Armored => 4,
            _ => 1,
        }
    }

    /// Очки за убийство (на будущее — система очков).
    pub fn score(self) -> u32 {
        match self {
            EnemyKind::Basic  => 100,
            EnemyKind::Fast   => 200,
            EnemyKind::Power  => 300,
            EnemyKind::Armored => 400,
        }
    }
}