use std::path::PathBuf;

/// Locate the `cmd` binary installed by `npm i -g command-code`.
///
/// Resolution order:
/// 1. The `COMMANDCODE_BIN` env var, if set and pointing to a file.
/// 2. The first `cmd` found in the directories of `$PATH`.
pub fn discover_commandcode_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("COMMANDCODE_BIN") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let path_var = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&path_var) {
        let candidate = entry.join("cmd");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_env_takes_priority() {
        // SAFETY: env mutation is single-threaded in tests via cargo's default runner.
        let prior = std::env::var("COMMANDCODE_BIN").ok();
        let unique = std::env::temp_dir().join("circulo-cmd-test-binary");
        std::fs::write(&unique, b"#!/bin/sh\nexit 0\n").unwrap();
        std::env::set_var("COMMANDCODE_BIN", &unique);
        let found = discover_commandcode_binary();
        match prior {
            Some(v) => std::env::set_var("COMMANDCODE_BIN", v),
            None => std::env::remove_var("COMMANDCODE_BIN"),
        }
        assert_eq!(found, Some(unique));
    }
}
