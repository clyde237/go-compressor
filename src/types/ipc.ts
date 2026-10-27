/**
 * Contrats d'interfaces IPC (React <-> Rust)
 * Conforme au Cahier des Charges §04
 */

export type ExportMode = 'copy' | 'suffix' | 'folder';

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
  crf: number; // 23 à 28
  export_mode: ExportMode;
  preserve_exif: boolean;
  output_directory: string;
}

export interface CompressionProgressPayload {
  file_path: string;
  progress_percent: number; // 0.0 à 100.0
  current_bytes: number;
  stage: string;
  is_done: boolean;
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
