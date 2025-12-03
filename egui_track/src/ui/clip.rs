use crate::structure::{Clip, TimelineState};
use crate::ui::TrackEditorOptions;
use egui::*;

#[allow(dead_code)]
pub struct ClipRenderer {
    clip: Clip,
    timeline: TimelineState,
    options: TrackEditorOptions,
    is_selected: bool,
    header_width: f32,
}

impl ClipRenderer {
    #[allow(dead_code)]
    pub fn new(clip: &Clip, timeline: &TimelineState, options: &TrackEditorOptions, header_width: f32) -> Self {
        Self {
            clip: clip.clone(),
            timeline: timeline.clone(),
            options: options.clone(),
            is_selected: false,
            header_width,
        }
    }

    #[allow(dead_code)]
    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
    }

    #[allow(dead_code)]
    pub fn render(&self, painter: &Painter, track_y: f32) {
        // 使用基于 tick 的坐标系统（与 MIDI 编辑器一致）
        let start_tick = self.timeline.time_to_tick(self.clip.start_time);
        let duration_tick = self.timeline.time_to_tick(self.clip.start_time + self.clip.duration) - start_tick;
        
        // 使用 tick_to_x 计算位置（与 MIDI 音符一致）
        let x = self.timeline.tick_to_x(start_tick, self.header_width);
        
        // 使用 tick_to_x 计算宽度（与 MIDI 音符一致）
        let tpb = self.timeline.ticks_per_beat.max(1) as f32;
        let width = ((duration_tick as f32 / tpb) * self.timeline.zoom_x).max(self.options.min_clip_width);
        
        let height = 60.0; // Track height minus padding
        let y = track_y + 10.0; // Padding from top

        let clip_rect = Rect::from_min_size(
            Pos2::new(x, y),
            Vec2::new(width, height),
        );

        // 绘制剪辑背景（参考 MIDI 音符的绘制方式）
        painter.rect_filled(clip_rect.shrink(1.0), 4.0, self.clip.color);

        // 绘制剪辑边框（参考 MIDI 音符的绘制方式）
        let stroke_width = if self.is_selected { 4.0 } else { 1.0 };
        let stroke_color = Color32::WHITE;
        painter.rect_stroke(
            clip_rect.shrink(1.0),
            4.0,
            Stroke::new(stroke_width, stroke_color),
        );

        // Draw clip name
        if width > 40.0 {
            painter.text(
                clip_rect.left_top() + Vec2::new(4.0, 4.0),
                Align2::LEFT_TOP,
                &self.clip.name,
                FontId::proportional(11.0),
                Color32::WHITE,
            );
        }

        // Draw clip type specific content
        match &self.clip.clip_type {
            crate::structure::ClipType::Midi { midi_data } => {
                self.render_midi_preview(painter, clip_rect, midi_data);
            }
            crate::structure::ClipType::Audio { audio_data } => {
                self.render_audio_waveform(painter, clip_rect, audio_data);
            }
        }
    }

    #[allow(dead_code)]
    fn render_midi_preview(
        &self,
        painter: &Painter,
        rect: Rect,
        midi_data: &Option<crate::structure::MidiClipData>,
    ) {
        if let Some(data) = midi_data {
            // Draw simplified note preview
            for note in &data.preview_notes {
                let note_x = rect.min.x + (note.start * self.timeline.zoom_x as f64) as f32;
                let note_width = (note.duration * self.timeline.zoom_x as f64).max(2.0) as f32;
                let note_y = rect.min.y + (127 - note.key) as f32 * 0.3;
                let note_height = 4.0;

                let note_rect = Rect::from_min_size(
                    Pos2::new(note_x, note_y),
                    Vec2::new(note_width, note_height),
                );

                painter.rect_filled(note_rect, 1.0, Color32::from_gray(200));
            }
        } else {
            // Draw placeholder text
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "MIDI",
                FontId::proportional(10.0),
                Color32::from_gray(150),
            );
        }
    }

    #[allow(dead_code)]
    fn render_audio_waveform(
        &self,
        painter: &Painter,
        rect: Rect,
        audio_data: &Option<crate::structure::AudioClipData>,
    ) {
        if let Some(data) = audio_data {
            if let Some(waveform) = &data.waveform_data {
                // Draw simplified waveform
                let center_y = rect.center().y;
                let step = rect.width() / waveform.len() as f32;

                for (i, &sample) in waveform.iter().enumerate() {
                    let x = rect.min.x + i as f32 * step;
                    let height = sample * rect.height() * 0.5;
                    painter.line_segment(
                        [
                            Pos2::new(x, center_y - height),
                            Pos2::new(x, center_y + height),
                        ],
                        Stroke::new(1.0, Color32::from_gray(180)),
                    );
                }
            } else {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Audio",
                    FontId::proportional(10.0),
                    Color32::from_gray(150),
                );
            }
        } else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Audio",
                FontId::proportional(10.0),
                Color32::from_gray(150),
            );
        }
    }

    #[allow(dead_code)]
    pub fn hit_test(&self, pos: Pos2, track_y: f32) -> Option<ClipHitRegion> {
        // 使用基于 tick 的坐标系统计算剪辑位置
        let start_tick = self.timeline.time_to_tick(self.clip.start_time);
        let duration_tick = self.timeline.time_to_tick(self.clip.start_time + self.clip.duration) - start_tick;
        
        let x = self.timeline.tick_to_x(start_tick, self.header_width);
        let tpb = self.timeline.ticks_per_beat.max(1) as f32;
        let width = ((duration_tick as f32 / tpb) * self.timeline.zoom_x).max(self.options.min_clip_width);
        let height = 60.0;
        let y = track_y + 10.0;

        let clip_rect = Rect::from_min_size(
            Pos2::new(x, y),
            Vec2::new(width, height),
        );

        if !clip_rect.contains(pos) {
            return None;
        }

        // Check if near edges for resizing
        let edge_threshold = 5.0;
        if (pos.x - clip_rect.min.x) < edge_threshold {
            Some(ClipHitRegion::LeftEdge)
        } else if (clip_rect.max.x - pos.x) < edge_threshold {
            Some(ClipHitRegion::RightEdge)
        } else {
            Some(ClipHitRegion::Body)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipHitRegion {
    Body,
    LeftEdge,
    RightEdge,
}
