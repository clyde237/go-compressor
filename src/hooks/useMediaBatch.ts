import { useState, useEffect, useCallback, useRef } from 'react';
import { MediaItem, QUOTAS } from '../types/media';
import { BatchConfig, CompressionProgressPayload, SystemMetrics, BatchReport } from '../types/ipc';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog';

export interface UseMediaBatchResult {
  files: MediaItem[];
  running: boolean;
  hasRun: boolean;
  cpu: number;
  ram: number;
  exportDirectory: string;
  alert: { title: string; body: string; detail?: string } | null;
  clearAlert: () => void;
  addFilesFromPaths: (paths: string[], kind: 'images' | 'videos') => void;
  openNativeFileDialog: (kind: 'images' | 'videos') => Promise<void>;
  selectNativeExportDirectory: () => Promise<void>;
  downloadZipArchive: () => Promise<void>;
  openExportFolder: (customPath?: string) => Promise<void>;
  saveSingleFile: (item: MediaItem) => Promise<void>;
  removeItem: (id: string) => void;
  clearQueue: (kind?: 'images' | 'videos') => void;
  startCompression: (config: BatchConfig) => Promise<BatchReport | null>;
  pauseCompression: () => void;
  setExportDirectory: (dir: string) => void;
}

export function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function useMediaBatch(): UseMediaBatchResult {
  const [files, setFiles] = useState<MediaItem[]>([]);
  const [running, setRunning] = useState<boolean>(false);
  const [hasRun, setHasRun] = useState<boolean>(false);
  const [cpu, setCpu] = useState<number>(0);
  const [ram, setRam] = useState<number>(0);
  const [exportDirectory, setExportDirectory] = useState<string>('');
  const [alert, setAlert] = useState<{ title: string; body: string; detail?: string } | null>(null);

  const runningRef = useRef(running);
  runningRef.current = running;

  const clearAlert = useCallback(() => {
    setAlert(null);
  }, []);

  // Ajout de fichiers par chemins avec contrôle des quotas et inspection réelle des métadonnées
  const addFilesFromPaths = useCallback(
    (paths: string[], kind: 'images' | 'videos') => {
      if (paths.length === 0) return;

      let shouldBlock = false;

      setFiles((prev) => {
        const currentImgs = prev.filter((f) => f.kind === 'image').length;
        const currentVids = prev.filter((f) => f.kind === 'video').length;

        if (kind === 'videos') {
          if (currentVids + paths.length > QUOTAS.MAX_VIDEOS) {
            setAlert({
              title: 'Quota vidéo dépassé',
              body: `Limite dépassée : ${QUOTAS.MAX_VIDEOS} vidéos maximum par lot.`,
              detail: `Vous tentez d'ajouter ${paths.length} vidéo(s) alors que la file en contient déjà ${currentVids}. Le quota strict est fixé à ${QUOTAS.MAX_VIDEOS} pour protéger la mémoire vive et le processeur.`,
            });
            shouldBlock = true;
            return prev;
          }
        } else {
          if (currentImgs + paths.length > QUOTAS.MAX_IMAGES) {
            setAlert({
              title: 'Quota photos dépassé',
              body: `Limite dépassée : ${QUOTAS.MAX_IMAGES} images maximum par lot.`,
              detail: `Vous tentez d'ajouter ${paths.length} photo(s) alors que la file en contient déjà ${currentImgs}. Le quota maximal par lot est de ${QUOTAS.MAX_IMAGES}.`,
            });
            shouldBlock = true;
            return prev;
          }
        }

        const newItems: MediaItem[] = paths.map((p, idx) => {
          const parts = p.split(/[\/\\]/);
          const fileName = parts[parts.length - 1] || 'fichier';
          const extParts = fileName.split('.');
          const ext = extParts.length > 1 ? extParts[extParts.length - 1].toUpperCase() : 'INCONNU';

          return {
            id: `${Date.now()}_${idx}_${fileName}`,
            name: fileName,
            path: p,
            kind: kind === 'images' ? 'image' : 'video',
            format: ext,
            sizeBytes: 0,
            progress: 0,
            status: 'wait',
          };
        });

        return [...prev, ...newItems];
      });

      if (shouldBlock) return;

      // Sonde asynchrone des métadonnées réelles via la commande Rust
      if (isTauriEnvironment()) {
        invoke<{ path: string; name: string; size_bytes: number; extension: string }[]>(
          'inspect_media_files',
          { paths }
        )
          .then((inspected) => {
            setFiles((prev) =>
              prev.map((f) => {
                const info = inspected.find((item) => item.path === f.path);
                return info ? { ...f, sizeBytes: info.size_bytes } : f;
              })
            );
          })
          .catch(() => {
            // Fallback silencieux en cas d'erreur de sonde
          });
      }
    },
    []
  );

  // Ouverture de la boîte de dialogue native pour importer des médias
  const openNativeFileDialog = useCallback(
    async (kind: 'images' | 'videos') => {
      if (!isTauriEnvironment()) {
        // Mode simulation browser
        const mockPaths =
          kind === 'images'
            ? [`/sample/photo_${Date.now()}.jpg`, `/sample/photo_${Date.now() + 1}.png`]
            : [`/sample/clip_${Date.now()}.mp4`];
        addFilesFromPaths(mockPaths, kind);
        return;
      }

      try {
        const filters =
          kind === 'images'
            ? [{ name: 'Photos', extensions: ['jpg', 'jpeg', 'png', 'webp'] }]
            : [{ name: 'Vidéos', extensions: ['mp4', 'mov', 'mkv', 'avi'] }];

        const selected = await open({
          multiple: true,
          directory: false,
          filters,
        });

        if (selected) {
          const paths = Array.isArray(selected) ? selected : [selected];
          addFilesFromPaths(paths, kind);
        }
      } catch (err) {
        setAlert({
          title: "Erreur lors de l'ouverture du sélecteur",
          body: String(err),
        });
      }
    },
    [addFilesFromPaths]
  );

  // Sélection native du répertoire d'export
  const selectNativeExportDirectory = useCallback(async () => {
    if (!isTauriEnvironment()) {
      setExportDirectory(`/utilisateurs/export_${Date.now()}`);
      return;
    }

    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });

      if (selected && typeof selected === 'string') {
        setExportDirectory(selected);
      }
    } catch (err) {
      setAlert({
        title: 'Erreur sélection répertoire',
        body: String(err),
      });
    }
  }, []);

  // Téléchargement d'une archive ZIP du lot (pour les lots groupés de 5 fichiers et plus)
  const downloadZipArchive = useCallback(async () => {
    // Utiliser impérativement les chemins de sortie des fichiers optimisés
    const donePaths = files
      .filter((f) => f.status === 'done')
      .map((f) => f.outputPath || f.path);

    if (donePaths.length === 0) {
      setAlert({
        title: 'Aucun fichier traité',
        body: 'Aucun média optimisé disponible pour générer une archive.',
      });
      return;
    }

    if (!isTauriEnvironment()) {
      setAlert({
        title: 'Archive ZIP générée',
        body: 'Téléchargement simulé dans le navigateur avec succès.',
      });
      return;
    }

    try {
      const savePath = await save({
        defaultPath: 'lot_optimise.zip',
        filters: [{ name: 'Archive ZIP', extensions: ['zip'] }],
      });

      if (savePath) {
        await invoke('create_zip_archive', {
          files: donePaths,
          destination: savePath,
        });

        setAlert({
          title: 'Archive enregistrée avec succès',
          body: `L'archive ZIP (${donePaths.length} fichier(s) optimisé(s)) a été enregistrée sous : ${savePath}`,
        });
      }
    } catch (err) {
      setAlert({
        title: "Erreur lors de la création de l'archive",
        body: String(err),
      });
    }
  }, [files]);

  // Ouverture du dossier d'export ou du répertoire du média optimisé dans l'explorateur natif
  const openExportFolder = useCallback(async (customPath?: string) => {
    const defaultExport = exportDirectory || (files.find((f) => f.outputPath)?.outputPath) || '';
    const target = customPath || defaultExport;

    if (!target) {
      setAlert({
        title: 'Emplacement introuvable',
        body: "Aucun dossier d'export ou fichier optimisé n'est disponible.",
      });
      return;
    }

    if (isTauriEnvironment()) {
      try {
        await invoke('open_directory_in_explorer', { path: target });
      } catch (err) {
        setAlert({
          title: "Impossible d'ouvrir le dossier",
          body: String(err),
        });
      }
    } else {
      setAlert({
        title: 'Ouverture du dossier',
        body: `Ouverture simulée du dossier : ${target}`,
      });
    }
  }, [exportDirectory, files]);

  // Enregistrement individuel d'un fichier optimisé
  const saveSingleFile = useCallback(async (item: MediaItem) => {
    const sourcePath = item.outputPath || item.path;

    if (!isTauriEnvironment()) {
      setAlert({
        title: 'Fichier enregistré',
        body: `Téléchargement individuel simulé pour ${item.name}`,
      });
      return;
    }

    try {
      const ext = item.format.toLowerCase();
      const savePath = await save({
        defaultPath: item.name,
        filters: [{ name: `Média ${item.format}`, extensions: [ext] }],
      });

      if (savePath) {
        await invoke('save_file_to', {
          source: sourcePath,
          destination: savePath,
        });

        setAlert({
          title: 'Fichier enregistré avec succès',
          body: `Le fichier optimisé a été enregistré sous : ${savePath}`,
        });
      }
    } catch (err) {
      setAlert({
        title: "Erreur lors de l'enregistrement",
        body: String(err),
      });
    }
  }, []);

  const removeItem = useCallback((id: string) => {
    setFiles((prev) => prev.filter((item) => item.id !== id));
  }, []);

  const clearQueue = useCallback((kind?: 'images' | 'videos') => {
    if (!kind) {
      setFiles([]);
      setRunning(false);
      return;
    }
    const targetKind = kind === 'images' ? 'image' : 'video';
    setFiles((prev) => prev.filter((item) => item.kind !== targetKind));
  }, []);

  const pauseCompression = useCallback(() => {
    setRunning(false);
  }, []);

  // Lancement de la compression par lots
  const startCompression = useCallback(
    async (config: BatchConfig): Promise<BatchReport | null> => {
      const pendingItems = files.filter((f) => f.status !== 'done');
      if (pendingItems.length === 0) return null;

      setRunning(true);
      setHasRun(true);

      const images = files.filter((f) => f.kind === 'image').map((f) => f.path);
      const videos = files.filter((f) => f.kind === 'video').map((f) => f.path);

      if (!isTauriEnvironment()) {
        // Simulation en environnement Web de développement
        return new Promise((resolve) => {
          setTimeout(() => {
            setFiles((prev) =>
              prev.map((f) => ({
                ...f,
                status: 'done',
                progress: 100,
                finalSizeBytes: f.kind === 'image' ? 1.1 * 1024 * 1024 : f.sizeBytes * 0.35,
              }))
            );
            setRunning(false);
            setCpu(8);
            setRam(2.4);
            resolve({
              total_processed: images.length + videos.length,
              total_original_bytes: 50 * 1024 * 1024,
              total_output_bytes: 8 * 1024 * 1024,
              average_gain_percent: 84,
              items: [],
            });
          }, 1500);
        });
      }

      try {
        const report = await invoke<BatchReport>('process_media_batch', {
          images,
          videos,
          config: {
            ...config,
            output_directory: exportDirectory || config.output_directory,
          },
        });

        // Mise à jour de l'état avec les résultats retournés
        setFiles((prev) =>
          prev.map((f) => {
            const reportItem = report.items.find((r) => r.file_path === f.path);
            if (reportItem) {
              return {
                ...f,
                status: reportItem.success ? 'done' : 'error',
                progress: 100,
                sizeBytes: reportItem.original_size_bytes > 0 ? reportItem.original_size_bytes : f.sizeBytes,
                finalSizeBytes: reportItem.output_size_bytes,
                outputPath: reportItem.output_path,
                errorMessage: reportItem.error_message,
              };
            }
            return f;
          })
        );

        setRunning(false);
        return report;
      } catch (err) {
        setRunning(false);
        setAlert({
          title: 'Erreur lors du traitement du lot',
          body: String(err),
        });
        return null;
      }
    },
    [files, exportDirectory]
  );

  // Écoute temps réel des événements IPC et de la télémétrie
  useEffect(() => {
    let unlistenProgress: UnlistenFn | null = null;

    if (isTauriEnvironment()) {
      listen<CompressionProgressPayload>('compression-progress', (event) => {
        const payload = event.payload;
        setFiles((prev) =>
          prev.map((f) => {
            if (f.path === payload.file_path) {
              return {
                ...f,
                progress: payload.progress_percent,
                status: payload.is_done ? 'done' : 'running',
                finalSizeBytes: payload.is_done && payload.current_bytes > 0 ? payload.current_bytes : f.finalSizeBytes,
              };
            }
            return f;
          })
        );
      }).then((unlisten) => {
        unlistenProgress = unlisten;
      });
    }

    return () => {
      if (unlistenProgress) {
        unlistenProgress();
      }
    };
  }, []);

  // Polling télémétrique CPU / RAM périodique pendant la compression
  useEffect(() => {
    let intervalId: ReturnType<typeof setInterval> | null = null;

    if (isTauriEnvironment() && running) {
      intervalId = setInterval(async () => {
        try {
          const metrics = await invoke<SystemMetrics>('get_system_metrics');
          setCpu(Math.round(metrics.cpu_usage));
          setRam(Math.round(metrics.used_memory_gb * 10) / 10);
        } catch {
          // Fallback silencieux en cas d'erreur de sonde
        }
      }, 1000);
    } else if (!running) {
      setCpu(8);
      setRam(2.4);
    }

    return () => {
      if (intervalId) {
        clearInterval(intervalId);
      }
    };
  }, [running]);

  return {
    files,
    running,
    hasRun,
    cpu,
    ram,
    exportDirectory,
    alert,
    clearAlert,
    addFilesFromPaths,
    openNativeFileDialog,
    selectNativeExportDirectory,
    downloadZipArchive,
    openExportFolder,
    saveSingleFile,
    removeItem,
    clearQueue,
    startCompression,
    pauseCompression,
    setExportDirectory,
  };
}
