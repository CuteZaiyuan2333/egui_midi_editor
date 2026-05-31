use eframe::egui;
use egui_dock::{
    DockArea, DockState, NodeIndex, Tree, dock_area, dock_state
};
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
        Ctrpnl{
            dock_state,
            viewer: MyTabViewer,
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


