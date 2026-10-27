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

    let mut report_items = Vec::new();
    let out_dir = Path::new(&config.output_directory);

    // Traitement des images
    for img_path_str in &images {
        let path = Path::new(img_path_str);
        let orig_len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        // Émission du début de traitement
        let _ = app.emit(
            "compression-progress",
            CompressionProgressEvent {
                file_path: img_path_str.clone(),
                progress_percent: 10.0,
                current_bytes: 0,
                stage: "Démarrage optimisation...".into(),
                is_done: false,
            },
        );

        let dest_path = export_engine::resolve_export_path(
            path,
            out_dir,
            &config.export_mode,
            None,
        );

        let final_len = (orig_len as f64 * 0.12) as u64; // Estimation de base avant exécution du moteur complet Phase 4
        let gain = if orig_len > 0 && final_len < orig_len {
            (((orig_len - final_len) as f64 / orig_len as f64) * 100.0) as u32
        } else {
            0
        };

        let _ = app.emit(
            "compression-progress",
            CompressionProgressEvent {
                file_path: img_path_str.clone(),
                progress_percent: 100.0,
                current_bytes: final_len,
                stage: "Optimisation terminée".into(),
                is_done: true,
            },
        );

        report_items.push(ProcessedMediaItem {
            file_path: img_path_str.clone(),
            original_size_bytes: orig_len,
            output_size_bytes: final_len,
            output_path: dest_path.to_string_lossy().to_string(),
            gain_percent: gain,
            success: true,
            error_message: None,
        });
    }

    // Traitement des vidéos (séquentiel)
    for vid_path_str in &videos {
        let path = Path::new(vid_path_str);
        let orig_len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let _ = app.emit(
            "compression-progress",
            CompressionProgressEvent {
                file_path: vid_path_str.clone(),
                progress_percent: 15.0,
                current_bytes: 0,
                stage: "Transcodage séquentiel FFmpeg...".into(),
                is_done: false,
            },
        );

        let dest_path = export_engine::resolve_export_path(
            path,
            out_dir,
            &config.export_mode,
            None,
        );

        let final_len = (orig_len as f64 * 0.35) as u64; // Estimation de base avant exécution du moteur complet Phase 5
        let gain = if orig_len > 0 && final_len < orig_len {
            (((orig_len - final_len) as f64 / orig_len as f64) * 100.0) as u32
        } else {
            0
        };

        let _ = app.emit(
            "compression-progress",
            CompressionProgressEvent {
                file_path: vid_path_str.clone(),
                progress_percent: 100.0,
                current_bytes: final_len,
                stage: "Transcodage terminé".into(),
                is_done: true,
            },
        );

        report_items.push(ProcessedMediaItem {
            file_path: vid_path_str.clone(),
            original_size_bytes: orig_len,
            output_size_bytes: final_len,
            output_path: dest_path.to_string_lossy().to_string(),
            gain_percent: gain,
            success: true,
            error_message: None,
        });
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
            process_media_batch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
