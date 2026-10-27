/**
 * Module de persistance locale des préférences utilisateur (Cahier des charges §06 & Phase 6.3)
 * Mémorisation non-bloquante et sécurisée dans localStorage.
 */

import { BatchConfig, ExportMode } from '../types/ipc';

export const SETTINGS_STORAGE_KEY = 'go_compressor_settings_v1';
export const EXPORT_DIR_STORAGE_KEY = 'go_compressor_export_dir_v1';

export const DEFAULT_BATCH_CONFIG: BatchConfig = {
  target_min_bytes: 800 * 1024,
  target_max_bytes: 2 * 1024 * 1024,
  target_bytes: 1104 * 1024,
  q_min: 65,
  q_max: 92,
  threads: 4,
  crf: 26,
  export_mode: 'copy',
  preserve_exif: true,
  output_directory: '',
};

const VALID_EXPORT_MODES: ExportMode[] = ['copy', 'suffix', 'folder'];

/**
 * Valide et assainit une configuration brute
 */
export function sanitizeBatchConfig(raw: unknown): BatchConfig {
  if (!raw || typeof raw !== 'object') {
    return { ...DEFAULT_BATCH_CONFIG };
  }

  const candidate = raw as Record<string, unknown>;

  const export_mode: ExportMode =
    typeof candidate.export_mode === 'string' &&
    VALID_EXPORT_MODES.includes(candidate.export_mode as ExportMode)
      ? (candidate.export_mode as ExportMode)
      : DEFAULT_BATCH_CONFIG.export_mode;

  const target_bytes =
    typeof candidate.target_bytes === 'number' &&
    candidate.target_bytes >= 100 * 1024 &&
    candidate.target_bytes <= 100 * 1024 * 1024
      ? candidate.target_bytes
      : DEFAULT_BATCH_CONFIG.target_bytes;

  const target_min_bytes =
    typeof candidate.target_min_bytes === 'number' && candidate.target_min_bytes > 0
      ? candidate.target_min_bytes
      : DEFAULT_BATCH_CONFIG.target_min_bytes;

  const target_max_bytes =
    typeof candidate.target_max_bytes === 'number' && candidate.target_max_bytes >= target_min_bytes
      ? candidate.target_max_bytes
      : DEFAULT_BATCH_CONFIG.target_max_bytes;

  const q_min =
    typeof candidate.q_min === 'number' && candidate.q_min >= 1 && candidate.q_min <= 100
      ? candidate.q_min
      : DEFAULT_BATCH_CONFIG.q_min;

  const q_max =
    typeof candidate.q_max === 'number' && candidate.q_max >= q_min && candidate.q_max <= 100
      ? candidate.q_max
      : DEFAULT_BATCH_CONFIG.q_max;

  const threads =
    typeof candidate.threads === 'number' && candidate.threads >= 1 && candidate.threads <= 32
      ? candidate.threads
      : DEFAULT_BATCH_CONFIG.threads;

  const crf =
    typeof candidate.crf === 'number' && candidate.crf >= 0 && candidate.crf <= 51
      ? candidate.crf
      : DEFAULT_BATCH_CONFIG.crf;

  const preserve_exif =
    typeof candidate.preserve_exif === 'boolean'
      ? candidate.preserve_exif
      : DEFAULT_BATCH_CONFIG.preserve_exif;

  const output_directory =
    typeof candidate.output_directory === 'string'
      ? candidate.output_directory
      : DEFAULT_BATCH_CONFIG.output_directory;

  return {
    target_min_bytes,
    target_max_bytes,
    target_bytes,
    q_min,
    q_max,
    threads,
    crf,
    export_mode,
    preserve_exif,
    output_directory,
  };
}

/**
 * Charge les paramètres persistés ou retourne la configuration par défaut
 */
export function loadSavedSettings(): BatchConfig {
  if (typeof window === 'undefined' || !window.localStorage) {
    return { ...DEFAULT_BATCH_CONFIG };
  }

  try {
    const raw = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (!raw) {
      return { ...DEFAULT_BATCH_CONFIG };
    }
    const parsed = JSON.parse(raw);
    return sanitizeBatchConfig(parsed);
  } catch (err) {
    console.warn('[Storage] Échec lecture localStorage settings :', err);
    return { ...DEFAULT_BATCH_CONFIG };
  }
}

/**
 * Sauvegarde les paramètres dans localStorage
 */
export function saveSettings(settings: BatchConfig): void {
  if (typeof window === 'undefined' || !window.localStorage) {
    return;
  }

  try {
    const sanitized = sanitizeBatchConfig(settings);
    window.localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(sanitized));
  } catch (err) {
    console.warn('[Storage] Échec écriture localStorage settings :', err);
  }
}

/**
 * Charge le dernier répertoire d'exportation mémorisé
 */
export function loadSavedExportDirectory(): string {
  if (typeof window === 'undefined' || !window.localStorage) {
    return '';
  }

  try {
    return window.localStorage.getItem(EXPORT_DIR_STORAGE_KEY) || '';
  } catch (err) {
    console.warn('[Storage] Échec lecture localStorage export directory :', err);
    return '';
  }
}

/**
 * Sauvegarde le dernier répertoire d'exportation mémorisé
 */
export function saveExportDirectory(dir: string): void {
  if (typeof window === 'undefined' || !window.localStorage) {
    return;
  }

  try {
    if (dir && dir.trim() !== '') {
      window.localStorage.setItem(EXPORT_DIR_STORAGE_KEY, dir.trim());
    }
  } catch (err) {
    console.warn('[Storage] Échec écriture localStorage export directory :', err);
  }
}

/**
 * Réinitialise les préférences au niveau de stockage
 */
export function resetSavedSettings(): BatchConfig {
  if (typeof window !== 'undefined' && window.localStorage) {
    try {
      window.localStorage.removeItem(SETTINGS_STORAGE_KEY);
    } catch (err) {
      console.warn('[Storage] Échec suppression localStorage settings :', err);
    }
  }
  return { ...DEFAULT_BATCH_CONFIG };
}
