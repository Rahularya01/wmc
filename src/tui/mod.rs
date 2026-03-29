pub mod app;
pub mod dashboard;
pub mod layout;
pub mod types;

use std::path::Path;

use app::TuiApp;

/// Launches the interactive terminal UI, blocking until the user quits.
/// Exits the process with an error message on fatal initialisation failures.
pub fn run_tui(target: &Path) {
    let mut app = match TuiApp::new(target.to_path_buf()) {
        Ok(app) => app,
        Err(error) => {
            eprintln!("Failed to initialize terminal UI: {}", error);
            std::process::exit(1);
        }
    };

    if let Err(error) = app.run() {
        let _ = app.terminal.restore();
        eprintln!("Terminal UI failed: {}", error);
        std::process::exit(1);
    }
}
