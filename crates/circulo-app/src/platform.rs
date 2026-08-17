//! macOS-native helpers for the desktop shell.

use std::path::PathBuf;

/// Opens a folder picker for adding a project. Must run on the main thread.
pub fn pick_project_folder(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}
