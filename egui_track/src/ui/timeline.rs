use crate::structure::TimelineState;
use egui::*;

pub struct Timeline {
    state: TimelineState,
    header_width: f32,
}

impl Timeline {
    pub fn new(state: &TimelineState, header_width: f32) -> Self {
        Self {
            state: state.clone(),
            header_width,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, height: f32) {
        let available_width = ui.available_width();
        let timeline_rect = Rect::from_min_size(
            ui.cursor().left_top(),
            Vec2::new(available_width, height),
        );

        // Draw timeline background
        let painter = ui.painter();
        painter.rect_filled(timeline_rect, 0.0, Color32::from_gray(40));

        // Draw time markers
        self.draw_time_markers(painter, timeline_rect);

        // Draw playhead
        self.draw_playhead(painter, timeline_rect);

        // Draw grid lines
        self.draw_grid_lines(painter, timeline_rect);

        ui.allocate_rect(timeline_rect, Sense::click_and_drag());
    }

    fn draw_time_markers(&self, painter: &Painter, rect: Rect) {
        // 使用基于节拍的坐标系统（与 MIDI 编辑器一致）
        let tpb = self.state.ticks_per_beat.max(1) as u64;
        let denom = self.state.time_signature.1.max(1) as u64;
        let numer = self.state.time_signature.0.max(1) as u64;
        let ticks_per_measure = (tpb * numer * 4).saturating_div(denom).max(tpb);

        // 计算可见的节拍范围
        let visible_beats_start = (-self.state.manual_scroll_x / self.state.zoom_x).floor();
        let visible_beats_end = visible_beats_start + (rect.width() / self.state.zoom_x) + 2.0;
        let mut start_tick = (visible_beats_start * tpb as f32).floor() as i64;
        if start_tick < 0 {
            start_tick = 0;
        }
        let end_tick = (visible_beats_end * tpb as f32).ceil() as i64;

        // 计算网格线的 x 坐标偏移
        let note_offset_x = self.header_width + self.state.manual_scroll_x;

        // 绘制时间标记（在小节线和拍线上）
        let mut tick = (start_tick / ticks_per_measure as i64) * ticks_per_measure as i64;
        if tick < 0 {
            tick = 0;
        }

        while tick <= end_tick {
            let x = note_offset_x + (tick as f32 / tpb as f32) * self.state.zoom_x;
            if x >= rect.min.x && x <= rect.max.x {
                // 判断是小节线还是拍线
                if tick as u64 % ticks_per_measure == 0 {
                    // 小节线：绘制小节标记
                    let measure = (tick as u64 / ticks_per_measure) as u32;
                    let label = format!("{}:1", measure + 1);
                    painter.text(
                        Pos2::new(x + 4.0, rect.min.y + 16.0),
                        Align2::LEFT_TOP,
                        label,
                        FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                } else if tick as u64 % tpb == 0 {
                    // 拍线：绘制拍标记
                    let beat_in_measure = ((tick as u64 % ticks_per_measure) / tpb) as u32 + 1;
                    let measure = (tick as u64 / ticks_per_measure) as u32;
                    let label = format!("{}:{}", measure + 1, beat_in_measure);
                    painter.text(
                        Pos2::new(x + 4.0, rect.min.y + 16.0),
                        Align2::LEFT_TOP,
                        label,
                        FontId::proportional(11.0),
                        Color32::from_gray(200),
                    );
                }
            }
            // 移动到下一个可能的标记位置（小节或拍）
            if tick as u64 % ticks_per_measure == 0 {
                tick += ticks_per_measure as i64;
            } else if tick as u64 % tpb == 0 {
                tick += tpb as i64;
            } else {
                tick += tpb as i64;
            }
        }
    }

    fn draw_playhead(&self, painter: &Painter, rect: Rect) {
        // 使用基于 tick 的坐标转换（与 MIDI 编辑器一致）
        let playhead_tick = self.state.time_to_tick(self.state.playhead_position);
        let x = self.state.tick_to_x(playhead_tick, self.header_width);
        if x >= rect.min.x && x <= rect.max.x {
            // 使用与 MIDI 编辑器一致的样式：半透明蓝色，宽度 2.0
            painter.line_segment(
                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                Stroke::new(2.0, Color32::from_rgba_premultiplied(100, 200, 255, 128)),
            );
        }
    }

    fn draw_grid_lines(&self, painter: &Painter, rect: Rect) {
        // 使用统一的网格绘制函数，确保与轨道网格对齐
        crate::ui::renderer::draw_unified_grid(
            painter,
            rect.min.y,
            rect.max.y,
            &self.state,
            self.header_width,
            rect.min.x,
            rect.max.x,
        );
    }

}
