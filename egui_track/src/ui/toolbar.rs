//! Toolbar module
//!
//! Contains the toolbar at the top of the timeline, including playback controls, time display, time signature, and BPM.
//! 参考 MIDI 编辑器的工具栏设计

use crate::structure::TimelineState;
use crate::editor::TrackEditorCommand;
use egui::*;

pub struct Toolbar {
    timeline: TimelineState,
    metronome_enabled: bool,
    is_playing: bool,
    current_time: f64,
}

impl Toolbar {
    pub fn new(timeline: &TimelineState) -> Self {
        Self {
            timeline: timeline.clone(),
            metronome_enabled: false,
            is_playing: false,
            current_time: 0.0,
        }
    }

    #[allow(dead_code)]
    pub fn set_timeline(&mut self, timeline: &TimelineState) {
        self.timeline = timeline.clone();
    }

    pub fn set_metronome(&mut self, enabled: bool) {
        self.metronome_enabled = enabled;
    }

    pub fn set_playing(&mut self, playing: bool) {
        self.is_playing = playing;
    }

    pub fn set_current_time(&mut self, time: f64) {
        self.current_time = time;
    }

    pub fn ui(&mut self, ui: &mut Ui, command_callback: &mut dyn FnMut(TrackEditorCommand)) {
        // 水平布局（与 MIDI 编辑器一致）
        ui.horizontal(|ui| {
            // Time display
            let total_seconds = self.current_time;
            let minutes = (total_seconds / 60.0) as u32;
            let seconds = (total_seconds % 60.0) as u32;
            let milliseconds = ((total_seconds % 1.0) * 1000.0) as u32;
            let time_display = format!("{:02}:{:02}.{:03}", minutes, seconds, milliseconds);
            ui.label(format!("Time: {}", time_display));
            ui.separator();

            // Playback controls
            if ui
                .button(if self.is_playing {
                    "⏸ Pause"
                } else {
                    "▶ Play"
                })
                .clicked()
            {
                command_callback(TrackEditorCommand::SetPlayback {
                    is_playing: !self.is_playing,
                });
            }
            if ui.button("⏹ Stop").clicked() {
                command_callback(TrackEditorCommand::StopPlayback);
            }

            ui.separator();

            // Undo/Redo buttons (占位，需要实现撤销/重做功能)
            if ui
                .add_enabled(false, Button::new("↺"))
                .clicked()
            {
                // TODO: 实现撤销
            }
            if ui
                .add_enabled(false, Button::new("↻"))
                .clicked()
            {
                // TODO: 实现重做
            }

            ui.separator();

            // Time signature (与 MIDI 编辑器一致)
            ui.label("Sig:");
            ui.horizontal(|ui| {
                let mut numer = self.timeline.time_signature.0;
                let mut denom = self.timeline.time_signature.1;
                let numer_changed = ui
                    .add(DragValue::new(&mut numer).speed(0.1).range(1..=32))
                    .changed();
                ui.label("/");
                let denom_changed = ui
                    .add(DragValue::new(&mut denom).speed(0.1).range(1..=32))
                    .changed();
                if numer_changed || denom_changed {
                    command_callback(TrackEditorCommand::SetTimeSignature { 
                        numer, 
                        denom 
                    });
                }
            });

            ui.separator();

            // BPM (与 MIDI 编辑器一致)
            ui.label("BPM:");
            let mut bpm = self.timeline.bpm;
            if ui
                .add(DragValue::new(&mut bpm).speed(1.0).range(20.0..=400.0))
                .changed()
            {
                command_callback(TrackEditorCommand::SetBPM { bpm });
            }

            ui.separator();

            // Position display (小节:拍格式)
            ui.horizontal(|ui| {
                ui.label("Position:");
                let current_beat = self.current_time * self.timeline.bpm as f64 / 60.0;
                let current_measure = (current_beat / self.timeline.time_signature.0 as f64).floor() + 1.0;
                let beat_in_measure = (current_beat % self.timeline.time_signature.0 as f64) + 1.0;
                ui.label(format!(
                    "{:.2}s ({:.1}:{:.1})",
                    self.current_time,
                    current_measure,
                    beat_in_measure
                ));
            });

            ui.separator();

            // Metronome toggle
            let mut metronome = self.metronome_enabled;
            if ui.checkbox(&mut metronome, "Metronome").changed() {
                command_callback(TrackEditorCommand::SetMetronome { enabled: metronome });
            }
        });
    }
}
