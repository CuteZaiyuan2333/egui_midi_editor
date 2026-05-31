use eframe::egui;

pub fn draw_window_menu(ui: &mut egui::Ui){
    ui.menu_button("window", |ui|{
        if ui.button("open track editor").clicked(){
            
        }
        ui.separator();
        if ui.button("open piano-roll editor").clicked(){
            
        }
        if ui.button("open sample editor").clicked(){
            
        }
        ui.separator();
        if ui.button("open wave inspector").clicked(){
            
        }
        ui.separator();
        if ui.button("open blueprint editor").clicked(){
            
        }
    });
}