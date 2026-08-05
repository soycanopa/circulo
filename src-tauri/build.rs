use std::{env, fs, path::PathBuf};

fn main() {
    // Tauri validates that every `externalBin` exists at build time. The
    // `circulo-mcp` sidecar is a `[[bin]]` target built by
    // `scripts/build-sidecar.mjs` (dev/build flows via `before*Command`).
    // For bare `cargo check`/`cargo test`/`cargo build` we pre-create an empty
    // placeholder so the build script doesn't fail; it is replaced by the real
    // binary in any real app run, and `resolve_circulo_mcp_binary` ignores
    // zero-length files.
    if let Some(manifest_dir) = env::var_os("CARGO_MANIFEST_DIR") {
        if let Ok(target_triple) = env::var("TARGET") {
            let binaries = PathBuf::from(manifest_dir).join("binaries");
            let placeholder = binaries.join(format!("circulo-mcp-{target_triple}"));
            if !placeholder.is_file() {
                let _ = fs::create_dir_all(&binaries);
                let _ = fs::write(&placeholder, []);
            }
        }
    }

    tauri_build::build()
}
