//! Status bar module
//!
//! Displays bottom information bar, containing project information, track count, clip count, and other status information.

use egui::*;

#[allow(dead_code)]
pub struct StatusBar {
    project_name: Option<String>,
    track_count: usize,
    clip_count: usize,
    playhead_position: f64,
}

impl StatusBar {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            project_name: None,
            track_count: 0,
            clip_count: 0,
            playhead_position: 0.0,
        }
    }

    #[allow(dead_code)]
    pub fn set_project_name(&mut self, name: Option<String>) {
        self.project_name = name;
    }

    #[allow(dead_code)]
    pub fn set_track_count(&mut self, count: usize) {
        self.track_count = count;
    }

    #[allow(dead_code)]
    pub fn set_clip_count(&mut self, count: usize) {
        self.clip_count = count;
    }

    #[allow(dead_code)]
    pub fn set_playhead_position(&mut self, position: f64) {
        self.playhead_position = position;
    }

    #[allow(dead_code)]
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Project name/path
            if let Some(ref name) = self.project_name {
                ui.label(format!("Project: {}", name));
            } else {
                ui.label("Project: Unsaved");
            }

            ui.separator();

            // Track count
            ui.label(format!("Tracks: {}", self.track_count));

            ui.separator();

            // Clip count
            ui.label(format!("Clips: {}", self.clip_count));

            ui.separator();

            // Playhead position
            let minutes = (self.playhead_position / 60.0) as u32;
            let seconds = (self.playhead_position % 60.0) as u32;
            let milliseconds = ((self.playhead_position % 1.0) * 1000.0) as u32;
            ui.label(format!(
                "Position: {:02}:{:02}.{:03}",
                minutes, seconds, milliseconds
            ));
        });
    }
}
