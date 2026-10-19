use eframe::egui;

pub trait HeadlessContext {
    fn run_headless(
        &self,
        input: egui::RawInput,
        draw: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput;
}

impl HeadlessContext for egui::Context {
    fn run_headless(
        &self,
        input: egui::RawInput,
        draw: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput {
        let mut output = self.run_ui(input, draw);
        // These tests inspect paint shapes without a renderer or GPU textures.
        output.textures_delta.clear();
        output
    }
}
