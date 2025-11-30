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

/// 绘制时间网格线
#[allow(dead_code)]
pub fn draw_timeline_grid(
    painter: &Painter,
    rect: Rect,
    start_time: f64,
    end_time: f64,
    zoom_x: f32,
    snap_interval: f64,
    header_width: f32,
) {
    if snap_interval <= 0.0 {
        return;
    }

    let mut time = (start_time / snap_interval).floor() * snap_interval;
    while time <= end_time {
        let x = ((time - start_time) * zoom_x as f64) as f32 + header_width;
        if x >= rect.min.x && x <= rect.max.x {
            draw_dashed_vertical_line(
                painter,
                x,
                rect.min.y,
                rect.max.y,
                Stroke::new(0.5, Color32::from_gray(50)),
            );
        }
        time += snap_interval;
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

/// 在轨道内容区域绘制垂直网格线
/// 参考 egui_midi 的网格绘制逻辑，但适配时间轴（秒）而非节拍
pub fn draw_track_grid(
    painter: &Painter,
    rect: Rect,
    timeline: &crate::structure::TimelineState,
    header_width: f32,
) {
    if !timeline.snap_enabled || timeline.snap_interval <= 0.0 {
        return;
    }

    let start_time = timeline.scroll_x;
    let end_time = start_time + (rect.width() as f64 / timeline.zoom_x as f64);
    
    // 计算主网格间隔（snap_interval）和次网格间隔
    let major_interval = timeline.snap_interval;
    let minor_interval = major_interval / 4.0; // 将主间隔分成4份作为次网格
    
    // 定义颜色
    let major_line_color = Color32::from_rgb(140, 140, 140);
    let minor_line_color = Color32::from_rgb(90, 90, 90);
    
    // 绘制次网格线（虚线）
    let mut time = (start_time / minor_interval).floor() * minor_interval;
    while time <= end_time {
        let x = timeline.time_to_x(time) + header_width;
        if x >= rect.min.x && x <= rect.max.x {
            // 只在不是主网格线位置时绘制次网格线
            if (time % major_interval).abs() > 0.001 {
                draw_dashed_vertical_line(
                    painter,
                    x,
                    rect.min.y,
                    rect.max.y,
                    Stroke::new(0.5, minor_line_color),
                );
            }
        }
        time += minor_interval;
    }
    
    // 绘制主网格线（实线，较粗）
    let mut time = (start_time / major_interval).floor() * major_interval;
    while time <= end_time {
        let x = timeline.time_to_x(time) + header_width;
        if x >= rect.min.x && x <= rect.max.x {
            painter.line_segment(
                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                Stroke::new(1.0, major_line_color),
            );
        }
        time += major_interval;
    }
}
