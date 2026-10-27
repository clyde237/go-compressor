pub mod export_engine;
pub mod image_engine;
pub mod metrics_engine;
pub mod video_engine;

use std::path::{Path, PathBuf};
use tauri::Emitter;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchConfig {
    pub target_min_bytes: u64,
    pub target_max_bytes: u64,
    pub target_bytes: u64,
    pub q_min: u8,
    pub q_max: u8,
    pub threads: usize,
    pub crf: u8,
    pub export_mode: export_engine::ExportMode,
    pub preserve_exif: bool,
    pub output_directory: String,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            target_min_bytes: image_engine::TARGET_MIN_BYTES,
            target_max_bytes: image_engine::TARGET_MAX_BYTES,
            target_bytes: 1_104 * 1024,
            q_min: 65,
            q_max: 92,
            threads: 4,
            crf: 26,
            export_mode: export_engine::ExportMode::Copy,
            preserve_exif: true,
            output_directory: String::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessedMediaItem {
    pub file_path: String,
    pub original_size_bytes: u64,
    pub output_size_bytes: u64,
    pub output_path: String,
    pub gain_percent: u32,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchReport {
    pub items: Vec<ProcessedMediaItem>,
    pub total_processed: usize,
    pub total_original_bytes: u64,
    pub total_output_bytes: u64,
    pub average_gain_percent: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompressionProgressEvent {
    pub file_path: String,
    pub progress_percent: f32,
    pub current_bytes: u64,
    pub stage: String,
    pub is_done: bool,
}

/// Récupère l'utilisation CPU et RAM en temps réel
#[tauri::command]
fn get_system_metrics(
    state: tauri::State<'_, metrics_engine::MetricsState>,
) -> Result<metrics_engine::SystemMetrics, String> {
    let mut sys = state.0.lock().map_err(|e| e.to_string())?;
    Ok(metrics_engine::get_current_metrics(&mut sys))
}

/// Métadonnées réelles d'un fichier lues sur le système de fichiers
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct MediaFileInfo {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub extension: String,
}

/// Inspecte une liste de chemins de fichiers et retourne leurs métadonnées réelles depuis le système de fichiers
#[tauri::command]
fn inspect_media_files(paths: Vec<String>) -> Result<Vec<MediaFileInfo>, String> {
    let mut results = Vec::with_capacity(paths.len());
    for p in paths {
        let path = Path::new(&p);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("fichier")
            .to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_uppercase();
        let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        results.push(MediaFileInfo {
            path: p,
            name,
            size_bytes,
            extension: ext,
        });
    }
    Ok(results)
}

/// Ouvre le dossier d'export ou le fichier dans l'explorateur natif de l'OS
#[tauri::command]
fn open_directory_in_explorer(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    let target = if p.is_file() {
        p.parent().unwrap_or(p)
    } else if p.exists() {
        p
    } else if let Some(parent) = p.parent() {
        parent
    } else {
        p
    };

    if !target.exists() {
        return Err(format!("L'emplacement spécifié n'existe pas : {}", target.display()));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Enregistre une copie d'un fichier optimisé vers une destination choisie
#[tauri::command]
fn save_file_to(source: String, destination: String) -> Result<(), String> {
    let src = Path::new(&source);
    let dst = Path::new(&destination);
    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::copy(src, dst).map_err(|e| e.to_string())?;
    Ok(())
}

/// Génère une archive ZIP contenant tous les fichiers spécifiés
#[tauri::command]
fn create_zip_archive(files: Vec<String>, destination: String) -> Result<u64, String> {
    let paths: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    export_engine::create_zip_archive(&paths, Path::new(&destination))
}

/// Commande principale de compression par lots (contrat IPC conforme CDC §04)
#[tauri::command]
async fn process_media_batch(
    app: tauri::AppHandle,
    images: Vec<String>,
    videos: Vec<String>,
    config: BatchConfig,
) -> Result<BatchReport, String> {
    // 1. Contrôle strict des quotas en amont
    if videos.len() > video_engine::MAX_VIDEOS {
        return Err(format!(
            "Limite dépassée : {} vidéos maximum par lot.",
            video_engine::MAX_VIDEOS
        ));
    }

    if images.len() > image_engine::MAX_IMAGES {
        return Err(format!(
            "Limite dépassée : {} images maximum par lot.",
            image_engine::MAX_IMAGES
        ));
    }

    let out_dir = Path::new(&config.output_directory);

    // 2. Traitement des images parallélisé avec Rayon (Phase 4)
    let image_opts = image_engine::ImageCompressionOptions {
        target_min_bytes: config.target_min_bytes,
        target_max_bytes: config.target_max_bytes,
        target_bytes: config.target_bytes,
        q_min: config.q_min,
        q_max: config.q_max,
        threads: config.threads,
        preserve_exif: config.preserve_exif,
    };

    let app_handle = app.clone();
    let image_reports = image_engine::compress_images_batch(
        &images,
        out_dir,
        &config.export_mode,
        &image_opts,
        move |file_path, progress, current_bytes, stage, is_done| {
            let _ = app_handle.emit(
                "compression-progress",
                CompressionProgressEvent {
                    file_path,
                    progress_percent: progress,
                    current_bytes,
                    stage: stage.to_string(),
                    is_done,
                },
            );
        },
    );

    let mut report_items = image_reports;

    // 3. Traitement des vidéos (séquentiel FIFO - Phase 5)
    if !videos.is_empty() {
        let app_handle_video = app.clone();
        let video_reports = video_engine::transcode_videos_sequential(
            &videos,
            out_dir,
            &config.export_mode,
            config.crf,
            move |file_path, progress, current_bytes, stage, is_done| {
                let _ = app_handle_video.emit(
                    "compression-progress",
                    CompressionProgressEvent {
                        file_path,
                        progress_percent: progress,
                        current_bytes,
                        stage: stage.to_string(),
                        is_done,
                    },
                );
            },
        )
        .await
        .map_err(|e| e.to_string())?;

        report_items.extend(video_reports);
    }

    let total_orig: u64 = report_items.iter().map(|i| i.original_size_bytes).sum();
    let total_out: u64 = report_items.iter().map(|i| i.output_size_bytes).sum();
    let avg_gain = if !report_items.is_empty() {
        (report_items.iter().map(|i| i.gain_percent as u64).sum::<u64>()
            / report_items.len() as u64) as u32
    } else {
        0
    };

    Ok(BatchReport {
        total_processed: report_items.len(),
        total_original_bytes: total_orig,
        total_output_bytes: total_out,
        average_gain_percent: avg_gain,
        items: report_items,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(metrics_engine::MetricsState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            get_system_metrics,
            create_zip_archive,
            process_media_batch,
            inspect_media_files,
            open_directory_in_explorer,
            save_file_to
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
