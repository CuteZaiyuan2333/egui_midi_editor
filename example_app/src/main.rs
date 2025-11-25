use eframe::egui;
use egui_midi::audio::{AudioEngine, PlaybackBackend};
use egui_midi::structure::Note;
use egui_midi::ui::MidiEditor;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    env_logger::init(); // For logging if needed

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "egui MIDI Editor Example",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

struct MyApp {
    editor: MidiEditor,
}

impl MyApp {
    fn new() -> Self {
        // Initialize Audio
        // Note: In a real app you might want to handle errors gracefully
        let audio: Arc<dyn PlaybackBackend> = Arc::new(AudioEngine::new());

        let mut editor = MidiEditor::new(Some(audio));

        // Add some dummy notes (C Major Chord Arpeggio)
        // Assuming 480 ticks per beat
        editor.insert_note(Note::new(0, 480, 60, 100)); // C4
        editor.insert_note(Note::new(480, 480, 64, 100)); // E4
        editor.insert_note(Note::new(960, 480, 67, 100)); // G4
        editor.insert_note(Note::new(1440, 1920, 72, 100)); // C5

        editor.center_on_c4();
        
        Self {
            editor,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.editor.ui(ui);
        });

        for event in self.editor.take_events() {
            log::info!("[EditorEvent] {:?}", event);
        }
    }
}
