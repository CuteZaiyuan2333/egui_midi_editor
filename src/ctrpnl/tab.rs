use eframe::egui;
use egui_dock::TabViewer;
use egui_midi::{
    audio::{
        AudioEngine,
        PlaybackBackend
    },
    structure::MidiState,
    ui::MidiEditor,
};
use midly::Smf;

pub enum Mytab{
    Track,
    PianoRoll,
    SampleRoll,
    WaveInspector,
    BlueprintEditor,
}

pub struct MyTabViewer{
    pub debugmidi: MidiEditor
}impl TabViewer for MyTabViewer {
    type Tab = Mytab;
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab{
            Mytab::Track => "track ditor".into(),
            Mytab::PianoRoll => "piano roll".into(),
            Mytab::SampleRoll => "sample roll".into(),
            Mytab::WaveInspector => "wave inspector".into(),
            Mytab::BlueprintEditor => "blueprint editor".into(),
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab{
            Mytab::Track => {
                ui.label("place-holder(track editor)");
            }
            Mytab::PianoRoll => {
                //ui.label("place-holder(piano roll)");
                self.debugmidi.ui(ui);
            }
            Mytab::SampleRoll => {
                ui.label("place-holder(sample roll)");
            }
            Mytab::WaveInspector => {
                ui.label("place-holder(wave inspector)");
            }
            Mytab::BlueprintEditor => {
                ui.label("place-holder(blueprint editor)");
            }
        }
    }
}