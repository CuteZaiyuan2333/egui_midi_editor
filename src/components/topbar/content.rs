use eframe::egui;

use crate::components::topbar::menu::{
    edit_menu::draw_edit_menu,
    file_menu::draw_file_menu,
    track_menu::draw_track_menu,
    window_menu::draw_window_menu
};

pub struct Topbar{
    //empty
}impl Topbar{
    pub fn init() -> Self{
        Topbar{
            
        }
    }
}

impl Topbar{
    pub fn show(self:&mut Self, ui: &mut egui::Ui){
        ui.horizontal(|ui|{
            draw_file_menu(ui);
            draw_edit_menu(ui);
            draw_track_menu(ui);
            draw_window_menu(ui);
        });
    }
}