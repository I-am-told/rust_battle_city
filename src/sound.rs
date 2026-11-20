//! Звук. Все звуки синтезируются при запуске (8-битный стиль:
//! квадратная и треугольная волна, шум) — внешних файлов нет,
//! мелодии свои.
//!
//! Игра не играет звуки сама: она складывает события `Sfx` в список
//! (`Game::sfx`), а `main` раз в кадр отдаёт их сюда. Так логика игры
//! не зависит от звука.

use std::collections::HashMap;
use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};

/// Звуковые события игры.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sfx {
    /// Выстрел игрока.
    Shot,
    /// Пуля игрока попала в кирпич.
    HitBrick,
    /// Пуля игрока попала в сталь или край поля.
    HitSteel,
    /// Попадание в бронированный танк (не убит).
    HitArmor,
    /// Уничтожен враг.
    EnemyBoom,
    /// Уничтожен игрок.
    PlayerBoom,
    /// Уничтожен штаб.
    BaseBoom,
    /// Появился бонус.
    BonusAppear,
    /// Подобран бонус.
    BonusPickup,
    /// Подобрана жизнь.
    LifeUp,
    /// Пауза.
    Pause,
    /// Начало уровня.
    LevelStart,
    /// Конец игры.
    GameOver,
}

const RATE: u32 = 22_050;

// ---------- синтез ----------

#[derive(Clone, Copy)]
enum Wave {
    /// Квадратная с заданной скважностью (0.125, 0.25, 0.5).
    Square(f32),
    Triangle,
    /// Шум: новое случайное значение `rate` раз в секунду
    /// (чем ниже — тем «глуше»).
    Noise,
}

/// Один звук: частота плавно меняется от `f0` до `f1`,
/// громкость — от 1 до 0 по кривой `decay` (1 — линейно, больше — резче).
struct Tone {
    wave: Wave,
    f0: f32,
    f1: f32,
    dur: f32,
    vol: f32,
    decay: f32,
}

fn render(tones: &[Tone], out: &mut Vec<f32>) {
    // Простой генератор шума (xorshift), одинаковый при каждом запуске.
    let mut seed: u32 = 0x1234_5678;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    for t in tones {
        let n = (t.dur * RATE as f32) as usize;
        let mut phase = 0.0f32;
        let mut noise_val = 0.0f32;
        let mut noise_acc = 1.0f32;
        for i in 0..n {
            let k = i as f32 / n as f32;
            let f = t.f0 + (t.f1 - t.f0) * k;
            phase = (phase + f / RATE as f32).fract();
            let v = match t.wave {
                Wave::Square(duty) => if phase < duty { 1.0 } else { -1.0 },
                Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                Wave::Noise => {
                    noise_acc += f / RATE as f32;
                    if noise_acc >= 1.0 {
                        noise_acc -= 1.0;
                        noise_val = rnd();
                    }
                    noise_val
                }
            };
            // Короткая атака (2 мс), чтобы не щёлкало.
            let attack = (i as f32 / (RATE as f32 * 0.002)).min(1.0);
            let env = (1.0 - k).powf(t.decay) * attack;
            out.push(v * env * t.vol);
        }
    }
}

/// Мелодия: ноты (частота, длительность) квадратной волной.
fn melody(notes: &[(f32, f32)], duty: f32, vol: f32) -> Vec<Tone> {
    notes
        .iter()
        .map(|&(f, d)| Tone { wave: Wave::Square(duty), f0: f, f1: f, dur: d, vol, decay: 0.6 })
        .collect()
}

/// Две дорожки (мелодия + бас) — смешать.
fn mix(a: Vec<f32>, b: Vec<f32>) -> Vec<f32> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0))
        .collect()
}

fn tones(t: &[Tone]) -> Vec<f32> {
    let mut v = Vec::new();
    render(t, &mut v);
    v
}

// Ноты (Гц).
const C4: f32 = 261.6;
const E4: f32 = 329.6;
const G4: f32 = 392.0;
const A4: f32 = 440.0;
const C5: f32 = 523.3;
const D5: f32 = 587.3;
const E5: f32 = 659.3;
const G5: f32 = 784.0;
const A5: f32 = 880.0;
const C6: f32 = 1046.5;
const E6: f32 = 1318.5;
const G6: f32 = 1568.0;
const C7: f32 = 2093.0;

/// Сэмплы для каждого звука.
fn samples(s: Sfx) -> Vec<f32> {
    use Wave::*;
    let t = |wave, f0, f1, dur, vol, decay| Tone { wave, f0, f1, dur, vol, decay };
    match s {
        // «Пиу»: быстрый спад частоты.
        Sfx::Shot => tones(&[t(Square(0.25), 950.0, 260.0, 0.09, 0.35, 1.0)]),
        // Глухой «тук» шумом.
        Sfx::HitBrick => tones(&[t(Noise, 2500.0, 900.0, 0.09, 0.5, 1.5)]),
        // Звонкое «дзинь».
        Sfx::HitSteel => tones(&[
            t(Square(0.5), 1900.0, 1700.0, 0.03, 0.25, 0.5),
            t(Square(0.5), 2400.0, 2200.0, 0.06, 0.2, 1.5),
        ]),
        Sfx::HitArmor => mix(
            tones(&[t(Square(0.125), 700.0, 500.0, 0.08, 0.3, 1.0)]),
            tones(&[t(Noise, 4000.0, 2000.0, 0.06, 0.25, 1.0)]),
        ),
        // Взрыв: шум, который «опускается» и затихает.
        Sfx::EnemyBoom => tones(&[t(Noise, 3500.0, 250.0, 0.45, 0.6, 1.4)]),
        Sfx::PlayerBoom => mix(
            tones(&[t(Noise, 2500.0, 150.0, 0.7, 0.55, 1.2)]),
            tones(&[t(Triangle, 140.0, 40.0, 0.5, 0.4, 1.0)]),
        ),
        Sfx::BaseBoom => mix(
            tones(&[t(Noise, 1800.0, 80.0, 1.3, 0.55, 1.1)]),
            tones(&[t(Triangle, 90.0, 30.0, 1.0, 0.4, 0.8)]),
        ),
        Sfx::BonusAppear => tones(&[
            t(Triangle, A5, A5, 0.05, 0.4, 0.3),
            t(Triangle, E6, E6, 0.07, 0.4, 0.8),
        ]),
        Sfx::BonusPickup => tones(&melody(&[(C6, 0.05), (E6, 0.05), (G6, 0.05), (C7, 0.1)], 0.5, 0.25)),
        Sfx::LifeUp => tones(&melody(
            &[(G5, 0.06), (C6, 0.06), (E6, 0.06), (G6, 0.06), (E6, 0.06), (G6, 0.06), (C7, 0.16)],
            0.25,
            0.25,
        )),
        Sfx::Pause => tones(&melody(&[(C6, 0.07), (G5, 0.07), (C6, 0.07), (G5, 0.12)], 0.5, 0.22)),
        // Своя короткая фанфара + бас.
        Sfx::LevelStart => mix(
            tones(&melody(
                &[(C5, 0.12), (E5, 0.12), (G5, 0.12), (C6, 0.24), (A5, 0.12), (C6, 0.36)],
                0.25,
                0.22,
            )),
            tones(&[
                t(Triangle, C4, C4, 0.36, 0.35, 0.3),
                t(Triangle, G4, G4, 0.36, 0.35, 0.3),
                t(Triangle, C4, C4, 0.36, 0.35, 0.6),
            ]),
        ),
        // Грустный спуск.
        Sfx::GameOver => mix(
            tones(&melody(&[(G5, 0.22), (E5, 0.22), (D5, 0.22), (C5, 0.6)], 0.5, 0.22)),
            tones(&[t(Triangle, A4, A4, 0.44, 0.3, 0.3), t(Triangle, E4, C4, 0.82, 0.3, 0.8)]),
        ),
    }
}

/// Гул мотора (зацикленный): низкий «квадрат» с лёгким дрожанием.
/// Длина — ровно целое число периодов, чтобы на стыке не щёлкало.
fn engine_samples() -> Vec<f32> {
    let f = 55.0;
    let periods = 22.0;
    let n = (periods / f * RATE as f32) as usize;
    (0..n)
        .map(|i| {
            let tt = i as f32 / RATE as f32;
            let ph = (tt * f).fract();
            let sq = if ph < 0.3 { 1.0 } else { -1.0 };
            let wobble = (tt * f * 2.0).fract();
            let tri = 4.0 * (wobble - 0.5).abs() - 1.0;
            (sq * 0.6 + tri * 0.4) * 0.25
        })
        .collect()
}

/// WAV (PCM 16 бит, моно) из сэмплов -1..1.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // моно
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

pub const ALL_SFX: [Sfx; 13] = [
    Sfx::Shot, Sfx::HitBrick, Sfx::HitSteel, Sfx::HitArmor, Sfx::EnemyBoom,
    Sfx::PlayerBoom, Sfx::BaseBoom, Sfx::BonusAppear, Sfx::BonusPickup,
    Sfx::LifeUp, Sfx::Pause, Sfx::LevelStart, Sfx::GameOver,
];

// ---------- воспроизведение ----------

pub struct Audio {
    sounds: HashMap<Sfx, Sound>,
    engine: Option<Sound>,
    engine_on: bool,
    volume: f32,
}

impl Audio {
    /// `volume` 0..1 (0 — звук выключен, ничего не загружается),
    /// `engine` — играть ли гул мотора.
    pub async fn load(volume: f32, engine: bool) -> Self {
        let volume = volume.clamp(0.0, 1.0);
        let mut sounds = HashMap::new();
        let mut engine_sound = None;
        if volume > 0.0 {
            for s in ALL_SFX {
                match load_sound_from_bytes(&wav_bytes(&samples(s))).await {
                    Ok(snd) => {
                        sounds.insert(s, snd);
                    }
                    Err(e) => eprintln!("Звук {s:?} не загрузился: {e}"),
                }
            }
            if engine {
                engine_sound = load_sound_from_bytes(&wav_bytes(&engine_samples())).await.ok();
            }
        }
        Self { sounds, engine: engine_sound, engine_on: false, volume }
    }

    pub fn play(&self, s: Sfx) {
        if let Some(snd) = self.sounds.get(&s) {
            play_sound(snd, PlaySoundParams { looped: false, volume: self.volume });
        }
    }

    /// Включить/выключить гул мотора (едет ли кто-то из игроков).
    pub fn set_engine(&mut self, on: bool) {
        let Some(snd) = &self.engine else { return };
        if on == self.engine_on {
            return;
        }
        self.engine_on = on;
        if on {
            play_sound(snd, PlaySoundParams { looped: true, volume: self.volume * 0.5 });
            set_sound_volume(snd, self.volume * 0.5);
        } else {
            stop_sound(snd);
        }
    }
}
