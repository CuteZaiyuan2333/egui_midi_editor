use eframe::egui;
use egui_track::{TrackEditor, TrackEditorOptions, ClipType};

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "egui_track Example",
        native_options,
        Box::new(|_cc| Ok(Box::new(TrackEditorApp::new()))),
    )
}

struct TrackEditorApp {
    editor: TrackEditor,
}

impl TrackEditorApp {
    fn new() -> Self {
        let options = TrackEditorOptions::default();
        let mut editor = TrackEditor::new(options);

        // Create some example tracks and clips
        editor.execute_command(egui_track::TrackEditorCommand::CreateTrack {
            name: "Track 1".to_string(),
        });
        editor.execute_command(egui_track::TrackEditorCommand::CreateTrack {
            name: "Track 2".to_string(),
        });
        editor.execute_command(egui_track::TrackEditorCommand::CreateTrack {
            name: "Track 3".to_string(),
        });

        // Add some MIDI clips
        let track1_id = editor.tracks().first().map(|t| t.id);
        if let Some(track_id) = track1_id {
            editor.execute_command(egui_track::TrackEditorCommand::CreateClip {
                track_id,
                start: 0.0,
                duration: 2.0,
                clip_type: ClipType::Midi { midi_data: None },
            });
            editor.execute_command(egui_track::TrackEditorCommand::CreateClip {
                track_id,
                start: 3.0,
                duration: 1.5,
                clip_type: ClipType::Midi { midi_data: None },
            });
        }

        // Add some audio clips
        let track2_id = editor.tracks().get(1).map(|t| t.id);
        if let Some(track_id) = track2_id {
            editor.execute_command(egui_track::TrackEditorCommand::CreateClip {
                track_id,
                start: 1.0,
                duration: 2.5,
                clip_type: ClipType::Audio { audio_data: None },
            });
            editor.execute_command(egui_track::TrackEditorCommand::CreateClip {
                track_id,
                start: 4.5,
                duration: 1.0,
                clip_type: ClipType::Audio { audio_data: None },
            });
        }

        // Add mixed clips to third track
        let track3_id = editor.tracks().get(2).map(|t| t.id);
        if let Some(track_id) = track3_id {
            editor.execute_command(egui_track::TrackEditorCommand::CreateClip {
                track_id,
                start: 0.5,
                duration: 1.0,
                clip_type: ClipType::Midi { midi_data: None },
            });
            editor.execute_command(egui_track::TrackEditorCommand::CreateClip {
                track_id,
                start: 2.0,
                duration: 2.0,
                clip_type: ClipType::Audio { audio_data: None },
            });
        }

        Self { editor }
    }
}

impl eframe::App for TrackEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.editor.ui(ui);
        });

        // Handle events
        for event in self.editor.take_events() {
            log::info!("[TrackEditorEvent] {:?}", event);
        }
    }
}
