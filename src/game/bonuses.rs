use crate::entities::tank::{Tank, Team};
use crate::entities::bonus::BonusKind;

/// Применяет эффект подобранного бонуса.
/// Возвращает `true`, если игрок (или игровое состояние) изменились настолько,
/// что это стоит перерисовать — но это необязательно, сейчас просто для
/// логирования и задела.
pub fn apply_bonus(
    kind: BonusKind,
    player: &mut Tank,
    player_lives: &mut u32,
    enemies: &mut Vec<Tank>,
    max_rank: u8,
) {
    match kind {
        BonusKind::Star => {
            let new_rank = (player.rank + 1).min(max_rank);
            player.apply_rank(new_rank);
            println!(
                "Star picked! Rank {} (max_bullets={}, pierce={}, speed_mult={})",
                player.rank,
                player.max_active_bullets,
                player.pierces_steel,
                player.bullet_speed_multiplier,
            );
        }
        BonusKind::Life => {
            *player_lives += 1;
            println!("Life picked! Lives: {}", player_lives);
        }
        BonusKind::Grenade => {
            let mut killed = 0;
            for e in enemies.iter_mut() {
                if e.alive {
                    e.take_hit();
                    killed += 1;
                }
            }
            enemies.retain(|e| e.alive);
            println!("Grenade! Killed {} enemies", killed);
        }
        BonusKind::Timer => {
            println!("Timer picked (effect pending)");
        }
        BonusKind::Shovel => {
            println!("Shovel picked (effect pending)");
        }
    }

    // Заглушка: Team используется в будущем (для мультиплеера и врагов).
    let _ = Team::Player;
}