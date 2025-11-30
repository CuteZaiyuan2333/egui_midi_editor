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
        let start_time = self.state.scroll_x;
        let end_time = start_time + (rect.width() as f64 / self.state.zoom_x as f64);
        
        // Calculate major and minor intervals
        let seconds_per_pixel = 1.0 / self.state.zoom_x as f64;
        let major_interval = self.calculate_major_interval(seconds_per_pixel);
        let minor_interval = major_interval / 4.0;

        // Draw minor markers
        let mut time = (start_time / minor_interval).floor() * minor_interval;
        while time <= end_time {
            let x = self.state.time_to_x(time) + self.header_width;
            if x >= rect.min.x && x <= rect.max.x {
                painter.line_segment(
                    [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                    Stroke::new(1.0, Color32::from_gray(60)),
                );
            }
            time += minor_interval;
        }

        // Draw major markers with labels
        let mut time = (start_time / major_interval).floor() * major_interval;
        while time <= end_time {
            let x = self.state.time_to_x(time) + self.header_width;
            if x >= rect.min.x && x <= rect.max.x {
                // Draw thicker line
                painter.line_segment(
                    [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                    Stroke::new(2.0, Color32::from_gray(80)),
                );

                // Draw time label
                let label = self.format_time(time);
                painter.text(
                    Pos2::new(x + 4.0, rect.min.y + 16.0),
                    Align2::LEFT_TOP,
                    label,
                    FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }
            time += major_interval;
        }
    }

    fn draw_playhead(&self, painter: &Painter, rect: Rect) {
        let x = self.state.time_to_x(self.state.playhead_position) + self.header_width;
        if x >= rect.min.x && x <= rect.max.x {
            painter.line_segment(
                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                Stroke::new(2.0, Color32::from_rgb(255, 100, 100)),
            );
        }
    }

    fn draw_grid_lines(&self, painter: &Painter, rect: Rect) {
        if !self.state.snap_enabled {
            return;
        }

        let start_time = self.state.scroll_x;
        let end_time = start_time + (rect.width() as f64 / self.state.zoom_x as f64);
        let mut time = (start_time / self.state.snap_interval).floor() * self.state.snap_interval;

        while time <= end_time {
            let x = self.state.time_to_x(time) + self.header_width;
            if x >= rect.min.x && x <= rect.max.x {
                painter.line_segment(
                    [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                    Stroke::new(0.5, Color32::from_gray(50)),
                );
            }
            time += self.state.snap_interval;
        }
    }

    fn calculate_major_interval(&self, seconds_per_pixel: f64) -> f64 {
        // Calculate appropriate major interval based on zoom level
        let target_pixels_between_markers = 100.0;
        let target_interval = seconds_per_pixel * target_pixels_between_markers;

        // Round to nice values: 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, etc.
        let magnitude = 10.0_f64.powf(target_interval.log10().floor());
        let normalized = target_interval / magnitude;

        let nice_value = if normalized <= 0.15 {
            0.1
        } else if normalized <= 0.35 {
            0.25
        } else if normalized <= 0.75 {
            0.5
        } else if normalized <= 1.5 {
            1.0
        } else if normalized <= 3.5 {
            2.0
        } else {
            5.0
        };

        nice_value * magnitude
    }

    fn format_time(&self, time: f64) -> String {
        let minutes = (time / 60.0) as u32;
        let seconds = time as u32 % 60;
        let milliseconds = ((time % 1.0) * 1000.0) as u32;
        format!("{:02}:{:02}.{:03}", minutes, seconds, milliseconds)
    }
}
