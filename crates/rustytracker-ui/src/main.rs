mod app;
mod effect_entry;
mod input;
mod io;
mod panels;
mod playback;
mod tracker_ui;

use app::RustyTrackerApp;

fn main() -> eframe::Result<()> {
    let startup_module_path = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("RustyTracker")
            .with_inner_size([1100.0, 750.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RustyTracker",
        options,
        Box::new(move |cc| {
            let mut app = RustyTrackerApp::new(&cc.egui_ctx);
            if let Some(path) = startup_module_path.as_deref() {
                app.load_module_file(path);
            }
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}
