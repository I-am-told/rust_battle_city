use macroquad::prelude::*;
use crate::entities::tank::Tank;
use crate::entities::bonus::BonusKind;
use crate::map::GameMap;

/// Применяет эффект подобранного бонуса.
pub fn apply_bonus(
    kind: BonusKind,
    player: &mut Tank,
    player_lives: &mut u32,
    enemies: &mut Vec<Tank>,
    map: &mut GameMap,
    freeze_timer: &mut f32,
    shovel_timer: &mut f32,
    shovel_tiles: &mut Vec<IVec2>,
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
            // Как в оригинале: граната уничтожает всех врагов на поле,
            // включая бронированных (независимо от HP).
            let mut killed = 0;
            for e in enemies.iter_mut() {
                if e.alive {
                    e.hp = 0;
                    e.alive = false;
                    killed += 1;
                }
            }
            enemies.retain(|e| e.alive);
            println!("Grenade! Killed {} enemies", killed);
        }
        BonusKind::Timer => {
            *freeze_timer = 10.0;
            println!("Timer picked! Enemies frozen for 10s");
        }
        BonusKind::Shovel => {
            // Если лопата уже активна — снимаем старую, потом ставим новую.
            if *shovel_timer > 0.0 {
                map.remove_shovel(shovel_tiles);
                shovel_tiles.clear();
            }
            *shovel_timer = 10.0;
            *shovel_tiles = map.apply_shovel();
            println!("Shovel picked! Base wrapped in steel for 10s");
        }
    }
}