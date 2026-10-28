/**
 * Contrats d'interfaces IPC (React <-> Rust)
 * Conforme au Cahier des Charges §04
 */

export type ExportMode = 'copy' | 'suffix' | 'folder';

/** Format des images produites ('auto' = format source si JPEG/PNG/WebP, sinon JPEG) */
export type ImageFormat = 'auto' | 'jpeg' | 'webp' | 'png';

/** Codec vidéo de sortie */
export type VideoCodec = 'h264' | 'h265' | 'vp9';

/** Conteneur vidéo de sortie ('auto' = conteneur source si MP4/MKV/MOV/WebM, sinon MP4) */
export type VideoContainer = 'auto' | 'mp4' | 'mkv' | 'mov' | 'webm';

export interface BatchSettings {
  targetMinBytes: number;
  targetMaxBytes: number;
  targetBytes: number;
  qMin: number;
  qMax: number;
  threads: number;
  crf: number;
  mode: ExportMode;
  preserveExif: boolean;
  outputDirectory: string;
}

export interface BatchConfig {
  target_min_bytes: number; // 800 * 1024
  target_max_bytes: number; // 2 * 1024 * 1024
  target_bytes: number; // Valeur du slider cible (ex: 1104 Ko)
  q_min: number; // 65
  q_max: number; // 92
  threads: number; // 1 à 8
  crf: number; // 18 à 32 (échelle x264, convertie par codec)
  export_mode: ExportMode;
  preserve_exif: boolean;
  output_directory: string;
  image_format: ImageFormat;
  video_codec: VideoCodec;
  video_container: VideoContainer;
  video_max_size_percent: number; // 10 à 100 (100 = pas de plafond de débit)
  video_max_height: number; // plus petit côté max en pixels, 0 = résolution d'origine
}

export interface CompressionProgressPayload {
  file_path: string;
  progress_percent: number; // 0.0 à 100.0
  current_bytes: number;
  stage: string;
  is_done: boolean;
  is_error: boolean;
  error_message?: string;
}

export interface ProcessedMediaItemReport {
  file_path: string;
  original_size_bytes: number;
  output_size_bytes: number;
  output_path: string;
  gain_percent: number;
  success: boolean;
  error_message?: string;
}

export interface BatchReport {
  items: ProcessedMediaItemReport[];
  total_processed: number;
  total_original_bytes: number;
  total_output_bytes: number;
  average_gain_percent: number;
}

export interface SystemMetrics {
  cpu_usage: number; // Pourcentage CPU
  used_memory_gb: number; // RAM utilisée en Go
  total_memory_gb: number; // RAM totale en Go
}
