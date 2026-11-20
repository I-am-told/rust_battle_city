//! Текст без «чёрных квадратиков».
//!
//! macroquad растеризует каждый символ отдельно для каждого размера шрифта
//! и складывает в общий атлас. Когда атлас переполняется, он пересоздаётся
//! (старая текстура удаляется), и на части видеодрайверов уже нарисованные
//! надписи превращаются в чёрные квадраты. Так было с панелью после паузы:
//! надпись PAUSE впервые запрашивала новый размер шрифта.
//!
//! Поэтому весь текст рисуется только в нескольких фиксированных размерах
//! растра (BASE_SIZES) с масштабированием, а все символы этих размеров
//! кладутся в атлас один раз при запуске (`warm_up`). После этого атлас
//! больше не растёт.
//!
//! Функции повторяют сигнатуры macroquad: достаточно
//! `use crate::ui_text::{draw_text, measure_text};` — явный импорт
//! перекрывает одноимённые функции из `macroquad::prelude::*`.

use macroquad::prelude::{Color, Font};
use macroquad::text::{self, get_default_font, TextDimensions, TextParams};

/// Размеры растра. Нужный размер берётся ближайшим сверху и уменьшается
/// (крупнее 64 — увеличивается из 64).
const BASE_SIZES: [u16; 3] = [16, 32, 64];

fn base_for(size: f32) -> u16 {
    BASE_SIZES
        .iter()
        .copied()
        .find(|b| *b as f32 >= size)
        .unwrap_or(BASE_SIZES[BASE_SIZES.len() - 1])
}

/// Положить в атлас все печатные ASCII-символы всех базовых размеров.
/// Вызывать один раз при запуске, до первой отрисовки.
pub fn warm_up() {
    let chars: Vec<char> = (32u8..=126).map(|c| c as char).collect();
    let font = get_default_font();
    for size in BASE_SIZES {
        font.populate_font_cache(&chars, size);
    }
}

pub fn draw_text(t: impl AsRef<str>, x: f32, y: f32, font_size: f32, color: Color) -> TextDimensions {
    let base = base_for(font_size);
    text::draw_text_ex(
        t,
        x,
        y,
        TextParams {
            font_size: base,
            font_scale: font_size / base as f32,
            color,
            ..Default::default()
        },
    )
}

pub fn measure_text(t: impl AsRef<str>, font: Option<&Font>, font_size: u16, font_scale: f32) -> TextDimensions {
    let size = font_size as f32 * font_scale;
    let base = base_for(size);
    text::measure_text(t, font, base, size / base as f32)
}
