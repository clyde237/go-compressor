import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useMediaBatch } from './useMediaBatch';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

describe('useMediaBatch Hook', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });
  it('doit initialiser avec un état vide', () => {
    const { result } = renderHook(() => useMediaBatch());

    expect(result.current.files).toEqual([]);
    expect(result.current.running).toBe(false);
    expect(result.current.hasRun).toBe(false);
    expect(result.current.alert).toBeNull();
  });

  it('doit ajouter des images dans la file si le quota est respecté', () => {
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.addFilesFromPaths(['/images/photo1.jpg', '/images/photo2.png'], 'images');
    });

    expect(result.current.files.length).toBe(2);
    expect(result.current.files[0].name).toBe('photo1.jpg');
    expect(result.current.files[0].kind).toBe('image');
    expect(result.current.alert).toBeNull();
  });

  it('doit bloquer et lever une alerte si le quota de 20 vidéos est dépassé', () => {
    const { result } = renderHook(() => useMediaBatch());

    const twentyOneVideos = Array.from({ length: 21 }, (_, i) => `/videos/clip_${i}.mp4`);

    act(() => {
      result.current.addFilesFromPaths(twentyOneVideos, 'videos');
    });

    expect(result.current.files.length).toBe(0);
    expect(result.current.alert).not.toBeNull();
    expect(result.current.alert?.title).toBe('Quota vidéo dépassé');
    expect(result.current.alert?.body).toContain('20 vidéos maximum');
  });

  it('doit bloquer et lever une alerte si le quota de 50 photos est dépassé', () => {
    const { result } = renderHook(() => useMediaBatch());

    const fiftyOneImages = Array.from({ length: 51 }, (_, i) => `/images/img_${i}.jpg`);

    act(() => {
      result.current.addFilesFromPaths(fiftyOneImages, 'images');
    });

    expect(result.current.files.length).toBe(0);
    expect(result.current.alert).not.toBeNull();
    expect(result.current.alert?.title).toBe('Quota photos dépassé');
    expect(result.current.alert?.body).toContain('50 images maximum');
  });

  it('doit supprimer un élément par son id', () => {
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.addFilesFromPaths(['/images/photo1.jpg'], 'images');
    });

    const fileId = result.current.files[0].id;

    act(() => {
      result.current.removeItem(fileId);
    });

    expect(result.current.files.length).toBe(0);
  });

  it('doit vider la file par type de média', () => {
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.addFilesFromPaths(['/images/photo1.jpg'], 'images');
      result.current.addFilesFromPaths(['/videos/clip1.mp4'], 'videos');
    });

    expect(result.current.files.length).toBe(2);

    act(() => {
      result.current.clearQueue('images');
    });

    expect(result.current.files.length).toBe(1);
    expect(result.current.files[0].kind).toBe('video');
  });

  it('permet de définir le répertoire dexport et de fermer les alertes', () => {
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.setExportDirectory('/tmp/custom-export');
    });

    expect(result.current.exportDirectory).toBe('/tmp/custom-export');

    act(() => {
      // Simuler le dépassement pour lever une alerte
      result.current.addFilesFromPaths(
        Array.from({ length: 25 }, (_, i) => `/video_${i}.mp4`),
        'videos'
      );
    });

    expect(result.current.alert).not.toBeNull();

    act(() => {
      result.current.clearAlert();
    });

    expect(result.current.alert).toBeNull();
  });

  it('refuse le téléchargement ZIP si aucun fichier nest terminé', async () => {
    const { result } = renderHook(() => useMediaBatch());

    await act(async () => {
      await result.current.downloadZipArchive();
    });

    expect(result.current.alert?.title).toBe('Aucun fichier traité');
  });

  it('permet douvrir le dialogue simulé en environnement non-tauri', async () => {
    const { result } = renderHook(() => useMediaBatch());

    await act(async () => {
      await result.current.openNativeFileDialog('images');
    });

    expect(result.current.files.length).toBeGreaterThan(0);
    expect(result.current.files[0].kind).toBe('image');
  });

  it('permet de sélectionner un dossier dexport simulé', async () => {
    const { result } = renderHook(() => useMediaBatch());

    await act(async () => {
      await result.current.selectNativeExportDirectory();
    });

    expect(result.current.exportDirectory).toContain('/utilisateurs/export_');
  });

  it('gère louverture de dossier dexport en simulation et cas vide', async () => {
    const { result } = renderHook(() => useMediaBatch());

    await act(async () => {
      await result.current.openExportFolder();
    });
    expect(result.current.alert?.title).toBe('Emplacement introuvable');

    act(() => {
      result.current.clearAlert();
      result.current.setExportDirectory('/test/export');
    });

    await act(async () => {
      await result.current.openExportFolder();
    });
    expect(result.current.alert?.title).toBe('Ouverture du dossier');
  });

  it('gère lenregistrement individuel de fichier simulé', async () => {
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.addFilesFromPaths(['/images/photo1.jpg'], 'images');
    });

    const item = result.current.files[0];

    await act(async () => {
      await result.current.saveSingleFile(item);
    });

    expect(result.current.alert?.title).toBe('Fichier enregistré');
  });

  it('gère pauseCompression et clearQueue globale', () => {
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.addFilesFromPaths(['/images/photo1.jpg'], 'images');
    });

    expect(result.current.files.length).toBe(1);

    act(() => {
      result.current.pauseCompression();
      result.current.clearQueue();
    });

    expect(result.current.files.length).toBe(0);
    expect(result.current.running).toBe(false);
  });

  it('exécute startCompression en environnement de simulation web', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useMediaBatch());

    act(() => {
      result.current.addFilesFromPaths(['/images/photo1.jpg'], 'images');
    });

    const compressionPromise = act(async () => {
      await result.current.startCompression({
        target_min_bytes: 800 * 1024,
        target_max_bytes: 2000 * 1024,
        target_bytes: 1104 * 1024,
        q_min: 65,
        q_max: 92,
        threads: 4,
        crf: 26,
        preserve_exif: true,
        export_mode: 'copy',
        output_directory: '/export',
      });
    });

    // Avance le timer simulé
    act(() => {
      vi.advanceTimersByTime(1600);
    });

    await compressionPromise;

    expect(result.current.files[0].status).toBe('done');
    expect(result.current.running).toBe(false);
    expect(result.current.hasRun).toBe(true);
    vi.useRealTimers();
  });

  describe('useMediaBatch en environnement Tauri natif', () => {
    beforeEach(() => {
      // Activer le flag Tauri
      (window as unknown as { __TAURI_INTERNALS__: Record<string, unknown> }).__TAURI_INTERNALS__ = {};
      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === 'inspect_media_files') {
          return Promise.resolve([
            {
              path: '/path/source.jpg',
              size_bytes: 15 * 1024 * 1024,
              format: 'JPG',
              has_audio: null,
            },
            {
              path: '/path/photo.jpg',
              size_bytes: 10 * 1024 * 1024,
              format: 'JPG',
              has_audio: null,
            },
            {
              path: '/path/image.jpg',
              size_bytes: 8 * 1024 * 1024,
              format: 'JPG',
              has_audio: null,
            },
          ]);
        }
        if (cmd === 'get_system_metrics') {
          return Promise.resolve({ cpu_usage_percent: 12, memory_used_gb: 2.1 });
        }
        if (cmd === 'create_zip_archive') {
          return Promise.resolve(1024);
        }
        if (cmd === 'open_directory_in_explorer' || cmd === 'save_file_to') {
          return Promise.resolve();
        }
        if (cmd === 'process_media_batch') {
          return Promise.resolve({
            total_processed: 1,
            total_original_bytes: 15 * 1024 * 1024,
            total_output_bytes: 1.2 * 1024 * 1024,
            average_gain_percent: 92,
            items: [
              {
                file_path: '/path/source.jpg',
                success: true,
                original_size_bytes: 15 * 1024 * 1024,
                output_size_bytes: 1.2 * 1024 * 1024,
                output_path: '/path/source_opt.jpg',
                error_message: null,
              },
            ],
          });
        }
        return Promise.resolve({
          total_processed: 0,
          total_original_bytes: 0,
          total_output_bytes: 0,
          average_gain_percent: 0,
          items: [],
        });
      });
    });

    afterEach(() => {
      delete (window as unknown as { __TAURI_INTERNALS__?: Record<string, unknown> }).__TAURI_INTERNALS__;
    });

    it('ouvre la boîte de dialogue native pour importer des photos', async () => {
      vi.mocked(open).mockResolvedValueOnce(['/path/native_photo.jpg']);
      const { result } = renderHook(() => useMediaBatch());

      await act(async () => {
        await result.current.openNativeFileDialog('images');
      });

      expect(open).toHaveBeenCalledWith({
        multiple: true,
        directory: false,
        filters: [
          {
            name: 'Toutes les images',
            extensions: [
              'jpg',
              'jpeg',
              'png',
              'webp',
              'avif',
              'tiff',
              'tif',
              'bmp',
              'gif',
              'svg',
              'heic',
              'heif',
              'ico',
              'tga',
              'pnm',
              'raw',
              'cr2',
              'nef',
              'arw',
              'dng',
            ],
          },
          { name: 'Photos courantes', extensions: ['jpg', 'jpeg', 'png', 'webp'] },
        ],
      });
      expect(result.current.files.some((f) => f.name === 'native_photo.jpg')).toBe(true);
    });

    it('ouvre la boîte de dialogue native pour importer des vidéos avec tous les formats supportés', async () => {
      vi.mocked(open).mockResolvedValueOnce(['/path/clip_action.mkv']);
      const { result } = renderHook(() => useMediaBatch());

      await act(async () => {
        await result.current.openNativeFileDialog('videos');
      });

      expect(open).toHaveBeenCalledWith({
        multiple: true,
        directory: false,
        filters: [
          {
            name: 'Toutes les vidéos',
            extensions: [
              'mp4',
              'mov',
              'mkv',
              'avi',
              'wmv',
              'flv',
              'webm',
              'm4v',
              'ts',
              'mts',
              'm2ts',
              'mpg',
              'mpeg',
              '3gp',
              'ogv',
              'vob',
              'asf',
              'rmvb',
              'divx',
            ],
          },
          { name: 'Vidéos courantes', extensions: ['mp4', 'mov', 'mkv', 'avi'] },
        ],
      });
      expect(result.current.files.some((f) => f.name === 'clip_action.mkv')).toBe(true);
    });

    it('sélectionne un répertoire de sortie natif', async () => {
      vi.mocked(open).mockResolvedValueOnce('/dossier/destination/native');
      const { result } = renderHook(() => useMediaBatch());

      await act(async () => {
        await result.current.selectNativeExportDirectory();
      });

      expect(result.current.exportDirectory).toBe('/dossier/destination/native');
    });

    it('lance startCompression avec invoke et applique le rapport', async () => {
      const { result } = renderHook(() => useMediaBatch());

      act(() => {
        result.current.addFilesFromPaths(['/path/source.jpg'], 'images');
      });

      await act(async () => {
        await result.current.startCompression({
          target_min_bytes: 800 * 1024,
          target_max_bytes: 2000 * 1024,
          target_bytes: 1104 * 1024,
          q_min: 65,
          q_max: 92,
          threads: 4,
          crf: 26,
          preserve_exif: true,
          export_mode: 'copy',
          output_directory: '/export',
        });
      });

      expect(invoke).toHaveBeenCalledWith('process_media_batch', expect.anything());
      expect(result.current.files[0].status).toBe('done');
      expect(result.current.files[0].outputPath).toBe('/path/source_opt.jpg');
    });

    it('télécharge une archive ZIP via Tauri dialog save et invoke', async () => {
      vi.mocked(save).mockResolvedValueOnce('/path/archive.zip');
      vi.mocked(invoke).mockResolvedValueOnce(1024);

      const { result } = renderHook(() => useMediaBatch());

      act(() => {
        result.current.addFilesFromPaths(['/path/photo.jpg'], 'images');
      });

      // Mettre le fichier en statut "done"
      act(() => {
        (result.current.files[0] as { status: string; outputPath?: string }).status = 'done';
        (result.current.files[0] as { status: string; outputPath?: string }).outputPath = '/path/photo_opt.jpg';
      });

      await act(async () => {
        await result.current.downloadZipArchive();
      });

      expect(save).toHaveBeenCalledWith({
        defaultPath: 'lot_optimise.zip',
        filters: [{ name: 'Archive ZIP', extensions: ['zip'] }],
      });
      expect(invoke).toHaveBeenCalledWith('create_zip_archive', {
        files: ['/path/photo_opt.jpg'],
        destination: '/path/archive.zip',
      });
      expect(result.current.alert?.title).toBe('Archive enregistrée avec succès');
    });

    it('ouvre le dossier dans lexplorateur natif', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);
      const { result } = renderHook(() => useMediaBatch());

      await act(async () => {
        await result.current.openExportFolder('/mon/dossier');
      });

      expect(invoke).toHaveBeenCalledWith('open_directory_in_explorer', {
        path: '/mon/dossier',
      });
    });

    it('enregistre un fichier individuel via Tauri', async () => {
      vi.mocked(save).mockResolvedValueOnce('/dest/image_perso.jpg');
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useMediaBatch());

      act(() => {
        result.current.addFilesFromPaths(['/path/image.jpg'], 'images');
      });

      await act(async () => {
        await result.current.saveSingleFile(result.current.files[0]);
      });

      expect(save).toHaveBeenCalledWith({
        defaultPath: 'image.jpg',
        filters: [{ name: 'Média JPG', extensions: ['jpg'] }],
      });
      expect(invoke).toHaveBeenCalledWith('save_file_to', {
        source: '/path/image.jpg',
        destination: '/dest/image_perso.jpg',
      });
      expect(result.current.alert?.title).toBe('Fichier enregistré avec succès');
    });

    it('met à jour la progression lors de la réception dun événement compression-progress', async () => {
      let progressCallback: ((event: { payload: unknown }) => void) | null = null;

      vi.mocked(listen).mockImplementationOnce((event: string, cb: unknown) => {
        if (event === 'compression-progress') {
          progressCallback = cb as (event: { payload: unknown }) => void;
        }
        return Promise.resolve(() => {});
      });

      const { result } = renderHook(() => useMediaBatch());

      await act(async () => {
        result.current.addFilesFromPaths(['/path/image.jpg'], 'images');
      });

      expect(progressCallback).not.toBeNull();

      act(() => {
        progressCallback!({
          payload: {
            file_path: '/path/image.jpg',
            progress_percent: 50,
            stage: 'compression',
            current_bytes: 1000000,
            is_done: false,
            is_error: false,
            error_message: null,
          },
        });
      });

      expect(result.current.files[0].progress).toBe(50);
      expect(result.current.files[0].status).toBe('running');

      act(() => {
        progressCallback!({
          payload: {
            file_path: '/path/image.jpg',
            progress_percent: 100,
            stage: 'done',
            current_bytes: 900000,
            is_done: true,
            is_error: false,
            error_message: null,
          },
        });
      });

      expect(result.current.files[0].progress).toBe(100);
      expect(result.current.files[0].status).toBe('done');
      expect(result.current.files[0].finalSizeBytes).toBe(900000);
    });
  });
});
