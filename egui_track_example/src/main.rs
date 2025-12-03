use eframe::egui;
use egui_track::{TrackEditor, TrackEditorOptions, ClipType, ProjectFile, format_time};
use std::path::PathBuf;
use rfd::FileDialog;

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
    current_project_path: Option<PathBuf>,
}

impl TrackEditorApp {
    fn new() -> Self {
        let options = TrackEditorOptions::default();
        let editor = TrackEditor::new(options);

        Self { 
            editor,
            current_project_path: None,
        }
    }

    #[allow(dead_code)]
    fn load_project(&mut self, path: &PathBuf) {
        match ProjectFile::load_from_path(path) {
            Ok(project_file) => {
                // Restore editor state
                // Note: This requires methods to set the editor's internal state
                // For now, just log the information
                log::info!("Project loaded: {:?}", path);
                log::info!("Track count: {}", project_file.tracks.len());
                self.current_project_path = Some(path.clone());
                // TODO: Implement state restoration when TrackEditor provides the necessary methods
            }
            Err(e) => {
                log::error!("Failed to load project: {}", e);
            }
        }
    }

    fn new_project(&mut self) {
        // Clear editor and reset project path
        // Note: This requires methods to clear the editor's state
        // For now, create a new editor instance
        let options = egui_track::TrackEditorOptions::default();
        self.editor = egui_track::TrackEditor::new(options);
        self.current_project_path = None;
        log::info!("New project created");
    }

    fn open_project(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Track Project", &["json"])
            .set_title("Open Project")
            .pick_file()
        {
            self.load_project(&path);
        }
    }

    fn save_project(&mut self) {
        if let Some(path) = self.current_project_path.clone() {
            self.save_project_to_path(&path);
        } else {
            // No current path, need to save as
            self.save_project_as();
        }
    }

    fn save_project_as(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Track Project", &["json"])
            .set_title("Save Project As")
            .set_file_name("project.json")
            .save_file()
        {
            self.save_project_to_path(&path);
        }
    }

    fn save_project_to_path(&mut self, path: &PathBuf) {
        let project_file = ProjectFile::new(
            self.editor.timeline().clone(),
            self.editor.tracks().to_vec(),
        );
        
        match project_file.save_to_path(path) {
            Ok(_) => {
                self.current_project_path = Some(path.clone());
                log::info!("Project saved to: {:?}", path);
            }
            Err(e) => {
                log::error!("Failed to save project: {}", e);
            }
        }
    }

    fn export_project(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Track Project", &["json"])
            .set_title("Export Project")
            .set_file_name("export.json")
            .save_file()
        {
            // For now, export is the same as save
            self.save_project_to_path(&path);
            log::info!("Project exported to: {:?}", path);
        }
    }
}

impl eframe::App for TrackEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_project();
                        ui.close_menu();
                    }
                    if ui.button("Open").clicked() {
                        self.open_project();
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_project();
                        ui.close_menu();
                    }
                    if ui.button("Save As").clicked() {
                        self.save_project_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export").clicked() {
                        self.export_project();
                        ui.close_menu();
                    }
                });
            });
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Project name/path
                if let Some(ref path) = self.current_project_path {
                    let project_name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown");
                    ui.label(format!("Project: {}", project_name));
                } else {
                    ui.label("Project: Unsaved");
                }

                ui.separator();

                // Track count
                ui.label(format!("Tracks: {}", self.editor.tracks().len()));

                ui.separator();

                // Clip count
                let total_clips: usize = self.editor.tracks().iter()
                    .map(|t| t.clips.len())
                    .sum();
                ui.label(format!("Clips: {}", total_clips));

                ui.separator();

                // Playhead position
                let pos = self.editor.timeline().playhead_position;
                ui.label(format!("Position: {}", format_time(pos)));
            });
        });

        // Central panel with track editor
        egui::CentralPanel::default().show(ctx, |ui| {
            self.editor.ui(ui);
        });

        // Handle events
        for event in self.editor.take_events() {
            log::info!("[TrackEditorEvent] {:?}", event);
        }
    }
}
