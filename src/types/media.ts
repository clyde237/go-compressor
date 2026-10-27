/**
 * Types pour les médias et les quotas du lot
 */

export type MediaKind = 'image' | 'video';

export type FileStatus = 'idle' | 'wait' | 'running' | 'done' | 'error';

export interface MediaItem {
  id: string;
  name: string;
  path: string;
  kind: MediaKind;
  format: string; // 'JPG' | 'PNG' | 'WebP' | 'MP4' | etc.
  sizeBytes: number; // Taille initiale réelle
  finalSizeBytes?: number; // Taille finale après optimisation
  outputPath?: string; // Chemin absolu du fichier optimisé sur le disque
  progress: number; // 0 à 100
  status: FileStatus;
  errorMessage?: string;
}

export interface QuotaLimits {
  readonly MAX_IMAGES: number;
  readonly MAX_VIDEOS: number;
  readonly TARGET_MIN_BYTES: number; // 800 Ko (819 200 octets)
  readonly TARGET_MAX_BYTES: number; // 2 Mo (2 097 152 octets)
}

export const QUOTAS: QuotaLimits = {
  MAX_IMAGES: 50,
  MAX_VIDEOS: 20,
  TARGET_MIN_BYTES: 800 * 1024,
  TARGET_MAX_BYTES: 2 * 1024 * 1024,
} as const;
