use eframe::egui;

pub fn draw_edit_menu(ui: &mut egui::Ui){
    ui.menu_button("edit", |ui|{
        if ui.button("undo").clicked(){
            
        }
        if ui.button("redo").clicked(){
            
        }
        ui.separator();
        if ui.button("preferance").clicked(){
            
        }
    });
}