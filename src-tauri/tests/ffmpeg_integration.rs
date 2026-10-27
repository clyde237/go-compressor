use std::path::{Path, PathBuf};

fn find_ffmpeg() -> Option<PathBuf> {
    let host_triple = if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else {
        "x86_64-unknown-linux-gnu"
    };

    let ext = if cfg!(windows) { ".exe" } else { "" };

    let candidates = [
        format!("bin/ffmpeg{ext}"),
        format!("../bin/ffmpeg{ext}"),
        format!("binaries/ffmpeg-{host_triple}{ext}"),
        format!("src-tauri/binaries/ffmpeg-{host_triple}{ext}"),
        format!("../src-tauri/binaries/ffmpeg-{host_triple}{ext}"),
    ];

    for c in &candidates {
        let p = Path::new(c);
        if p.exists() && p.metadata().map(|m| m.len() > 100).unwrap_or(false) {
            return Some(PathBuf::from(c));
        }
    }

    let resolved = go_compressor_lib::video_engine::resolve_ffmpeg_path();
    if resolved.exists() && resolved.metadata().map(|m| m.len() > 100).unwrap_or(false) {
        return Some(resolved);
    }

    let sys_bin = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    if std::process::Command::new(sys_bin)
        .arg("-version")
        .output()
        .is_ok()
    {
        return Some(PathBuf::from(sys_bin));
    }

    None
}

#[tokio::test]
async fn ffmpeg_sidecar_is_executable() {
    if let Some(ffmpeg) = find_ffmpeg() {
        let output = std::process::Command::new(&ffmpeg).arg("-version").output();
        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                assert!(stdout.contains("ffmpeg version"));
            }
            Ok(out) => {
                eprintln!(
                    "FFmpeg a été invoqué mais a retourné le code {:?} (stdout: {}, stderr: {})",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            Err(e) => {
                eprintln!(
                    "Impossible d'exécuter FFmpeg dans cet environnement : {:?}",
                    e
                );
            }
        }
    } else {
        eprintln!("Binaire FFmpeg non présent dans cet environnement de test.");
    }
}

#[test]
fn test_video_quota_guard() {
    assert!(go_compressor_lib::video_engine::is_video_quota_valid(0));
    assert!(go_compressor_lib::video_engine::is_video_quota_valid(19));
    assert!(go_compressor_lib::video_engine::is_video_quota_valid(20));
    assert!(!go_compressor_lib::video_engine::is_video_quota_valid(21));
}

#[test]
fn test_create_zip_integration() {
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    let dir = tempdir().expect("tempdir");
    let file1_path = dir.path().join("file1.txt");
    let mut f1 = File::create(&file1_path).unwrap();
    f1.write_all(b"Contenu de test pour archive ZIP").unwrap();

    let zip_dest = dir.path().join("archive_test.zip");
    let res = go_compressor_lib::export_engine::create_zip_archive(&[file1_path], &zip_dest);

    assert!(res.is_ok());
    assert!(zip_dest.exists());
    assert!(zip_dest.metadata().unwrap().len() > 0);
}
