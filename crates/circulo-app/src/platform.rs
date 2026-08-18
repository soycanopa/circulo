//! macOS-native helpers for the desktop shell.

use std::path::PathBuf;

/// Opens a folder picker for adding a project.
///
/// Run via `BackgroundExecutor` — not on the GPUI main thread. A modal panel on
/// the main thread pumps the Cocoa run loop and can re-enter GPUI while the app
/// `RefCell` is still borrowed (`RefCell already borrowed` panic).
pub fn pick_project_folder(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}
