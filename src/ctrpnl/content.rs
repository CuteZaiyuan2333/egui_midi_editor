use std::sync::Arc;

use eframe::egui;
use egui_dock::{
    DockArea,
    DockState,
    NodeIndex,
};
use egui_midi::{audio::{AudioEngine, PlaybackBackend}, ui::MidiEditor};
use crate::ctrpnl::tab::{self, MyTabViewer};

pub struct Ctrpnl{
    dock_state: DockState<tab::Mytab>,
    viewer: MyTabViewer,
}impl Ctrpnl{
    pub fn init() -> Self{
        let mut dock_state = DockState::new(vec![tab::Mytab::Track]);
        dock_state.main_surface_mut().split_below(
            NodeIndex::root(),
            0.8,
            vec![tab::Mytab::PianoRoll],
        );
        let audio: Arc<dyn PlaybackBackend> = Arc::new(AudioEngine::new());
        Ctrpnl{
            dock_state,
            viewer: MyTabViewer{
                debugmidi: MidiEditor::new(Some(audio))
            },
        }
    }
}

impl Ctrpnl{
    pub fn show(self: &mut Self, ui: &mut egui::Ui){
        //ui.label("place-holder(track-editor)");
        DockArea::new(&mut self.dock_state)
            .show_inside(ui, &mut self.viewer);
    }
}


