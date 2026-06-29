mod app;
mod audio;
mod effect_entry;
mod input;
mod panels;
mod tracker_ui;

use app::RustyTrackerApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("RustyTracker")
            .with_inner_size([1100.0, 750.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RustyTracker",
        options,
        Box::new(|cc| Box::new(RustyTrackerApp::new(&cc.egui_ctx)) as Box<dyn eframe::App>),
    )
}
