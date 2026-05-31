use eframe::egui;
use chrono::Local;

pub struct Btmbar{
    
}impl Btmbar{
    pub fn init() -> Self{
        Btmbar{
            
        }
    }
}

impl Btmbar{
    pub fn show(self: &mut Self, ui: &mut egui::Ui){
        ui.horizontal(|ui|{
            ui.label("Free and open-sourced under MIT license.");
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui|{
                ui.label(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                ui.separator();
            });
        });
    }
}