use crate::structure::Track;
use egui::*;

pub struct TrackLaneHeader {
    track: Track,
    width: f32,
}

impl TrackLaneHeader {
    pub fn new(track: &Track, width: f32) -> Self {
        Self {
            track: track.clone(),
            width,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.allocate_ui_with_layout(
            Vec2::new(self.width, self.track.height),
            Layout::top_down(Align::LEFT),
            |ui| {
                ui.horizontal(|ui| {
                    // Mute button
                    let mute_response = ui.selectable_label(self.track.muted, "M");
                    if mute_response.clicked() {
                        // This would need to be handled by parent
                    }

                    // Solo button
                    let solo_response = ui.selectable_label(self.track.solo, "S");
                    if solo_response.clicked() {
                        // This would need to be handled by parent
                    }

                    // Track name
                    ui.label(&self.track.name);
                });

                // Volume slider
                ui.add_space(4.0);
                let mut volume = self.track.volume;
                let _volume_slider = ui.add(Slider::new(&mut volume, 0.0..=1.0).text("Vol"));
                // Volume changes would need to be handled by parent
            },
        );
    }
}
