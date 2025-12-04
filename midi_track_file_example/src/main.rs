use eframe::egui;
use egui_track::{TrackEditor, TrackEditorOptions, ProjectFile, format_time};
use egui_midi::{ui::MidiEditor, audio::{AudioEngine, PlaybackBackend}, structure::MidiState};
use egui_file_tree::{FileTree, FileTreeEvent};
use std::path::PathBuf;
use std::sync::Arc;
use rfd::FileDialog;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "MIDI & Track & File Example",
        native_options,
        Box::new(|_cc| Ok(Box::new(MidiTrackFileApp::new()))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TopTab {
    TrackEditor,
    OtherTools,
}

struct MidiEditorTab {
    #[allow(dead_code)]
    id: usize,
    name: String,
    editor: MidiEditor,
}

impl MidiEditorTab {
    fn new(id: usize, name: String, audio: Arc<dyn PlaybackBackend>) -> Self {
        Self {
            id,
            name,
            editor: MidiEditor::new(Some(audio)),
        }
    }
}

struct MidiTrackFileApp {
    // Top tabs
    top_active_tab: TopTab,
    
    // Track editor
    track_editor: TrackEditor,
    current_project_path: Option<PathBuf>,
    
    // MIDI editors
    midi_editors: Vec<MidiEditorTab>,
    active_midi_tab: Option<usize>,
    next_midi_tab_id: usize,
    
    // Shared audio engine for MIDI editors
    audio_engine: Arc<dyn PlaybackBackend>,
    
    // File tree
    file_tree: Option<FileTree>,
    
    // Splitter states
    vertical_split_ratio: f32,  // Ratio for top/bottom split (0.0-1.0)
    horizontal_split_ratio: f32,  // Ratio for left/right split in bottom area (0.0-1.0)
    dragging_vertical_splitter: bool,
    dragging_horizontal_splitter: bool,
}

impl MidiTrackFileApp {
    fn new() -> Self {
        let options = TrackEditorOptions::default();
        let track_editor = TrackEditor::new(options);
        
        let audio: Arc<dyn PlaybackBackend> = Arc::new(AudioEngine::new());
        
        // Initialize file tree with current directory
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let file_tree = Some(FileTree::new(current_dir));
        
        Self {
            top_active_tab: TopTab::TrackEditor,
            track_editor,
            current_project_path: None,
            midi_editors: Vec::new(),
            active_midi_tab: None,
            next_midi_tab_id: 0,
            audio_engine: audio,
            file_tree,
            vertical_split_ratio: 0.5,  // 50% for top, 50% for bottom
            horizontal_split_ratio: 0.2,  // 20% for file tree, 80% for MIDI editors
            dragging_vertical_splitter: false,
            dragging_horizontal_splitter: false,
        }
    }

    fn add_midi_editor(&mut self) {
        let id = self.next_midi_tab_id;
        self.next_midi_tab_id += 1;
        let name = format!("MIDI {}", id + 1);
        let tab = MidiEditorTab::new(id, name, Arc::clone(&self.audio_engine));
        self.midi_editors.push(tab);
        self.active_midi_tab = Some(self.midi_editors.len() - 1);
    }

    fn close_midi_editor(&mut self, index: usize) {
        if index < self.midi_editors.len() {
            self.midi_editors.remove(index);
            if self.midi_editors.is_empty() {
                self.active_midi_tab = None;
            } else {
                // Adjust active tab index
                if let Some(active) = self.active_midi_tab {
                    if active >= index {
                        if active > 0 {
                            self.active_midi_tab = Some(active - 1);
                        } else {
                            self.active_midi_tab = Some(0);
                        }
                    }
                }
            }
        }
    }

    fn load_project(&mut self, path: &PathBuf) {
        match ProjectFile::load_from_path(path) {
            Ok(project_file) => {
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
        let options = egui_track::TrackEditorOptions::default();
        self.track_editor = egui_track::TrackEditor::new(options);
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
            self.track_editor.timeline().clone(),
            self.track_editor.tracks().to_vec(),
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
            self.save_project_to_path(&path);
            log::info!("Project exported to: {:?}", path);
        }
    }

    fn open_directory(&mut self) {
        if let Some(path) = FileDialog::new()
            .set_title("Select Directory")
            .pick_folder()
        {
            self.file_tree = Some(FileTree::new(path));
            log::info!("Opened directory: {:?}", self.file_tree.as_ref().unwrap().root_path());
        }
    }

    fn open_midi_file(&mut self, path: &PathBuf) {
        match std::fs::read(path) {
            Ok(data) => {
                match midly::Smf::parse(&data) {
                    Ok(smf) => {
                        match MidiState::from_smf_strict(&smf) {
                            Ok(state) => {
                                // Create a new MIDI editor tab with the loaded state
                                let id = self.next_midi_tab_id;
                                self.next_midi_tab_id += 1;
                                let file_name = path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("MIDI")
                                    .to_string();
                                let name = format!("{}", file_name);
                                
                                let mut tab = MidiEditorTab::new(id, name, Arc::clone(&self.audio_engine));
                                tab.editor.replace_state(state);
                                self.midi_editors.push(tab);
                                self.active_midi_tab = Some(self.midi_editors.len() - 1);
                                log::info!("Opened MIDI file: {:?}", path);
                            }
                            Err(e) => {
                                log::error!("Failed to parse MIDI file: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to parse MIDI file: {:?}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to read file: {:?}", e);
            }
        }
    }
}

impl eframe::App for MidiTrackFileApp {
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
                
                ui.menu_button("MIDI", |ui| {
                    if ui.button("New MIDI Editor").clicked() {
                        self.add_midi_editor();
                        ui.close_menu();
                    }
                });
            });
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
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

                ui.label(format!("Tracks: {}", self.track_editor.tracks().len()));

                ui.separator();

                let total_clips: usize = self.track_editor.tracks().iter()
                    .map(|t| t.clips.len())
                    .sum();
                ui.label(format!("Clips: {}", total_clips));

                ui.separator();

                let pos = self.track_editor.timeline().playhead_position;
                ui.label(format!("Position: {}", format_time(pos)));
            });
        });

        // Main content area - split into top and bottom
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_rect = ui.available_rect_before_wrap();
            let min_top_height = 150.0;
            let min_bottom_height = 150.0;
            
            // Calculate top and bottom heights based on split ratio
            let top_height = (available_rect.height() * self.vertical_split_ratio)
                .max(min_top_height)
                .min(available_rect.height() - min_bottom_height);
            let bottom_height = available_rect.height() - top_height;
            
            ui.vertical(|ui| {
                // Top section: Tabs for track editor and other tools
                let top_rect = ui.allocate_ui_with_layout(
                    egui::Vec2::new(available_rect.width(), top_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        // Tab bar
                        ui.horizontal(|ui| {
                            let track_selected = self.top_active_tab == TopTab::TrackEditor;
                            if ui.selectable_label(track_selected, "Track Editor").clicked() {
                                self.top_active_tab = TopTab::TrackEditor;
                            }
                            
                            let other_selected = self.top_active_tab == TopTab::OtherTools;
                            if ui.selectable_label(other_selected, "Other Tools").clicked() {
                                self.top_active_tab = TopTab::OtherTools;
                            }
                        });
                        
                        ui.separator();
                        
                        // Tab content
                        let content_rect = ui.available_rect_before_wrap();
                        ui.allocate_ui(content_rect.size(), |ui| {
                            match self.top_active_tab {
                                TopTab::TrackEditor => {
                                    self.track_editor.ui(ui);
                                }
                                TopTab::OtherTools => {
                                    ui.centered_and_justified(|ui| {
                                        ui.label("Other Tools Tab (Placeholder)");
                                        ui.label("Additional tools and features can be added here");
                                    });
                                }
                            }
                        });
                    }
                ).response.rect;
                
                // Vertical splitter (between top and bottom)
                let splitter_height = 4.0;
                let splitter_rect = egui::Rect::from_min_size(
                    egui::pos2(top_rect.min.x, top_rect.max.y),
                    egui::Vec2::new(available_rect.width(), splitter_height)
                );
                let splitter_response = ui.allocate_rect(splitter_rect, egui::Sense::click_and_drag());
                
                // Use default separator style
                ui.painter().line_segment(
                    [egui::pos2(available_rect.min.x, splitter_rect.center().y),
                     egui::pos2(available_rect.max.x, splitter_rect.center().y)],
                    ui.style().visuals.widgets.noninteractive.bg_stroke
                );
                
                // Handle vertical splitter dragging
                if splitter_response.drag_started() {
                    self.dragging_vertical_splitter = true;
                }
                if self.dragging_vertical_splitter {
                    if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                        let new_ratio = ((pointer.y - available_rect.min.y) / available_rect.height())
                            .clamp(min_top_height / available_rect.height(), 1.0 - min_bottom_height / available_rect.height());
                        self.vertical_split_ratio = new_ratio;
                    }
                    if ui.input(|i| i.pointer.any_released()) {
                        self.dragging_vertical_splitter = false;
                    }
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                } else if splitter_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }
                
                // Bottom section: File tree (left) + MIDI editors (right)
                let min_file_tree_width = 150.0;
                let min_midi_editor_width = 200.0;
                let file_tree_width = (available_rect.width() * self.horizontal_split_ratio)
                    .max(min_file_tree_width)
                    .min(available_rect.width() - min_midi_editor_width);
                
                let _bottom_rect = ui.allocate_ui_with_layout(
                    egui::Vec2::new(available_rect.width(), bottom_height),
                    egui::Layout::left_to_right(egui::Align::TOP),
                    |ui| {
                        // Left: File tree panel
                        let file_tree_rect = ui.allocate_ui_with_layout(
                            egui::Vec2::new(file_tree_width, bottom_height),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                if let Some(ref mut file_tree) = self.file_tree {
                                    let events = file_tree.ui(ui);
                                    
                                    // Handle file tree events
                                    for event in events {
                                        match event {
                                            FileTreeEvent::PathSelected { path } => {
                                                log::info!("File selected: {:?}", path);
                                            }
                                            FileTreeEvent::PathDoubleClicked { path } => {
                                                // Check if it's a MIDI file
                                                if let Some(ext) = path.extension() {
                                                    if ext == "mid" || ext == "midi" {
                                                        self.open_midi_file(&path);
                                                    }
                                                }
                                                log::info!("File double clicked: {:?}", path);
                                            }
                                            FileTreeEvent::NavigateToParent => {
                                                if let Some(ref mut file_tree) = self.file_tree {
                                                    if let Some(parent) = file_tree.root_path().parent() {
                                                        let parent_path = parent.to_path_buf();
                                                        file_tree.set_root_path(parent_path.clone());
                                                        log::info!("Navigated to parent directory: {:?}", parent_path);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.label("No directory opened");
                                        ui.add_space(10.0);
                                        if ui.button("Open Directory").clicked() {
                                            self.open_directory();
                                        }
                                    });
                                }
                            }
                        ).response.rect;
                        
                        // Horizontal splitter (between file tree and MIDI editors)
                        let splitter_width = 4.0;
                        let splitter_rect = egui::Rect::from_min_size(
                            egui::pos2(file_tree_rect.max.x, file_tree_rect.min.y),
                            egui::Vec2::new(splitter_width, bottom_height)
                        );
                        let splitter_response = ui.allocate_rect(splitter_rect, egui::Sense::click_and_drag());
                        
                        // Use default separator style
                        ui.painter().line_segment(
                            [egui::pos2(splitter_rect.center().x, splitter_rect.min.y),
                             egui::pos2(splitter_rect.center().x, splitter_rect.max.y)],
                            ui.style().visuals.widgets.noninteractive.bg_stroke
                        );
                        
                        // Handle horizontal splitter dragging
                        if splitter_response.drag_started() {
                            self.dragging_horizontal_splitter = true;
                        }
                        if self.dragging_horizontal_splitter {
                            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                                let new_ratio = ((pointer.x - available_rect.min.x) / available_rect.width())
                                    .clamp(min_file_tree_width / available_rect.width(), 1.0 - min_midi_editor_width / available_rect.width());
                                self.horizontal_split_ratio = new_ratio;
                            }
                            if ui.input(|i| i.pointer.any_released()) {
                                self.dragging_horizontal_splitter = false;
                            }
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        } else if splitter_response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }
                        
                        // Right: MIDI editors tabs
                        ui.vertical(|ui| {
                            // MIDI editor tab bar
                            if !self.midi_editors.is_empty() {
                                ui.horizontal(|ui| {
                                    let mut to_remove: Option<usize> = None;
                                    
                                    for (index, tab) in self.midi_editors.iter().enumerate() {
                                        let is_active = self.active_midi_tab == Some(index);
                                        
                                        // Tab button with close button
                                        ui.horizontal(|ui| {
                                            if ui.selectable_label(is_active, &tab.name).clicked() {
                                                self.active_midi_tab = Some(index);
                                            }
                                            
                                            // Close button
                                            if ui.small_button("✕").clicked() {
                                                to_remove = Some(index);
                                            }
                                        });
                                    }
                                    
                                    // Add new MIDI editor button
                                    if ui.button("+").clicked() {
                                        self.add_midi_editor();
                                    }
                                    
                                    // Remove tab if needed
                                    if let Some(index) = to_remove {
                                        self.close_midi_editor(index);
                                    }
                                });
                                
                                ui.separator();
                                
                                // Active MIDI editor content
                                if let Some(active_index) = self.active_midi_tab {
                                    if let Some(tab) = self.midi_editors.get_mut(active_index) {
                                        let content_rect = ui.available_rect_before_wrap();
                                        ui.allocate_ui(content_rect.size(), |ui| {
                                            tab.editor.ui(ui);
                                        });
                                    }
                                }
                            } else {
                                // No MIDI editors, show placeholder
                                ui.centered_and_justified(|ui| {
                                    ui.label("No MIDI editors open");
                                    ui.add_space(10.0);
                                    if ui.button("New MIDI Editor").clicked() {
                                        self.add_midi_editor();
                                    }
                                });
                            }
                        });
                    }
                ).response.rect;
            });
        });

        // Handle track editor events
        for event in self.track_editor.take_events() {
            log::info!("[TrackEditorEvent] {:?}", event);
        }
        
        // Handle MIDI editor events
        for tab in &mut self.midi_editors {
            for event in tab.editor.take_events() {
                log::info!("[MidiEditorEvent] {:?}", event);
            }
        }
    }
}
