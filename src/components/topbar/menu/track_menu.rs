use eframe::egui;

pub fn draw_track_menu(ui: &mut egui::Ui){
    ui.menu_button("track", |ui|{
        if ui.button("add midi track").clicked(){
            
        }
        if ui.button("add audio track").clicked(){
            
        }
        if ui.button("add sample track").clicked(){
            
        }
    });
}