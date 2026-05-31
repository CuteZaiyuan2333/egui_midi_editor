use eframe::egui;

fn draw_block<F>(ui: &mut egui::Ui, mut callback: F) 
    where 
        F: FnMut(&mut egui::Ui, f32, f32),{
    ui
}