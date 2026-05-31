use eframe::egui;

pub fn draw_file_menu(ui: &mut egui::Ui){
    ui.menu_button("file", |ui|{
        if ui.button("open file").clicked(){
            
        }
        if ui.button("open project").clicked(){
            
        }
        ui.separator();
        if ui.button("save").clicked(){
            
        }
        if ui.button("save as project").clicked(){
            
        }
        if ui.button("save track as file").clicked(){
            
        }
    });
}