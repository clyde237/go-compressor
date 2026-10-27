fn main() {
    // Garantir l'existence des binaires sidecars requis par tauri-build
    let target = std::env::var("TARGET").unwrap_or_default();
    let ext = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let binaries_dir = std::path::Path::new("binaries");
    if !binaries_dir.exists() {
        let _ = std::fs::create_dir_all(binaries_dir);
    }
    for bin in ["ffmpeg", "ffprobe"] {
        let filename = format!("{bin}-{target}{ext}");
        let target_path = binaries_dir.join(&filename);
        if !target_path.exists() {
            let _ = std::fs::write(&target_path, b"");
        }
    }

    tauri_build::build();
}
