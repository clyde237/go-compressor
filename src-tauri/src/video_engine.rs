//! Video transcoding engine using FFmpeg sidecar and sequential Tokio queue.
//!
//! Enforces:
//! - Strict sequential processing (1 video at a time) to prevent CPU/RAM thermal saturation
//! - FIFO queue capped at MAX_VIDEOS = 20 (rejecting any attempt beyond 20)
//! - H.264 video encoding with libx264, AAC audio, movflags +faststart
//! - Real-time progress monitoring via `-progress pipe:1 -nostats`
//! - Non-destructive atomic write (.part -> final destination)
//! - Automatic cleanup on failure or cancellation

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, mpsc};

pub const MAX_VIDEOS: usize = 20;
pub const DEFAULT_CRF: u8 = 26;
pub const MIN_CRF: u8 = 23;
pub const MAX_CRF: u8 = 28;

#[derive(Error, Debug)]
pub enum VideoEngineError {
    #[error("Quota vidéo dépassé : {0} vidéos fournies (maximum autorisé : {MAX_VIDEOS})")]
    QuotaExceeded(usize),

    #[error("Fichier source introuvable : {0}")]
    FileNotFound(PathBuf),

    #[error("Impossible de déterminer la durée de la vidéo : {0}")]
    DurationDetectionFailed(PathBuf),

    #[error("Erreur I/O système sur '{0}' : {1}")]
    IoError(PathBuf, #[source] std::io::Error),

    #[error("Échec de l'encodage FFmpeg (code de sortie : {0}) : {1}")]
    TranscodeFailed(i32, String),

    #[error("Opération annulée par l'utilisateur")]
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct VideoTranscodeOptions {
    pub crf: u8,
    pub max_videos: usize,
}

impl Default for VideoTranscodeOptions {
    fn default() -> Self {
        Self {
            crf: DEFAULT_CRF,
            max_videos: MAX_VIDEOS,
        }
    }
}

pub fn is_video_quota_valid(count: usize) -> bool {
    count <= MAX_VIDEOS
}

#[derive(Debug, Clone)]
pub struct VideoJob {
    pub id: String,
    pub input: PathBuf,
    pub output: PathBuf,
    pub crf: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VideoTranscodeReport {
    pub file_path: String,
    pub original_size_bytes: u64,
    pub output_size_bytes: u64,
    pub output_path: String,
    pub gain_percent: u32,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum VideoEvent {
    Queued {
        id: String,
    },
    Rejected {
        id: String,
        reason: String,
    },
    Started {
        id: String,
    },
    Progress {
        id: String,
        percent: f32,
        current_bytes: u64,
    },
    Completed {
        id: String,
        output: PathBuf,
        final_size_bytes: u64,
        gain_percent: u32,
    },
    Failed {
        id: String,
        error: String,
    },
}

/// Vérifie si un binaire existe, est accessible et répond avec succès à `-version`
pub fn is_working_binary(path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    // Si c'est un chemin avec des composantes de répertoire, vérifie l'existence physique
    if (path.is_absolute() || path.components().count() > 1) && !path.exists() {
        return false;
    }
    std::process::Command::new(path)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Résout le chemin absolu ou relatif vers l'exécutable FFmpeg
pub fn resolve_ffmpeg_path() -> PathBuf {
    let ext = if cfg!(windows) { ".exe" } else { "" };
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

    // 1. Dossier relatif à l'exécutable en cours d'exécution (déploiement Tauri installé)
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
    {
        for exe_sidecar_name in [
            format!("gocompressor-ffmpeg-{host_triple}{ext}"),
            format!("gocompressor-ffmpeg{ext}"),
            format!("ffmpeg-{host_triple}{ext}"),
            format!("ffmpeg{ext}"),
        ] {
            let exe_sidecar = exe_dir.join(exe_sidecar_name);
            if is_working_binary(&exe_sidecar) {
                return exe_sidecar;
            }
        }
    }

    // 2. Dossier bin/ à la racine du projet ou parent (développement / tests)
    for bin_cand in [
        format!("bin/gocompressor-ffmpeg{ext}"),
        format!("bin/ffmpeg{ext}"),
        format!("../bin/gocompressor-ffmpeg{ext}"),
        format!("../bin/ffmpeg{ext}"),
    ] {
        let p = PathBuf::from(bin_cand);
        if is_working_binary(&p) {
            return p;
        }
    }

    // 3. Dossier binaries/ avec triplet cible Tauri
    for sidecar_cand in [
        format!("src-tauri/binaries/gocompressor-ffmpeg-{host_triple}{ext}"),
        format!("binaries/gocompressor-ffmpeg-{host_triple}{ext}"),
        format!("../src-tauri/binaries/gocompressor-ffmpeg-{host_triple}{ext}"),
        format!("src-tauri/binaries/ffmpeg-{host_triple}{ext}"),
        format!("binaries/ffmpeg-{host_triple}{ext}"),
        format!("../src-tauri/binaries/ffmpeg-{host_triple}{ext}"),
    ] {
        let p = PathBuf::from(sidecar_cand);
        if is_working_binary(&p) {
            return p;
        }
    }

    // 4. Binaire préfixé gocompressor dans le PATH
    let scoped_bin = if cfg!(windows) {
        "gocompressor-ffmpeg.exe"
    } else {
        "gocompressor-ffmpeg"
    };
    let scoped_path = PathBuf::from(scoped_bin);
    if is_working_binary(&scoped_path) {
        return scoped_path;
    }

    // 5. Binaire standard ffmpeg dans le PATH
    let sys_bin = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let sys_path = PathBuf::from(sys_bin);
    if is_working_binary(&sys_path) {
        return sys_path;
    }

    // 6. Chemins absolus usuels sous Unix / Windows
    for fallback in [
        "/usr/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/opt/homebrew/bin/ffmpeg",
        "C:\\ProgramData\\chocolatey\\bin\\ffmpeg.exe",
        "C:\\ffmpeg\\bin\\ffmpeg.exe",
    ] {
        let p = PathBuf::from(fallback);
        if is_working_binary(&p) {
            return p;
        }
    }

    sys_path
}

/// Résout le chemin absolu ou relatif vers l'exécutable ffprobe
pub fn resolve_ffprobe_path() -> PathBuf {
    let ext = if cfg!(windows) { ".exe" } else { "" };
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

    // 1. Dossier relatif à l'exécutable en cours d'exécution
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
    {
        for exe_sidecar_name in [
            format!("gocompressor-ffprobe-{host_triple}{ext}"),
            format!("gocompressor-ffprobe{ext}"),
            format!("ffprobe-{host_triple}{ext}"),
            format!("ffprobe{ext}"),
        ] {
            let exe_sidecar = exe_dir.join(exe_sidecar_name);
            if is_working_binary(&exe_sidecar) {
                return exe_sidecar;
            }
        }
    }

    // 2. Dossier bin/ à la racine du projet ou parent
    for bin_cand in [
        format!("bin/gocompressor-ffprobe{ext}"),
        format!("bin/ffprobe{ext}"),
        format!("../bin/gocompressor-ffprobe{ext}"),
        format!("../bin/ffprobe{ext}"),
    ] {
        let p = PathBuf::from(bin_cand);
        if is_working_binary(&p) {
            return p;
        }
    }

    // 3. Dossier binaries/ avec triplet cible Tauri
    for sidecar_cand in [
        format!("src-tauri/binaries/gocompressor-ffprobe-{host_triple}{ext}"),
        format!("binaries/gocompressor-ffprobe-{host_triple}{ext}"),
        format!("../src-tauri/binaries/gocompressor-ffprobe-{host_triple}{ext}"),
        format!("src-tauri/binaries/ffprobe-{host_triple}{ext}"),
        format!("binaries/ffprobe-{host_triple}{ext}"),
        format!("../src-tauri/binaries/ffprobe-{host_triple}{ext}"),
    ] {
        let p = PathBuf::from(sidecar_cand);
        if is_working_binary(&p) {
            return p;
        }
    }

    // 4. Binaire préfixé gocompressor dans le PATH
    let scoped_bin = if cfg!(windows) {
        "gocompressor-ffprobe.exe"
    } else {
        "gocompressor-ffprobe"
    };
    let scoped_path = PathBuf::from(scoped_bin);
    if is_working_binary(&scoped_path) {
        return scoped_path;
    }

    // 5. Binaire standard ffprobe dans le PATH
    let sys_bin = if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    };
    let sys_path = PathBuf::from(sys_bin);
    if is_working_binary(&sys_path) {
        return sys_path;
    }

    // 6. Chemins absolus usuels sous Unix / Windows
    for fallback in [
        "/usr/bin/ffprobe",
        "/usr/local/bin/ffprobe",
        "/opt/homebrew/bin/ffprobe",
        "C:\\ProgramData\\chocolatey\\bin\\ffprobe.exe",
        "C:\\ffmpeg\\bin\\ffprobe.exe",
    ] {
        let p = PathBuf::from(fallback);
        if is_working_binary(&p) {
            return p;
        }
    }

    sys_path
}

/// Obtient la durée d'une vidéo en secondes avec ffprobe ou fallback ffmpeg
pub async fn get_video_duration(input: &Path) -> Result<f64, VideoEngineError> {
    if !input.exists() {
        return Err(VideoEngineError::FileNotFound(input.to_path_buf()));
    }

    let ffprobe = resolve_ffprobe_path();
    if let Ok(output) = Command::new(&ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output()
        .await
    {
        let text = String::from_utf8_lossy(&output.stdout);
        if let Some(dur) = text.trim().parse::<f64>().ok().filter(|&d| d > 0.0) {
            return Ok(dur);
        }
    }

    // Fallback : extraction de la durée via la sortie d'erreur de ffmpeg -i
    let ffmpeg = resolve_ffmpeg_path();
    if let Ok(output) = Command::new(&ffmpeg).arg("-i").arg(input).output().await {
        let stderr_text = String::from_utf8_lossy(&output.stderr);
        if let Some(dur) = parse_duration_from_stderr(&stderr_text) {
            return Ok(dur);
        }
    }

    Err(VideoEngineError::DurationDetectionFailed(
        input.to_path_buf(),
    ))
}

/// Analyse une durée au format "Duration: HH:MM:SS.xx" dans le flux de FFmpeg
pub fn parse_duration_from_stderr(text: &str) -> Option<f64> {
    let dur_pos = text.find("Duration:")?;
    let slice = &text[dur_pos + 9..];
    let end_pos = slice.find(',')?;
    let time_str = slice[..end_pos].trim();

    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() == 3 {
        let hours: f64 = parts[0].parse().ok()?;
        let minutes: f64 = parts[1].parse().ok()?;
        let seconds: f64 = parts[2].parse().ok()?;
        Some(hours * 3600.0 + minutes * 60.0 + seconds)
    } else {
        None
    }
}

/// Vérifie si une vidéo contient au moins un flux audio via ffprobe (avec fallback ffmpeg -i)
pub fn probe_video_has_audio(input: &Path) -> Option<bool> {
    if !input.exists() {
        return None;
    }

    let ffprobe = resolve_ffprobe_path();
    let probe_res = std::process::Command::new(&ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let stdout_text = String::from_utf8_lossy(&o.stdout);
            stdout_text.trim() == "audio"
        });

    if let Some(has_audio) = probe_res {
        return Some(has_audio);
    }

    // Fallback : inspection du flux d'erreur de ffmpeg -i
    let ffmpeg = resolve_ffmpeg_path();
    std::process::Command::new(&ffmpeg)
        .arg("-i")
        .arg(input)
        .output()
        .ok()
        .map(|o| {
            let stderr_text = String::from_utf8_lossy(&o.stderr);
            stderr_text.contains("Audio:")
        })
}

/// Encode un unique fichier vidéo avec FFmpeg et parsing temps réel
pub async fn encode_video_job<F>(
    job: &VideoJob,
    mut progress_callback: F,
) -> Result<VideoTranscodeReport, VideoEngineError>
where
    F: FnMut(f32, u64, &str) + Send,
{
    if !job.input.exists() {
        return Err(VideoEngineError::FileNotFound(job.input.clone()));
    }

    let orig_size = std::fs::metadata(&job.input).map(|m| m.len()).unwrap_or(0);

    // Durée source en secondes et microsecondes
    let duration_sec = get_video_duration(&job.input).await.unwrap_or(1.0);
    let duration_us = (duration_sec * 1_000_000.0).max(1.0) as u64;

    // Fichier temporaire atomique préservant l'extension pour le multiplexeur FFmpeg
    let stem = job
        .output
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let ext = job
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4")
        .to_lowercase();
    let part_output = job.output.with_file_name(format!("{stem}_temp_part.{ext}"));

    if let Some(parent) = part_output.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    let crf = job.crf.clamp(MIN_CRF, MAX_CRF);
    let ffmpeg_bin = resolve_ffmpeg_path();

    let mut cmd = Command::new(&ffmpeg_bin);
    cmd.arg("-y").arg("-i").arg(&job.input).args([
        "-map",
        "0:v:0?",
        "-map",
        "0:a?",
        "-c:v",
        "libx264",
        "-preset",
        "medium",
        "-crf",
        &crf.to_string(),
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-b:a",
        "128k",
        "-ac",
        "2",
        "-ar",
        "44100",
    ]);

    if ext == "mp4" || ext == "m4v" {
        cmd.args(["-movflags", "+faststart", "-f", "mp4"]);
    } else if ext == "mov" {
        cmd.args(["-movflags", "+faststart", "-f", "mov"]);
    }

    cmd.args(["-progress", "pipe:1", "-nostats"])
        .arg(&part_output)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| VideoEngineError::IoError(ffmpeg_bin, e))?;

    let stdout = child.stdout.take().ok_or_else(|| {
        VideoEngineError::TranscodeFailed(-1, "Impossible de capturer stdout de FFmpeg".into())
    })?;

    let stderr_handle = child.stderr.take();
    let stderr_task = tokio::spawn(async move {
        if let Some(stderr) = stderr_handle {
            let mut reader = BufReader::new(stderr).lines();
            let mut all_lines = Vec::new();
            while let Ok(Some(line)) = reader.next_line().await {
                all_lines.push(line);
            }
            all_lines.join("\n")
        } else {
            String::new()
        }
    });

    let mut reader = BufReader::new(stdout).lines();
    let mut current_out_time_us: Option<u64> = None;
    let mut current_size_bytes: u64 = 0;

    progress_callback(5.0, 0, "Transcodage H.264 démarré...");

    while let Ok(Some(line)) = reader.next_line().await {
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };

        match key.trim() {
            "out_time_us" | "out_time_ms" => {
                current_out_time_us = val.trim().parse::<u64>().ok();
            }
            "total_size" => {
                if let Ok(bytes) = val.trim().parse::<u64>() {
                    current_size_bytes = bytes;
                }
            }
            "progress" => {
                let status_val = val.trim();
                if status_val == "end" {
                    progress_callback(99.0, current_size_bytes, "Finalisation du conteneur...");
                } else if let Some(elapsed_us) = current_out_time_us {
                    let pct =
                        ((elapsed_us as f64 / duration_us as f64) * 100.0).clamp(5.0, 98.0) as f32;
                    progress_callback(pct, current_size_bytes, "Encodage H.264...");
                }
            }
            _ => {}
        }
    }

    let status = child.wait().await.map_err(|e| {
        let _ = std::fs::remove_file(&part_output);
        VideoEngineError::IoError(part_output.clone(), e)
    })?;

    let stderr_output = stderr_task.await.unwrap_or_default();

    if !status.success() {
        let _ = std::fs::remove_file(&part_output);
        let code = status.code().unwrap_or(-1);
        return Err(VideoEngineError::TranscodeFailed(
            code,
            format!(
                "Processus FFmpeg terminé avec le code {} : {}",
                code, stderr_output
            ),
        ));
    }

    // Écriture atomique : renommer .part vers la destination définitive
    if let Err(e) = tokio::fs::rename(&part_output, &job.output).await {
        let _ = std::fs::remove_file(&part_output);
        return Err(VideoEngineError::IoError(job.output.clone(), e));
    }

    let final_size = std::fs::metadata(&job.output)
        .map(|m| m.len())
        .unwrap_or(current_size_bytes);

    let gain_pct = if orig_size > 0 && final_size < orig_size {
        (((orig_size - final_size) as f64 / orig_size as f64) * 100.0) as u32
    } else {
        0
    };

    progress_callback(100.0, final_size, "Transcodage terminé");

    Ok(VideoTranscodeReport {
        file_path: job.input.to_string_lossy().to_string(),
        original_size_bytes: orig_size,
        output_size_bytes: final_size,
        output_path: job.output.to_string_lossy().to_string(),
        gain_percent: gain_pct,
        success: true,
        error_message: None,
    })
}

/// Traitement séquentiel d'un lot de vidéos via la boucle FIFO asynchrone
pub async fn transcode_videos_sequential<F>(
    videos: &[String],
    output_dir: &Path,
    export_mode: &crate::export_engine::ExportMode,
    folder_timestamp: Option<&str>,
    crf: u8,
    mut progress_callback: F,
) -> Result<Vec<crate::ProcessedMediaItem>, VideoEngineError>
where
    F: FnMut(String, f32, u64, &str, bool, bool, Option<String>) + Send,
{
    if videos.len() > MAX_VIDEOS {
        return Err(VideoEngineError::QuotaExceeded(videos.len()));
    }

    let mut reports = Vec::with_capacity(videos.len());

    for (idx, vid_str) in videos.iter().enumerate() {
        let input_path = Path::new(vid_str);
        if !input_path.exists() {
            progress_callback(
                vid_str.clone(),
                100.0,
                0,
                "Fichier introuvable sur le disque",
                true,
                true,
                Some("Fichier introuvable sur le disque".into()),
            );
            reports.push(crate::ProcessedMediaItem {
                file_path: vid_str.clone(),
                original_size_bytes: 0,
                output_size_bytes: 0,
                output_path: String::new(),
                gain_percent: 0,
                success: false,
                error_message: Some("Fichier introuvable sur le disque".into()),
            });
            continue;
        }

        let orig_len = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
        let dest_path = crate::export_engine::resolve_export_path(
            input_path,
            output_dir,
            export_mode,
            folder_timestamp,
        );

        let job = VideoJob {
            id: format!("video_{idx}"),
            input: input_path.to_path_buf(),
            output: dest_path.clone(),
            crf,
        };

        let current_path = vid_str.clone();
        progress_callback(
            current_path.clone(),
            0.0,
            0,
            "Initialisation du transcodage séquentiel FFmpeg...",
            false,
            false,
            None,
        );

        let mut progress_for_job = |pct: f32, bytes: u64, stage: &str| {
            progress_callback(current_path.clone(), pct, bytes, stage, false, false, None);
        };

        match encode_video_job(&job, &mut progress_for_job).await {
            Ok(rep) => {
                progress_callback(
                    current_path.clone(),
                    100.0,
                    rep.output_size_bytes,
                    "Transcodage terminé",
                    true,
                    false,
                    None,
                );
                reports.push(crate::ProcessedMediaItem {
                    file_path: rep.file_path,
                    original_size_bytes: rep.original_size_bytes,
                    output_size_bytes: rep.output_size_bytes,
                    output_path: rep.output_path,
                    gain_percent: rep.gain_percent,
                    success: true,
                    error_message: None,
                });
            }
            Err(err) => {
                progress_callback(
                    current_path.clone(),
                    100.0,
                    0,
                    "Échec du transcodage",
                    true,
                    true,
                    Some(err.to_string()),
                );
                reports.push(crate::ProcessedMediaItem {
                    file_path: current_path,
                    original_size_bytes: orig_len,
                    output_size_bytes: 0,
                    output_path: dest_path.to_string_lossy().to_string(),
                    gain_percent: 0,
                    success: false,
                    error_message: Some(err.to_string()),
                });
            }
        }
    }

    Ok(reports)
}

/// File d'attente FIFO strictement séquentielle (1 encodage à la fois, max 20 vidéos)
#[derive(Clone)]
pub struct VideoQueue {
    sender: mpsc::Sender<VideoJob>,
    queue_len: Arc<Mutex<usize>>,
}

impl VideoQueue {
    pub fn new<F>(mut on_event: F) -> Self
    where
        F: FnMut(VideoEvent) + Send + 'static,
    {
        let (sender, mut receiver) = mpsc::channel::<VideoJob>(MAX_VIDEOS);
        let queue_len = Arc::new(Mutex::new(0usize));
        let worker_queue_len = queue_len.clone();

        tokio::spawn(async move {
            // Traitement STRICTEMENT séquentiel : 1 vidéo à la fois
            while let Some(job) = receiver.recv().await {
                {
                    let mut len = worker_queue_len.lock().await;
                    *len = len.saturating_sub(1);
                }

                on_event(VideoEvent::Started { id: job.id.clone() });

                let job_id = job.id.clone();
                let output = job.output.clone();
                let fail_id = job.id.clone();

                let res = encode_video_job(&job, |_pct, _bytes, _stage| {}).await;

                match res {
                    Ok(rep) => {
                        on_event(VideoEvent::Completed {
                            id: job_id,
                            output,
                            final_size_bytes: rep.output_size_bytes,
                            gain_percent: rep.gain_percent,
                        });
                    }
                    Err(err) => {
                        on_event(VideoEvent::Failed {
                            id: fail_id,
                            error: err.to_string(),
                        });
                    }
                }
            }
        });

        Self { sender, queue_len }
    }

    /// Tente d'ajouter un travail d'encodage vidéo dans la file FIFO
    pub async fn enqueue(&self, job: VideoJob) -> VideoEvent {
        let mut len = self.queue_len.lock().await;
        if *len >= MAX_VIDEOS {
            return VideoEvent::Rejected {
                id: job.id,
                reason: format!("File pleine : maximum {MAX_VIDEOS} vidéos par lot."),
            };
        }

        match self.sender.try_send(job.clone()) {
            Ok(()) => {
                *len += 1;
                VideoEvent::Queued { id: job.id }
            }
            Err(e) => VideoEvent::Rejected {
                id: job.id,
                reason: e.to_string(),
            },
        }
    }

    pub async fn len(&self) -> usize {
        *self.queue_len.lock().await
    }

    pub async fn is_empty(&self) -> bool {
        *self.queue_len.lock().await == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_video_options() {
        let opts = VideoTranscodeOptions::default();
        assert_eq!(opts.crf, 26);
        assert_eq!(opts.max_videos, 20);
    }

    #[test]
    fn test_video_quota_validation() {
        assert!(is_video_quota_valid(0));
        assert!(is_video_quota_valid(20));
        assert!(!is_video_quota_valid(21)); // Le rejet de la 21e vidéo exigé par le CDC
    }

    #[test]
    fn test_parse_duration_from_stderr() {
        let sample_stderr = "
  Metadata:
    major_brand     : isom
  Duration: 00:01:23.45, start: 0.000000, bitrate: 1234 kb/s
  Stream #0:0: Video: h264
";
        let dur = parse_duration_from_stderr(sample_stderr);
        assert!(dur.is_some());
        let seconds = dur.unwrap();
        // 1 min 23.45 s = 60 + 23.45 = 83.45
        assert!((seconds - 83.45).abs() < 0.01);
    }

    #[test]
    fn test_resolve_ffmpeg_path() {
        let path = resolve_ffmpeg_path();
        assert!(!path.as_os_str().is_empty());
        assert!(is_working_binary(&path));
    }

    #[test]
    fn test_is_working_binary_with_invalid_path() {
        let invalid = Path::new("/path/that/does/not/exist/or_crash");
        assert!(!is_working_binary(invalid));
    }

    #[tokio::test]
    async fn test_video_queue_rejection() {
        let queue = VideoQueue::new(|_| {});
        for i in 1..=20 {
            let job = VideoJob {
                id: format!("job_{i}"),
                input: PathBuf::from(format!("/tmp/dummy_{i}.mp4")),
                output: PathBuf::from(format!("/tmp/dummy_{i}_out.mp4")),
                crf: 26,
            };
            let ev = queue.enqueue(job).await;
            match ev {
                VideoEvent::Queued { .. } => {}
                _ => panic!("Le job {i} aurait dû être accepté"),
            }
        }

        // 21e tentative : doit être rejetée avec VideoEvent::Rejected
        let job_21 = VideoJob {
            id: "job_21".into(),
            input: PathBuf::from("/tmp/dummy_21.mp4"),
            output: PathBuf::from("/tmp/dummy_21_out.mp4"),
            crf: 26,
        };
        let ev_21 = queue.enqueue(job_21).await;
        match ev_21 {
            VideoEvent::Rejected { reason, .. } => {
                assert!(reason.contains("20 vidéos"));
            }
            _ => panic!("Le 21e job aurait dû être formellement rejeté !"),
        }
    }

    #[tokio::test]
    async fn test_encode_synthetic_video() {
        let temp_dir = std::env::temp_dir().join("go_compressor_video_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let input_path = temp_dir.join("input_test.mp4");
        let output_path = temp_dir.join("output_test.mp4");

        // Générer une courte vidéo de test synthétique de 1 seconde avec vidéo ET audio
        let ffmpeg_bin = resolve_ffmpeg_path();
        let gen_status = Command::new(&ffmpeg_bin)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=1:size=160x120:rate=10",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:duration=1",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
                input_path.to_str().unwrap(),
            ])
            .output()
            .await;

        if let Ok(gen_res) = gen_status {
            if !gen_res.status.success() {
                eprintln!(
                    "FFmpeg présent mais la génération de vidéo synthétique a échoué (stderr: {}) - test ignoré.",
                    String::from_utf8_lossy(&gen_res.stderr)
                );
                return;
            }

            assert_eq!(probe_video_has_audio(&input_path), Some(true));

            let job = VideoJob {
                id: "test_job".into(),
                input: input_path.clone(),
                output: output_path.clone(),
                crf: 28,
            };

            let mut progress_count = 0;
            let res = encode_video_job(&job, |_pct, _bytes, _stage| {
                progress_count += 1;
            })
            .await;

            assert!(
                res.is_ok(),
                "L'encodage du job vidéo a échoué : {:?}",
                res.err()
            );
            let report = res.unwrap();
            assert!(report.success);
            assert!(output_path.exists());
            assert_eq!(
                probe_video_has_audio(&output_path),
                Some(true),
                "Le flux audio doit être conservé dans le fichier compressé !"
            );
            assert!(report.output_size_bytes > 0);
            assert!(progress_count > 0);

            // Nettoyage
            let _ = std::fs::remove_file(&input_path);
            let _ = std::fs::remove_file(&output_path);
        }
    }
}
