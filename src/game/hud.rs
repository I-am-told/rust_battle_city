//! Рамка и боковая панель, как на Dandy: серый фон, чёрное поле,
//! справа — значки оставшихся врагов, жизни и флажок с номером уровня.
//!
//! Все размеры — в тайлах поля (тайл = половина танка), поэтому
//! картинка одинаковая при любом tile_size.

use macroquad::prelude::*;
use crate::ui_text::draw_text;

/// Поля рамки вокруг игрового поля, в тайлах (как в оригинале:
/// слева 2, сверху и снизу по 1, справа — панель шириной 4).
pub const FRAME_LEFT: f32 = 2.0;
pub const FRAME_TOP: f32 = 1.0;
pub const FRAME_RIGHT: f32 = 4.0;
pub const FRAME_BOTTOM: f32 = 1.0;

/// Цвет рамки (серый, как в оригинале).
pub const FRAME_COLOR: Color = Color::new(0.45, 0.45, 0.45, 1.0);
const INK: Color = Color::new(0.0, 0.0, 0.0, 1.0);

/// Размер всего «экрана» игры (поле + рамка) в пикселях мира.
pub fn frame_size(tile_size: f32, grid_w: i32, grid_h: i32) -> Vec2 {
    vec2(
        (grid_w as f32 + FRAME_LEFT + FRAME_RIGHT) * tile_size,
        (grid_h as f32 + FRAME_TOP + FRAME_BOTTOM) * tile_size,
    )
}

/// Камера для игры: поле в координатах мира начинается в (0, 0),
/// рамка — вокруг. Масштаб одинаковый по обеим осям (пропорции
/// сохраняются при любом размере окна), лишнее место — чёрные полосы.
pub fn game_camera(tile_size: f32, grid_w: i32, grid_h: i32) -> Camera2D {
    let size = frame_size(tile_size, grid_w, grid_h);
    let origin = vec2(-FRAME_LEFT * tile_size, -FRAME_TOP * tile_size);
    let center = origin + size / 2.0;

    let sw = screen_width().max(1.0);
    let sh = screen_height().max(1.0);
    let scale = (sw / size.x).min(sh / size.y);

    Camera2D {
        target: center,
        zoom: vec2(2.0 * scale / sw, 2.0 * scale / sh),
        ..Default::default()
    }
}

/// Серая рамка и чёрное поле. Рисовать в камере `game_camera`.
pub fn draw_frame(tile_size: f32, grid_w: i32, grid_h: i32) {
    let size = frame_size(tile_size, grid_w, grid_h);
    draw_rectangle(
        -FRAME_LEFT * tile_size,
        -FRAME_TOP * tile_size,
        size.x,
        size.y,
        FRAME_COLOR,
    );
    draw_rectangle(
        0.0,
        0.0,
        grid_w as f32 * tile_size,
        grid_h as f32 * tile_size,
        BLACK,
    );
}

/// Значок танка (вид сверху, ствол вверх). `x, y` — левый верхний угол.
pub fn draw_tank_icon(x: f32, y: f32, size: f32, color: Color) {
    let track = size * 0.25;
    // Гусеницы.
    draw_rectangle(x, y + size * 0.1, track, size * 0.9, color);
    draw_rectangle(x + size - track, y + size * 0.1, track, size * 0.9, color);
    // Корпус.
    draw_rectangle(x + track * 0.6, y + size * 0.3, size - track * 1.2, size * 0.55, color);
    // Ствол.
    draw_rectangle(x + size * 0.42, y, size * 0.16, size * 0.5, color);
}

/// Данные для панели.
pub struct PanelInfo<'a> {
    pub enemies_left: u32,
    pub lives: u32,
    /// Цвета танков игроков (1 или 2).
    pub player_colors: &'a [Color],
    pub level: u32,
    pub freeze_timer: f32,
    pub shovel_timer: f32,
}

/// Боковая панель справа от поля. Рисовать в камере `game_camera`.
pub fn draw_panel(tile_size: f32, grid_w: i32, grid_h: i32, info: &PanelInfo) {
    let ts = tile_size;
    let x0 = grid_w as f32 * ts; // левый край панели
    let gh = grid_h as f32;

    // Значки врагов, которые ещё не вышли: 2 столбца сверху вниз.
    let icon = ts * 0.8;
    for i in 0..info.enemies_left {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        let y = ts * 0.5 + row * ts;
        if y + icon > gh * ts * 0.55 {
            break; // не залезать на блок жизней (больше ~26 врагов)
        }
        draw_tank_icon(x0 + ts * 1.0 + col * ts, y, icon, INK);
    }

    // Жизни (общий запас): значки игроков и число.
    let ly = gh * ts * 0.58;
    draw_text("IP", x0 + ts * 1.0, ly, ts * 1.1, INK);
    for (i, c) in info.player_colors.iter().enumerate() {
        draw_tank_icon(x0 + ts * 1.0 + i as f32 * ts, ly + ts * 0.3, icon, *c);
    }
    draw_text(
        &info.lives.to_string(),
        x0 + ts * 1.0,
        ly + ts * 2.6,
        ts * 1.6,
        INK,
    );

    // Действующие бонусы: заморозка и лопата (секунды).
    let mut ty = gh * ts * 0.75;
    if info.freeze_timer > 0.0 {
        draw_text(
            &format!("T {:.0}", info.freeze_timer.ceil()),
            x0 + ts * 1.0,
            ty,
            ts * 1.1,
            DARKBLUE,
        );
        ty += ts;
    }
    if info.shovel_timer > 0.0 {
        draw_text(
            &format!("S {:.0}", info.shovel_timer.ceil()),
            x0 + ts * 1.0,
            ty,
            ts * 1.1,
            DARKBROWN,
        );
    }

    // Флажок и номер уровня внизу.
    let fx = x0 + ts * 1.1;
    let fy = gh * ts * 0.84;
    draw_rectangle(fx, fy, ts * 0.15, ts * 1.6, INK); // древко
    draw_triangle(
        vec2(fx + ts * 0.15, fy),
        vec2(fx + ts * 0.15, fy + ts * 0.9),
        vec2(fx + ts * 1.3, fy + ts * 0.45),
        ORANGE,
    );
    draw_text(
        &info.level.to_string(),
        x0 + ts * 1.0,
        fy + ts * 2.9,
        ts * 1.6,
        INK,
    );
}
