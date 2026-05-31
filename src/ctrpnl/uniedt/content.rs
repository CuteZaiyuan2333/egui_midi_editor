use eframe::egui;

pub struct Uniedt{
    //data
}impl Uniedt{
    pub fn init() -> Self{
        Uniedt{
            //initialize
        }
    }
}

impl Uniedt{
    pub fn show(self: &mut Self, ui: &mut egui::Ui) -> &mut Self{
        
        return self;
    }

    pub fn add_pad<F>(ui: &mut egui::Ui, mut callback: F) 
        where 
            F: FnMut(&mut egui::Ui, f32, f32),{
        
    }
    
}

// {
//  let a: Uniedt = Uniedt::init();
//  a.show(ui)
//      .add_pad(ui, |ui|{
// 
//      })
//      .node