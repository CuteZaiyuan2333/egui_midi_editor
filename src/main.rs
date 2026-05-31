use eframe::egui;
mod components;
mod ctrpnl;
struct Application {
    top_bar: components::topbar::content::Topbar,
    btm_bar: components::btmbar::content::Btmbar,
    lft_pnl: components::lftpnl::content::Lftpnl,
    rht_pnl: components::rhtpnl::content::Rhtpnl,
    ctr_pnl: ctrpnl::content::Ctrpnl,
}impl Application{
    fn init() -> Self{
        Application{
            top_bar: components::topbar::content::Topbar::init(),
            btm_bar: components::btmbar::content::Btmbar::init(),
            lft_pnl: components::lftpnl::content::Lftpnl::init(),
            rht_pnl: components::rhtpnl::content::Rhtpnl::init(),
            ctr_pnl: ctrpnl::content::Ctrpnl::init(),
        }
    }
}

impl eframe::App for Application {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top-panel").show_inside(ui, |ui|{
            self.top_bar.show(ui);
        });
        egui::Panel::bottom("btm-panel").show_inside(ui, |ui|{
            self.btm_bar.show(ui);
        });
        egui::Panel::left("left-panel").resizable(true).size_range(100.0..=400.0).show_inside(ui, |ui|{
            self.lft_pnl.show(ui);
            ui.take_available_space()
        });
        egui::Panel::right("right-panel").resizable(true).size_range(100.0..=400.0).show_inside(ui, |ui|{
            self.rht_pnl.show(ui);
            ui.take_available_space()
        });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.ctr_pnl.show(ui);
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Application",
        options,
        Box::new(|_cc| Ok(Box::new(Application::init()))),
    )
}
