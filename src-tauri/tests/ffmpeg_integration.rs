use assert_cmd::Command;
use predicates::str::contains;
use std::path::{Path, PathBuf};

fn find_ffmpeg() -> PathBuf {
    let candidates = [
        "bin/ffmpeg",
        "bin/ffmpeg.exe",
        "../bin/ffmpeg",
        "../bin/ffmpeg.exe",
        "binaries/ffmpeg-x86_64-unknown-linux-gnu",
        "src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu",
        "../src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu",
    ];

    for c in candidates {
        if Path::new(c).exists() {
            return PathBuf::from(c);
        }
    }

    if cfg!(windows) {
        PathBuf::from("ffmpeg.exe")
    } else {
        PathBuf::from("ffmpeg")
    }
}

#[tokio::test]
async fn ffmpeg_sidecar_is_executable() {
    let ffmpeg = find_ffmpeg();
    Command::new(ffmpeg)
        .arg("-version")
        .assert()
        .success()
        .stdout(contains("ffmpeg version"));
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
