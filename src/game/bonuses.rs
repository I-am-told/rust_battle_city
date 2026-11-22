use crate::entities::bonus::BonusKind;
use crate::game::state::{Game, MAX_RANK};

impl Game {
    /// Применяет эффект бонуса, подобранного игроком `pi`.
    /// Времена (заморозка, лопата, каска) — из `gameplay` в config.ron.
    pub(crate) fn apply_bonus(&mut self, pi: usize, kind: BonusKind) {
        let tuning = self.tuning;
        match kind {
            BonusKind::Star => {
                // Лёгкий режим: звезда — обоим игрокам, кто бы её ни взял.
                if self.son_rules() {
                    for (i, p) in self.players.iter_mut().enumerate() {
                        if i != pi {
                            let r = (p.tank.rank + 1).min(MAX_RANK);
                            p.tank.apply_rank(r);
                        }
                    }
                }
                let tank = &mut self.players[pi].tank;
                let new_rank = (tank.rank + 1).min(MAX_RANK);
                tank.apply_rank(new_rank);
                println!(
                    "Star picked! Rank {} (max_bullets={}, pierce={}, speed_mult={})",
                    tank.rank, tank.max_active_bullets, tank.pierces_steel, tank.bullet_speed_multiplier,
                );
            }
            BonusKind::Life => {
                self.lives += 1;
                println!("Life picked! Lives: {}", self.lives);
            }
            BonusKind::Grenade => {
                // Как в оригинале: граната уничтожает всех врагов на поле,
                // включая бронированных (независимо от HP).
                // Лёгкий режим: бонусный (мигающий) враг не взрывается —
                // пусть сын подобьёт его и получит бонус.
                let spare_bonus = self.son_rules();
                let mut killed = 0;
                for e in self.enemies.iter_mut().filter(|e| e.alive && !(spare_bonus && e.is_bonus)) {
                    e.hp = 0;
                    e.alive = false;
                    killed += 1;
                }
                // Убитых не удаляем здесь: Game сам уберёт их и нарисует взрывы.
                println!("Grenade! Killed {killed} enemies");
            }
            BonusKind::Timer => {
                self.freeze_timer = tuning.freeze_time;
                println!("Timer picked! Enemies frozen for {}s", tuning.freeze_time);
            }
            BonusKind::Helmet => {
                // Лёгкий режим: щит — обоим (живым) игрокам, кто бы его ни взял.
                if self.son_rules() {
                    for p in self.players.iter_mut().filter(|p| p.tank.alive) {
                        p.give_shield(tuning.helmet_shield);
                    }
                }
                self.players[pi].give_shield(tuning.helmet_shield);
                println!("Helmet picked! Shield for {}s", tuning.helmet_shield);
            }
            BonusKind::Mama => {
                // Помощник (кого выбрали в меню или случайный) появляется
                // слева от сына и сразу начинает облетать его.
                let pos = self.guarded_center().unwrap_or_default()
                    - macroquad::prelude::vec2(tuning.monkey_orbit_radius * self.tank_size, 0.0);
                let time = tuning.mama_time;
                let kind = self.helper.pick();
                self.monkey = Some(crate::entities::monkey::Monkey::new(kind, pos, self.tank_size, time));
                println!("Mama picked! {} for {time}s", kind.label());
            }
            BonusKind::Shovel => {
                // Если лопата уже активна — снимаем старую, потом ставим новую.
                if self.shovel_timer > 0.0 {
                    self.map.remove_shovel(&self.shovel_tiles);
                    self.shovel_tiles.clear();
                }
                self.shovel_timer = tuning.shovel_time;
                self.shovel_tiles = self.map.apply_shovel();
                println!("Shovel picked! Base wrapped in steel for {}s", tuning.shovel_time);
            }
        }
    }
}
