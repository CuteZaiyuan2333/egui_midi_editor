use egui::*;

/// 绘制虚线垂直线的工具函数
/// 参考 egui_midi 的绘制模式，但不依赖其内部结构
#[allow(dead_code)]
pub fn draw_dashed_vertical_line(
    painter: &Painter,
    x: f32,
    top: f32,
    bottom: f32,
    stroke: Stroke,
) {
    let dash_len = 2.0;
    let gap_len = 2.0;
    let mut y = top;
    while y < bottom {
        let next = (y + dash_len).min(bottom);
        painter.line_segment([Pos2::new(x, y), Pos2::new(x, next)], stroke);
        y += dash_len + gap_len;
    }
}

/// 绘制选择框
pub fn draw_selection_box(painter: &Painter, rect: Rect) {
    // Draw filled semi-transparent background
    painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(100, 150, 255, 50));

    // Draw border
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(2.0, Color32::from_rgb(100, 150, 255)),
    );
}

/// 绘制统一的时间网格线（用于时间轴和所有轨道）
/// 直接复用 MIDI 编辑器的网格绘制逻辑，确保时间轴和轨道使用完全相同的网格系统
pub fn draw_unified_grid(
    painter: &Painter,
    grid_top: f32,
    grid_bottom: f32,
    timeline: &crate::structure::TimelineState,
    header_width: f32,
    visible_start_x: f32,
    visible_end_x: f32,
) {
    // 使用与 MIDI 编辑器完全相同的网格计算逻辑
    let tpb = timeline.ticks_per_beat.max(1) as u64;
    let denom = timeline.time_signature.1.max(1) as u64;
    let numer = timeline.time_signature.0.max(1) as u64;
    let ticks_per_measure = (tpb * numer * 4).saturating_div(denom).max(tpb);

    // 计算可见的节拍范围
    let visible_beats_start = (-timeline.manual_scroll_x / timeline.zoom_x).floor();
    let visible_beats_end = visible_beats_start + ((visible_end_x - visible_start_x) / timeline.zoom_x) + 2.0;
    let mut start_tick = (visible_beats_start * tpb as f32).floor() as i64;
    if start_tick < 0 {
        start_tick = 0;
    }
    let end_tick = (visible_beats_end * tpb as f32).ceil() as i64;

    // 根据缩放级别自动调整细分级别（与 MIDI 编辑器一致）
    let subdivision = if timeline.zoom_x >= 220.0 {
        8
    } else if timeline.zoom_x >= 90.0 {
        4
    } else if timeline.zoom_x >= 45.0 {
        2
    } else {
        1
    };
    let tick_step = (tpb / subdivision).max(1);

    // 网格线颜色（与 MIDI 编辑器一致）
    let measure_line_color = Color32::from_rgb(210, 210, 210);  // 小节线：较亮的灰色
    let beat_line_color = Color32::from_rgb(140, 140, 140);    // 拍线：中等灰色
    let subdivision_color = Color32::from_rgb(90, 90, 90);     // 细分线：较暗的灰色

    // 计算网格线的 x 坐标偏移（考虑 header_width 和 manual_scroll_x）
    let note_offset_x = header_width + timeline.manual_scroll_x;

    // 绘制垂直网格线
    let mut tick = (start_tick / tick_step as i64) * tick_step as i64;
    if tick < 0 {
        tick = 0;
    }

    while tick <= end_tick {
        let x = note_offset_x + (tick as f32 / tpb as f32) * timeline.zoom_x;
        if x >= visible_start_x && x <= visible_end_x {
            if tick as u64 % ticks_per_measure == 0 {
                // 小节线：实线，较粗
                painter.line_segment(
                    [Pos2::new(x, grid_top), Pos2::new(x, grid_bottom)],
                    Stroke::new(1.0, measure_line_color),
                );
            } else if tick as u64 % tpb == 0 {
                // 拍线：实线
                painter.line_segment(
                    [Pos2::new(x, grid_top), Pos2::new(x, grid_bottom)],
                    Stroke::new(1.0, beat_line_color),
                );
            } else {
                // 细分线：虚线
                draw_dashed_vertical_line(
                    painter,
                    x,
                    grid_top,
                    grid_bottom,
                    Stroke::new(1.0, subdivision_color),
                );
            }
        }
        tick += tick_step as i64;
    }
}

/// 绘制音符矩形（用于 MIDI 剪辑预览）
/// 参考 egui_midi 的绘制模式，但适配剪辑的外观需求
#[allow(dead_code)]
pub fn draw_note_rect(
    painter: &Painter,
    rect: Rect,
    is_selected: bool,
    color: Color32,
) {
    // 剪辑中的音符通常更粗更高
    let stroke_width = if is_selected { 3.0 } else { 1.5 };
    
    painter.rect_filled(rect.shrink(1.0), 2.0, color);
    painter.rect_stroke(
        rect.shrink(1.0),
        2.0,
        Stroke::new(stroke_width, Color32::WHITE),
    );
}
