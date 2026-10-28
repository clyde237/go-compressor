import { describe, it, expect, beforeEach } from 'vitest';
import {
  DEFAULT_BATCH_CONFIG,
  SETTINGS_STORAGE_KEY,
  EXPORT_DIR_STORAGE_KEY,
  sanitizeBatchConfig,
  loadSavedSettings,
  saveSettings,
  loadSavedExportDirectory,
  saveExportDirectory,
  resetSavedSettings,
} from './storage';
import { BatchConfig } from '../types/ipc';

describe('storage.ts - Persistance des préférences utilisateur', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('sanitizeBatchConfig retourne la configuration par défaut pour une entrée nulle ou invalide', () => {
    expect(sanitizeBatchConfig(null)).toEqual(DEFAULT_BATCH_CONFIG);
    expect(sanitizeBatchConfig('invalid')).toEqual(DEFAULT_BATCH_CONFIG);
    expect(sanitizeBatchConfig({})).toEqual(DEFAULT_BATCH_CONFIG);
  });

  it('sanitizeBatchConfig assainit les valeurs hors limites', () => {
    const corrupted = {
      q_min: -10, // invalide
      q_max: 200, // invalide
      threads: 99, // invalide
      crf: 99, // invalide
      export_mode: 'unsupported_mode', // invalide
      preserve_exif: 'not_a_bool', // invalide
      target_bytes: 10, // trop bas
    };

    const sanitized = sanitizeBatchConfig(corrupted);
    expect(sanitized.q_min).toBe(DEFAULT_BATCH_CONFIG.q_min);
    expect(sanitized.q_max).toBe(DEFAULT_BATCH_CONFIG.q_max);
    expect(sanitized.threads).toBe(DEFAULT_BATCH_CONFIG.threads);
    expect(sanitized.crf).toBe(DEFAULT_BATCH_CONFIG.crf);
    expect(sanitized.export_mode).toBe(DEFAULT_BATCH_CONFIG.export_mode);
    expect(sanitized.preserve_exif).toBe(DEFAULT_BATCH_CONFIG.preserve_exif);
    expect(sanitized.target_bytes).toBe(DEFAULT_BATCH_CONFIG.target_bytes);
  });

  it('loadSavedSettings retourne les valeurs par défaut quand localStorage est vide', () => {
    const loaded = loadSavedSettings();
    expect(loaded).toEqual(DEFAULT_BATCH_CONFIG);
  });

  it('sanitizeBatchConfig rejette les formats de sortie et plafonds vidéo invalides', () => {
    const sanitized = sanitizeBatchConfig({
      image_format: 'gif',
      video_codec: 'mpeg2',
      video_container: 'avi',
      video_max_size_percent: 5,
      video_max_height: 333,
    });
    expect(sanitized.image_format).toBe(DEFAULT_BATCH_CONFIG.image_format);
    expect(sanitized.video_codec).toBe(DEFAULT_BATCH_CONFIG.video_codec);
    expect(sanitized.video_container).toBe(DEFAULT_BATCH_CONFIG.video_container);
    expect(sanitized.video_max_size_percent).toBe(DEFAULT_BATCH_CONFIG.video_max_size_percent);
    expect(sanitized.video_max_height).toBe(DEFAULT_BATCH_CONFIG.video_max_height);
  });

  it('saveSettings persiste correctement les paramètres dans localStorage', () => {
    const customConfig: BatchConfig = {
      target_min_bytes: 800 * 1024,
      target_max_bytes: 2000 * 1024,
      target_bytes: 1500 * 1024,
      q_min: 70,
      q_max: 88,
      threads: 6,
      crf: 24,
      export_mode: 'folder',
      preserve_exif: false,
      output_directory: '/opt/media/export',
      image_format: 'webp',
      video_codec: 'h265',
      video_container: 'mkv',
      video_max_size_percent: 35,
      video_max_height: 720,
    };

    saveSettings(customConfig);
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    expect(raw).not.toBeNull();

    const loaded = loadSavedSettings();
    expect(loaded).toEqual(customConfig);
  });

  it('loadSavedExportDirectory et saveExportDirectory gèrent correctement le dernier dossier utilisé', () => {
    expect(loadSavedExportDirectory()).toBe('');

    saveExportDirectory('/home/user/Vidéos/Compressées');
    expect(loadSavedExportDirectory()).toBe('/home/user/Vidéos/Compressées');
    expect(localStorage.getItem(EXPORT_DIR_STORAGE_KEY)).toBe('/home/user/Vidéos/Compressées');
  });

  it('resetSavedSettings supprime les paramètres de localStorage et retourne la configuration par défaut', () => {
    const customConfig: BatchConfig = {
      ...DEFAULT_BATCH_CONFIG,
      crf: 28,
      export_mode: 'suffix',
    };
    saveSettings(customConfig);
    expect(localStorage.getItem(SETTINGS_STORAGE_KEY)).not.toBeNull();

    const resetResult = resetSavedSettings();
    expect(resetResult).toEqual(DEFAULT_BATCH_CONFIG);
    expect(localStorage.getItem(SETTINGS_STORAGE_KEY)).toBeNull();
  });
});
