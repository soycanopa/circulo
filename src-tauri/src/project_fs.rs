use serde::Serialize;
use std::path::Path;

/// Directory segments pruned from file listings (search + file tree).
pub const IGNORED_DIR_NAMES: [&str; 8] = [
    "node_modules",
    ".git",
    "target",
    ".build",
    "dist",
    ".next",
    ".nuxt",
    "coverage",
];

/// File names pruned from the file tree.
pub const IGNORED_FILE_NAMES: [&str; 1] = [".DS_Store"];

pub fn should_ignore_dir(name: &str) -> bool {
    IGNORED_DIR_NAMES.contains(&name)
}

pub fn should_ignore_file(name: &str) -> bool {
    IGNORED_FILE_NAMES.contains(&name)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

/// List one directory (lazy, per-folder) sorted with directories first.
pub fn read_directory(dir: &Path) -> Result<Vec<DirectoryEntry>, String> {
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", dir.display()));
    }
    let mut entries = Vec::new();
    let read_dir =
        std::fs::read_dir(dir).map_err(|err| format!("Failed to read {}: {err}", dir.display()))?;
    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() && should_ignore_dir(&name) {
            continue;
        }
        if !file_type.is_dir() && should_ignore_file(&name) {
            continue;
        }
        entries.push(DirectoryEntry {
            path: entry.path().to_string_lossy().to_string(),
            is_dir: file_type.is_dir(),
            name,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn lists_directories_first_sorted_and_prunes_ignored() {
        let root = tempdir().unwrap();
        fs::create_dir_all(root.path().join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.path().join("zebra")).unwrap();
        fs::create_dir_all(root.path().join("apple")).unwrap();
        fs::write(root.path().join(".DS_Store"), "").unwrap();
        fs::write(root.path().join("b.txt"), "b").unwrap();
        fs::write(root.path().join("a.txt"), "a").unwrap();

        let entries = read_directory(root.path()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["apple", "zebra", "a.txt", "b.txt"]);
        assert!(entries.iter().all(|e| e.path != "node_modules"));
        assert!(!entries.iter().any(|e| e.name == ".DS_Store"));
    }

    #[test]
    fn rejects_non_directory_paths() {
        let err = read_directory(Path::new("/definitely/not/a/dir")).unwrap_err();
        assert!(err.contains("Not a directory"));
    }
}
