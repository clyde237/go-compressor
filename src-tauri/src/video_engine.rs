//! Video transcoding engine using FFmpeg sidecar and sequential Tokio queue.
//!
//! Enforces:
//! - Strict sequential processing (1 video at a time) to prevent CPU/RAM thermal saturation
//! - FIFO queue capped at MAX_VIDEOS = 20 (rejecting any attempt beyond 20)
//! - H.264 / H.265 / VP9 encoding in MP4, MKV, MOV or WebM containers
//! - Capped CRF: CRF sets the quality floor, a VBV bitrate cap derived from the
//!   source bitrate guarantees the output weighs at most `max_size_percent` of the
//!   original. CRF alone targets quality, not size: an already compressed source
//!   (HEVC smartphone video, for example) re-encoded at CRF 26 barely shrinks.
//! - Optional downscaling (short side capped at `max_height`)
//! - Real-time progress monitoring via `-progress pipe:1 -nostats`
//! - Non-destructive atomic write (.part -> final destination)
//! - Automatic cleanup on failure or cancellation

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, mpsc};

pub const MAX_VIDEOS: usize = 20;
pub const DEFAULT_CRF: u8 = 26;
pub const MIN_CRF: u8 = 18;
pub const MAX_CRF: u8 = 32;
/// Taille de sortie maximale par défaut, en pourcentage de la taille source
pub const DEFAULT_MAX_SIZE_PERCENT: u8 = 50;
/// Marge appliquée au budget de débit : le VBV tolère de légers dépassements
const VBV_SAFETY_MARGIN: f64 = 0.95;
/// Débit vidéo plancher absolu (kb/s)
const MIN_VIDEO_KBPS: u32 = 100;

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

/// Codec vidéo de sortie
#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum VideoCodec {
    /// H.264 / AVC : compatibilité maximale
    #[default]
    H264,
    /// H.265 / HEVC : environ 40 % plus léger à qualité égale
    H265,
    /// VP9 : format ouvert du web (WebM, MKV)
    Vp9,
}

impl VideoCodec {
    pub const ALL: [VideoCodec; 3] = [VideoCodec::H264, VideoCodec::H265, VideoCodec::Vp9];

    pub fn encoder_name(self) -> &'static str {
        match self {
            Self::H264 => "libx264",
            Self::H265 => "libx265",
            Self::Vp9 => "libvpx-vp9",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::H264 => "H.264",
            Self::H265 => "H.265",
            Self::Vp9 => "VP9",
        }
    }

    /// Convertit le CRF utilisateur (échelle x264) en CRF équivalent pour ce codec
    pub fn native_crf(self, crf: u8) -> u8 {
        match self {
            Self::H264 => crf.min(51),
            Self::H265 => crf.saturating_add(3).min(51),
            Self::Vp9 => crf.saturating_add(9).min(63),
        }
    }

    /// Densité minimale (bits par pixel et par image) sous laquelle l'image se dégrade
    fn min_bits_per_pixel(self) -> f64 {
        match self {
            Self::H264 => 0.008,
            Self::H265 | Self::Vp9 => 0.005,
        }
    }
}

/// Conteneur (extension) du fichier vidéo produit
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum VideoContainer {
    /// Conserve le conteneur source s'il est supporté (MP4, MKV, MOV, WebM), sinon MP4
    #[default]
    Auto,
    Mp4,
    Mkv,
    Mov,
    Webm,
}

/// Résout l'extension de sortie et un codec compatible avec ce conteneur.
/// WebM n'accepte que VP9 ; MP4 et MOV n'utilisent pas VP9 (repli H.264).
pub fn resolve_output_format(
    input: &Path,
    container: VideoContainer,
    codec: VideoCodec,
) -> (&'static str, VideoCodec) {
    let ext = match container {
        VideoContainer::Mp4 => "mp4",
        VideoContainer::Mkv => "mkv",
        VideoContainer::Mov => "mov",
        VideoContainer::Webm => "webm",
        VideoContainer::Auto => {
            let source_ext = input
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            match (source_ext.as_str(), codec) {
                ("mkv", _) => "mkv",
                ("webm", VideoCodec::Vp9) => "webm",
                ("mov", VideoCodec::H264 | VideoCodec::H265) => "mov",
                (_, VideoCodec::Vp9) => "webm",
                _ => "mp4",
            }
        }
    };
    (ext, codec_for_extension(ext, codec))
}

/// Codec effectivement utilisable dans le conteneur désigné par `ext`
fn codec_for_extension(ext: &str, codec: VideoCodec) -> VideoCodec {
    match (ext, codec) {
        ("webm", _) => VideoCodec::Vp9,
        ("mp4" | "m4v" | "mov", VideoCodec::Vp9) => VideoCodec::H264,
        _ => codec,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct VideoTranscodeOptions {
    /// Qualité plancher, échelle x264 (MIN_CRF..=MAX_CRF), convertie par codec
    pub crf: u8,
    pub codec: VideoCodec,
    pub container: VideoContainer,
    /// Taille de sortie maximale en % de la source ; 100 = pas de plafond de débit
    pub max_size_percent: u8,
    /// Plus petit côté maximal en pixels (720 = 720p) ; 0 = résolution d'origine
    pub max_height: u32,
}

impl Default for VideoTranscodeOptions {
    fn default() -> Self {
        Self {
            crf: DEFAULT_CRF,
            codec: VideoCodec::default(),
            container: VideoContainer::default(),
            max_size_percent: DEFAULT_MAX_SIZE_PERCENT,
            max_height: 0,
        }
    }
}

/// Caractéristiques techniques de la source lues par ffprobe
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VideoProbe {
    pub duration_sec: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub video_codec: String,
    pub audio_bitrate_bps: Option<u64>,
    pub has_audio: bool,
}

/// Paramètres d'encodage calculés pour une source donnée
#[derive(Debug, Clone, PartialEq)]
pub struct VideoEncodePlan {
    pub codec: VideoCodec,
    /// CRF natif du codec
    pub crf: u8,
    /// Plafond de débit vidéo (kb/s) ; `None` = CRF seul
    pub max_video_kbps: Option<u32>,
    pub scale_filter: Option<String>,
    pub audio_kbps: u32,
}

pub fn is_video_quota_valid(count: usize) -> bool {
    count <= MAX_VIDEOS
}

#[derive(Debug, Clone)]
pub struct VideoJob {
    pub id: String,
    pub input: PathBuf,
    pub output: PathBuf,
    pub options: VideoTranscodeOptions,
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

/// Liste des encodeurs compilés dans le binaire FFmpeg (calculée une seule fois).
/// Ensemble vide si FFmpeg ne répond pas : aucun codec n'est alors exclu a priori.
pub fn available_encoders() -> &'static HashSet<String> {
    static ENCODERS: OnceLock<HashSet<String>> = OnceLock::new();
    ENCODERS.get_or_init(|| {
        std::process::Command::new(resolve_ffmpeg_path())
            .args(["-hide_banner", "-encoders"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| parse_encoder_list(&String::from_utf8_lossy(&o.stdout)))
            .unwrap_or_default()
    })
}

/// Extrait les noms d'encodeurs de la sortie de `ffmpeg -encoders`
/// (lignes de la forme ` V....D libx264  description`)
pub fn parse_encoder_list(text: &str) -> HashSet<String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let flags = parts.next()?;
            let name = parts.next()?;
            let is_capability_flags = flags.len() == 6
                && flags.starts_with(['V', 'A', 'S'])
                && flags.chars().all(|c| c.is_ascii_uppercase() || c == '.');
            (is_capability_flags && name != "=").then(|| name.to_string())
        })
        .collect()
}

fn encoder_available(name: &str) -> bool {
    let encoders = available_encoders();
    encoders.is_empty() || encoders.contains(name)
}

/// Codecs vidéo utilisables avec le binaire FFmpeg embarqué
pub fn supported_video_codecs() -> Vec<VideoCodec> {
    VideoCodec::ALL
        .into_iter()
        .filter(|c| encoder_available(c.encoder_name()))
        .collect()
}

/// Analyse la sortie JSON de `ffprobe -show_entries format=duration:stream=...`
pub fn parse_ffprobe_json(text: &str) -> Option<VideoProbe> {
    let json: serde_json::Value = serde_json::from_str(text).ok()?;
    let as_f64 = |v: &serde_json::Value| -> Option<f64> {
        v.as_str()
            .and_then(|s| s.parse().ok())
            .or_else(|| v.as_f64())
    };

    let mut probe = VideoProbe {
        duration_sec: json
            .pointer("/format/duration")
            .and_then(as_f64)
            .unwrap_or(0.0),
        ..Default::default()
    };

    for stream in json.get("streams")?.as_array()? {
        match stream.get("codec_type").and_then(|v| v.as_str()) {
            Some("video") if probe.video_codec.is_empty() => {
                probe.video_codec = stream
                    .get("codec_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                probe.width = stream.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                probe.height = stream.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                probe.fps = stream
                    .get("avg_frame_rate")
                    .and_then(|v| v.as_str())
                    .and_then(parse_frame_rate)
                    .unwrap_or(0.0);
            }
            Some("audio") if !probe.has_audio => {
                probe.has_audio = true;
                probe.audio_bitrate_bps = stream.get("bit_rate").and_then(as_f64).map(|b| b as u64);
            }
            _ => {}
        }
    }

    Some(probe)
}

/// Convertit un débit d'images ffprobe ("30000/1001", "25/1") en images par seconde
fn parse_frame_rate(text: &str) -> Option<f64> {
    let (num, den) = text.split_once('/').unwrap_or((text, "1"));
    let num: f64 = num.trim().parse().ok()?;
    let den: f64 = den.trim().parse().ok()?;
    (num > 0.0 && den > 0.0).then_some(num / den)
}

/// Sonde la source (durée, dimensions, cadence, audio). Repli sur la durée seule.
pub async fn probe_video(input: &Path) -> VideoProbe {
    let ffprobe = resolve_ffprobe_path();
    let parsed = Command::new(&ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type,codec_name,width,height,avg_frame_rate,bit_rate",
            "-of",
            "json",
        ])
        .arg(input)
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| parse_ffprobe_json(&String::from_utf8_lossy(&o.stdout)));

    let mut probe = parsed.unwrap_or_default();
    if probe.duration_sec <= 0.0 {
        probe.duration_sec = get_video_duration(input).await.unwrap_or(0.0);
    }
    if !probe.has_audio {
        probe.has_audio = probe_video_has_audio(input).unwrap_or(true);
    }
    probe
}

/// Calcule les paramètres d'encodage : CRF natif, plafond de débit et mise à l'échelle.
///
/// Le plafond vise `max_size_percent` % du débit moyen de la source (taille / durée),
/// audio déduit. Un plancher par pixel évite une image détruite sur les sources
/// déjà très compressées : le gain est alors limité, sauf à réduire la résolution.
pub fn plan_encode(
    probe: &VideoProbe,
    original_size_bytes: u64,
    options: &VideoTranscodeOptions,
    codec: VideoCodec,
) -> VideoEncodePlan {
    let audio_ceiling_kbps: u32 = if codec == VideoCodec::Vp9 { 96 } else { 128 };
    let audio_kbps = probe
        .audio_bitrate_bps
        .map(|bps| ((bps / 1000) as u32).clamp(64, audio_ceiling_kbps))
        .unwrap_or(audio_ceiling_kbps);

    let short_side = probe.width.min(probe.height);
    let long_side = probe.width.max(probe.height);
    let (scale_filter, out_short, out_long) =
        if options.max_height > 0 && short_side > options.max_height {
            let n = options.max_height;
            let ratio = n as f64 / short_side as f64;
            (
                // Contraint le plus petit côté : valable en paysage comme en portrait
                Some(format!(
                    "scale='if(gt(iw,ih),-2,{n})':'if(gt(iw,ih),{n},-2)'"
                )),
                n,
                (long_side as f64 * ratio).round() as u32,
            )
        } else {
            (None, short_side, long_side)
        };

    let max_video_kbps = (options.max_size_percent < 100
        && probe.duration_sec > 0.0
        && original_size_bytes > 0)
        .then(|| {
            let source_total_kbps = original_size_bytes as f64 * 8.0 / probe.duration_sec / 1000.0;
            let budget_kbps =
                source_total_kbps * options.max_size_percent as f64 / 100.0 * VBV_SAFETY_MARGIN;
            let audio_share = if probe.has_audio {
                audio_kbps as f64
            } else {
                0.0
            };
            let fps = if probe.fps > 0.0 {
                probe.fps.min(60.0)
            } else {
                30.0
            };
            let floor_kbps =
                out_short as f64 * out_long as f64 * fps * codec.min_bits_per_pixel() / 1000.0;
            (budget_kbps - audio_share)
                .max(floor_kbps)
                .max(MIN_VIDEO_KBPS as f64)
                .round() as u32
        });

    VideoEncodePlan {
        codec,
        crf: codec.native_crf(options.crf.clamp(MIN_CRF, MAX_CRF)),
        max_video_kbps,
        scale_filter,
        audio_kbps,
    }
}

/// Construit les arguments FFmpeg (hors entrée et sortie) pour un plan et un conteneur
pub fn build_encode_args(plan: &VideoEncodePlan, ext: &str) -> Vec<String> {
    let mut args: Vec<String> = ["-map", "0:v:0?", "-map", "0:a?"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut push = |items: &[&str]| args.extend(items.iter().map(|s| s.to_string()));

    if let Some(filter) = &plan.scale_filter {
        push(&["-vf", filter]);
    }

    let crf = plan.crf.to_string();
    match plan.codec {
        VideoCodec::H264 | VideoCodec::H265 => {
            push(&["-c:v", plan.codec.encoder_name(), "-preset", "medium"]);
            push(&["-crf", &crf]);
            if let Some(kbps) = plan.max_video_kbps {
                let maxrate = format!("{kbps}k");
                let bufsize = format!("{}k", kbps * 2);
                push(&["-maxrate", &maxrate, "-bufsize", &bufsize]);
            }
            if plan.codec == VideoCodec::H265 {
                push(&["-x265-params", "log-level=error"]);
                if matches!(ext, "mp4" | "m4v" | "mov") {
                    // Étiquette hvc1 requise par QuickTime / iOS / macOS
                    push(&["-tag:v", "hvc1"]);
                }
            }
        }
        VideoCodec::Vp9 => {
            // Mode « qualité contrainte » : -b:v agit comme plafond, 0 = CRF pur
            let bitrate = plan
                .max_video_kbps
                .map(|k| format!("{k}k"))
                .unwrap_or_else(|| "0".into());
            push(&["-c:v", "libvpx-vp9", "-crf", &crf, "-b:v", &bitrate]);
            push(&["-deadline", "good", "-cpu-used", "4", "-row-mt", "1"]);
        }
    }
    push(&["-pix_fmt", "yuv420p"]);

    let audio_bitrate = format!("{}k", plan.audio_kbps);
    if ext == "webm" {
        let opus = if encoder_available("libopus") {
            "libopus"
        } else {
            "libvorbis"
        };
        push(&[
            "-c:a",
            opus,
            "-b:a",
            &audio_bitrate,
            "-ac",
            "2",
            "-ar",
            "48000",
        ]);
    } else {
        push(&["-c:a", "aac", "-b:a", &audio_bitrate, "-ac", "2"]);
    }

    match ext {
        "mp4" | "m4v" => push(&["-movflags", "+faststart", "-f", "mp4"]),
        "mov" => push(&["-movflags", "+faststart", "-f", "mov"]),
        "mkv" => push(&["-f", "matroska"]),
        "webm" => push(&["-f", "webm"]),
        _ => {}
    }
    args
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

    // Analyse de la source : durée, dimensions, cadence, audio
    let probe = probe_video(&job.input).await;
    let duration_us = (probe.duration_sec * 1_000_000.0).max(1.0) as u64;

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

    // Codec compatible avec le conteneur, puis repli H.264 si l'encodeur manque
    let requested = codec_for_extension(&ext, job.options.codec);
    let codec = if encoder_available(requested.encoder_name()) {
        requested
    } else {
        VideoCodec::H264
    };
    let plan = plan_encode(&probe, orig_size, &job.options, codec);
    let codec_label = codec.label();
    let ffmpeg_bin = resolve_ffmpeg_path();

    let mut cmd = Command::new(&ffmpeg_bin);
    cmd.arg("-y")
        .arg("-i")
        .arg(&job.input)
        .args(build_encode_args(&plan, &ext))
        .args(["-progress", "pipe:1", "-nostats"])
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

    let fallback_note = if codec != requested {
        format!(" ({} indisponible, repli {codec_label})", requested.label())
    } else {
        String::new()
    };
    progress_callback(
        5.0,
        0,
        &format!("Transcodage {codec_label} démarré...{fallback_note}"),
    );
    let encoding_stage = format!("Encodage {codec_label}...");

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
                    progress_callback(pct, current_size_bytes, &encoding_stage);
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

    let mut final_size = std::fs::metadata(&job.output)
        .map(|m| m.len())
        .unwrap_or(current_size_bytes);

    // Garde-fou : même conteneur et aucun gain -> livrer une copie de l'original
    let same_container = job
        .input
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(&ext));
    if same_container && final_size >= orig_size && orig_size > 0 {
        tokio::fs::copy(&job.input, &job.output)
            .await
            .map_err(|e| VideoEngineError::IoError(job.output.clone(), e))?;
        final_size = orig_size;
    }

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
    options: &VideoTranscodeOptions,
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
        let (output_ext, _) = resolve_output_format(input_path, options.container, options.codec);
        let dest_path = crate::export_engine::resolve_export_path(
            input_path,
            output_dir,
            export_mode,
            folder_timestamp,
            Some(output_ext),
        );

        let job = VideoJob {
            id: format!("video_{idx}"),
            input: input_path.to_path_buf(),
            output: dest_path.clone(),
            options: options.clone(),
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
        assert_eq!(opts.codec, VideoCodec::H264);
        assert_eq!(opts.container, VideoContainer::Auto);
        assert_eq!(opts.max_size_percent, 50);
        assert_eq!(opts.max_height, 0);
    }

    fn hevc_phone_probe() -> VideoProbe {
        VideoProbe {
            duration_sec: 60.0,
            width: 1920,
            height: 1080,
            fps: 30.0,
            video_codec: "hevc".into(),
            audio_bitrate_bps: Some(128_000),
            has_audio: true,
        }
    }

    #[test]
    fn test_plan_caps_bitrate_relative_to_source() {
        // 67 Mo sur 60 s = ~8,9 Mb/s ; plafond 50 % -> budget total ~4,2 Mb/s
        let probe = hevc_phone_probe();
        let size = 67 * 1024 * 1024;
        let plan = plan_encode(
            &probe,
            size,
            &VideoTranscodeOptions::default(),
            VideoCodec::H264,
        );
        let cap = plan.max_video_kbps.expect("plafond attendu");
        let expected_total_bytes = (cap + plan.audio_kbps) as f64 * 1000.0 / 8.0 * 60.0;
        let ratio = expected_total_bytes / size as f64;
        assert!(
            ratio <= 0.50,
            "le plafond doit viser <= 50 % (ratio {ratio})"
        );
        assert!(
            ratio >= 0.40,
            "le plafond ne doit pas sur-compresser (ratio {ratio})"
        );
        assert_eq!(plan.crf, 26);
        assert!(plan.scale_filter.is_none());
    }

    #[test]
    fn test_plan_without_cap_and_with_downscale() {
        let probe = VideoProbe {
            width: 1080,
            height: 1920, // vidéo portrait
            ..hevc_phone_probe()
        };
        let opts = VideoTranscodeOptions {
            max_size_percent: 100,
            max_height: 720,
            ..Default::default()
        };
        let plan = plan_encode(&probe, 50_000_000, &opts, VideoCodec::H265);
        assert_eq!(plan.max_video_kbps, None);
        assert_eq!(plan.crf, 29);
        assert!(plan.scale_filter.unwrap().contains("720"));
    }

    #[test]
    fn test_plan_respects_quality_floor() {
        // Source déjà très compressée : 5 Mo pour 60 s en 1080p
        let probe = hevc_phone_probe();
        let opts = VideoTranscodeOptions {
            max_size_percent: 20,
            ..Default::default()
        };
        let plan = plan_encode(&probe, 5_000_000, &opts, VideoCodec::H264);
        // Plancher H.264 : 1920 x 1080 x 30 x 0,008 / 1000 ~ 498 kb/s
        assert_eq!(plan.max_video_kbps, Some(498));
    }

    #[test]
    fn test_resolve_output_format() {
        let mp4 = Path::new("/v/clip.mp4");
        let avi = Path::new("/v/clip.avi");
        let webm = Path::new("/v/clip.webm");
        assert_eq!(
            resolve_output_format(mp4, VideoContainer::Auto, VideoCodec::H265),
            ("mp4", VideoCodec::H265)
        );
        assert_eq!(
            resolve_output_format(avi, VideoContainer::Auto, VideoCodec::H264),
            ("mp4", VideoCodec::H264)
        );
        assert_eq!(
            resolve_output_format(webm, VideoContainer::Auto, VideoCodec::H264),
            ("mp4", VideoCodec::H264)
        );
        assert_eq!(
            resolve_output_format(mp4, VideoContainer::Auto, VideoCodec::Vp9),
            ("webm", VideoCodec::Vp9)
        );
        assert_eq!(
            resolve_output_format(mp4, VideoContainer::Webm, VideoCodec::H264),
            ("webm", VideoCodec::Vp9)
        );
        assert_eq!(
            resolve_output_format(mp4, VideoContainer::Mov, VideoCodec::Vp9),
            ("mov", VideoCodec::H264)
        );
        assert_eq!(
            resolve_output_format(avi, VideoContainer::Mkv, VideoCodec::Vp9),
            ("mkv", VideoCodec::Vp9)
        );
    }

    #[test]
    fn test_build_encode_args_capped_hevc_mp4() {
        let plan = VideoEncodePlan {
            codec: VideoCodec::H265,
            crf: 29,
            max_video_kbps: Some(2000),
            scale_filter: None,
            audio_kbps: 128,
        };
        let args = build_encode_args(&plan, "mp4").join(" ");
        assert!(args.contains("-c:v libx265"));
        assert!(args.contains("-crf 29"));
        assert!(args.contains("-maxrate 2000k -bufsize 4000k"));
        assert!(args.contains("-tag:v hvc1"));
        assert!(args.contains("-c:a aac -b:a 128k"));
        assert!(args.contains("-f mp4"));
    }

    #[test]
    fn test_parse_ffprobe_json() {
        let sample = r#"{
            "streams": [
                {"codec_name": "hevc", "codec_type": "video", "width": 1920, "height": 1080,
                 "avg_frame_rate": "30000/1001", "bit_rate": "8123456"},
                {"codec_name": "aac", "codec_type": "audio", "bit_rate": "125000"}
            ],
            "format": {"duration": "66.733333"}
        }"#;
        let probe = parse_ffprobe_json(sample).expect("json valide");
        assert_eq!(probe.video_codec, "hevc");
        assert_eq!((probe.width, probe.height), (1920, 1080));
        assert!((probe.fps - 29.97).abs() < 0.01);
        assert!((probe.duration_sec - 66.73).abs() < 0.01);
        assert!(probe.has_audio);
        assert_eq!(probe.audio_bitrate_bps, Some(125_000));
    }

    #[test]
    fn test_parse_encoder_list() {
        let sample = " V....D libx264              libx264 H.264\n A....D aac                  AAC\n ------\n V..... = Video\n";
        let set = parse_encoder_list(sample);
        assert!(set.contains("libx264"));
        assert!(set.contains("aac"));
        assert!(!set.contains("="));
        assert_eq!(set.len(), 2);
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
                options: VideoTranscodeOptions::default(),
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
            options: VideoTranscodeOptions::default(),
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
                options: VideoTranscodeOptions {
                    crf: 28,
                    ..Default::default()
                },
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

    /// Régression : une source déjà compressée ré-encodée au CRF seul ne perdait
    /// que quelques pourcents (67 Mo -> 60 Mo). Le plafond de débit doit garantir
    /// une sortie <= max_size_percent de l'original.
    #[tokio::test]
    async fn test_already_compressed_source_is_really_reduced() {
        let temp_dir = std::env::temp_dir().join("go_compressor_video_cap_test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let input_path = temp_dir.join("grainy_source.mp4");
        let output_path = temp_dir.join("grainy_output.mp4");

        // Source granuleuse encodée au CRF 28 : le CRF 26 seul ne la réduirait pas
        let generated = Command::new(resolve_ffmpeg_path())
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=duration=4:size=640x360:rate=25,noise=alls=22:allf=t+u",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=4",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "28",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
            ])
            .arg(&input_path)
            .output()
            .await;
        if !generated.is_ok_and(|o| o.status.success()) {
            eprintln!("Génération de la source impossible - test ignoré.");
            return;
        }

        let job = VideoJob {
            id: "cap_job".into(),
            input: input_path.clone(),
            output: output_path.clone(),
            options: VideoTranscodeOptions::default(),
        };
        let report = encode_video_job(&job, |_, _, _| {})
            .await
            .expect("encodage plafonné");

        let ratio = report.output_size_bytes as f64 / report.original_size_bytes as f64;
        assert!(
            ratio <= 0.55,
            "sortie {} o pour source {} o (ratio {ratio:.2}) : plafond 50 % non respecté",
            report.output_size_bytes,
            report.original_size_bytes
        );
        assert_eq!(probe_video_has_audio(&output_path), Some(true));

        let _ = std::fs::remove_file(&input_path);
        let _ = std::fs::remove_file(&output_path);
    }
}
