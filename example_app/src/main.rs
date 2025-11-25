use eframe::egui::{self, TopBottomPanel};
use egui_midi::audio::{AudioEngine, PlaybackBackend};
use egui_midi::structure::{MidiState, Note};
use egui_midi::ui::MidiEditor;
use midly::Smf;
use rfd::FileDialog;
use std::fs;
use std::path::{Path, PathBuf};
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
    current_path: Option<PathBuf>,
    status_line: String,
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
            current_path: None,
            status_line: "Ready".to_owned(),
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    self.new_project();
                    ui.close_menu();
                }
                if ui.button("Open...").clicked() {
                    self.open_project_dialog();
                    ui.close_menu();
                }
                if ui.button("Save").clicked() {
                    self.save_project();
                    ui.close_menu();
                }
                if ui.button("Save As...").clicked() {
                    self.save_project_as_dialog();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export MIDI...").clicked() {
                    self.export_midi_dialog();
                    ui.close_menu();
                }
            });

            if let Some(path) = &self.current_path {
                ui.label(format!(" Project: {}", path.display()));
            } else {
                ui.label(" Project: (unsaved)");
            }
        });
    }

    fn new_project(&mut self) {
        self.editor.replace_state(MidiState::default());
        self.current_path = None;
        self.set_status("Created new project");
    }

    fn open_project_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("AquaMIDI Project", &["aquamidi"])
            .pick_file()
        {
            match read_aquamidi_file(&path) {
                Ok(state) => {
                    self.editor.replace_state(state);
                    self.current_path = Some(path.clone());
                    self.set_status(format!("Opened {}", path.display()));
                }
                Err(err) => self.set_error(err),
            }
        }
    }

    fn save_project(&mut self) {
        if let Some(path) = self.current_path.clone() {
            match write_aquamidi_file(&path, &self.editor.snapshot_state()) {
                Ok(_) => self.set_status(format!("Saved {}", path.display())),
                Err(err) => self.set_error(err),
            }
        } else {
            self.save_project_as_dialog();
        }
    }

    fn save_project_as_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("AquaMIDI Project", &["aquamidi"])
            .set_file_name(self.default_file_name("aquamidi"))
            .save_file()
        {
            match write_aquamidi_file(&path, &self.editor.snapshot_state()) {
                Ok(_) => {
                    self.current_path = Some(path.clone());
                    self.set_status(format!("Saved {}", path.display()));
                }
                Err(err) => self.set_error(err),
            }
        }
    }

    fn export_midi_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Standard MIDI", &["mid", "midi"])
            .set_file_name(self.default_file_name("mid"))
            .save_file()
        {
            match export_midi_file(&path, &self.editor.snapshot_state()) {
                Ok(_) => self.set_status(format!("Exported {}", path.display())),
                Err(err) => self.set_error(err),
            }
        }
    }

    fn default_file_name(&self, extension: &str) -> String {
        self.current_path
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .map(|name| format!("{name}.{extension}"))
            .unwrap_or_else(|| format!("project.{extension}"))
    }

    fn set_status<S: Into<String>>(&mut self, msg: S) {
        self.status_line = msg.into();
        log::info!("{}", self.status_line);
    }

    fn set_error<E: Into<String>>(&mut self, err: E) {
        let msg = err.into();
        self.status_line = format!("Error: {msg}");
        log::error!("{msg}");
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.menu_bar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.editor.ui(ui);
        });

        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label(self.status_line.clone());
        });

        for event in self.editor.take_events() {
            log::info!("[EditorEvent] {:?}", event);
        }
    }
}

const AQUAMIDI_MAGIC: &[u8; 8] = b"AQUAMIDI";
const AQUAMIDI_VERSION: u32 = 1;

fn write_aquamidi_file(path: &Path, state: &MidiState) -> Result<(), String> {
    let mut buffer = Vec::new();
    buffer.extend_from_slice(AQUAMIDI_MAGIC);
    buffer.extend_from_slice(&AQUAMIDI_VERSION.to_le_bytes());
    let smf = state
        .to_single_track_smf()
        .map_err(|err| format!("Export error: {err}"))?;
    smf.write_std(&mut buffer)
        .map_err(|err| format!("Failed to encode project: {err}"))?;
    fs::write(path, buffer).map_err(|err| format!("Failed to write {}: {err}", path.display()))
}

fn read_aquamidi_file(path: &Path) -> Result<MidiState, String> {
    let data = fs::read(path).map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    if data.len() < AQUAMIDI_MAGIC.len() + 4 {
        return Err("File is too small to be a valid AquaMIDI project".into());
    }
    if &data[..AQUAMIDI_MAGIC.len()] != AQUAMIDI_MAGIC {
        return Err("File is not in AquaMIDI format".into());
    }
    let mut version_bytes = [0u8; 4];
    version_bytes.copy_from_slice(&data[AQUAMIDI_MAGIC.len()..AQUAMIDI_MAGIC.len() + 4]);
    let version = u32::from_le_bytes(version_bytes);
    if version > AQUAMIDI_VERSION {
        return Err(format!("Unsupported AquaMIDI version {version}"));
    }
    let smf_bytes = &data[AQUAMIDI_MAGIC.len() + 4..];
    let smf = Smf::parse(smf_bytes).map_err(|err| format!("Failed to parse SMF: {err}"))?;
    MidiState::from_smf_strict(&smf).map_err(|err| format!("Invalid MIDI data: {err}"))
}

fn export_midi_file(path: &Path, state: &MidiState) -> Result<(), String> {
    let smf = state
        .to_single_track_smf()
        .map_err(|err| format!("Export error: {err}"))?;
    let mut buffer = Vec::new();
    smf.write_std(&mut buffer)
        .map_err(|err| format!("Failed to encode SMF: {err}"))?;
    fs::write(path, buffer).map_err(|err| format!("Failed to write {}: {err}", path.display()))
}
