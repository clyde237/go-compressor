/**
 * Contrats d'interfaces IPC (React <-> Rust)
 */

export type ExportMode = 'copy' | 'suffix' | 'folder';

export interface BatchSettings {
  targetMinBytes: number; // 800 * 1024
  targetMaxBytes: number; // 2 * 1024 * 1024
  targetBytes: number; // Valeur du slider cible (ex: 1104 Ko)
  qMin: number; // 65
  qMax: number; // 92
  threads: number; // 1 à 8
  crf: number; // 23 à 28
  mode: ExportMode;
  preserveExif: boolean;
  outputDirectory: string;
}

export interface CompressionProgressPayload {
  fileId: string;
  progressPercent: number;
  stage: string;
  currentSizeBytes?: number;
}

export interface SystemMetricsPayload {
  cpuUsage: number;
  usedMemoryGb: number;
  totalMemoryGb: number;
}

export interface BatchReportSummary {
  processedCount: number;
  averageGainPercent: number;
  totalSavedBytes: number;
}
