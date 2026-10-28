import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MediaItem } from '../../types/media';
import { DashboardView, OutputSettings } from './DashboardView';

const OUTPUT_SETTINGS: OutputSettings = {
  image_format: 'jpeg',
  video_codec: 'h264',
  video_container: 'auto',
  video_max_size_percent: 50,
  video_max_height: 0,
};

const MOCK_ITEMS: MediaItem[] = [
  {
    id: '1',
    name: 'photo_test.jpg',
    path: '/path/photo_test.jpg',
    kind: 'image',
    format: 'JPG',
    sizeBytes: 15 * 1024 * 1024,
    finalSizeBytes: 1.2 * 1024 * 1024,
    progress: 100,
    status: 'done',
  },
  {
    id: '2',
    name: 'video_test.mp4',
    path: '/path/video_test.mp4',
    kind: 'video',
    format: 'MP4',
    sizeBytes: 50 * 1024 * 1024,
    progress: 0,
    status: 'wait',
  },
];

describe('DashboardView Component', () => {
  it('doit afficher les deux sous-onglets Photos & Images et Vidéos', () => {
    render(
      <DashboardView
        items={MOCK_ITEMS}
        activeMediaTab="images"
        onMediaTabChange={vi.fn()}
        folders={[{ label: 'Dossier 1', value: '0' }]}
        destValue="0"
        onDestChange={vi.fn()}
        onBrowseFolder={vi.fn()}
        targetKo={1104}
        onTargetChange={vi.fn()}
        onAddBatch={vi.fn()}
        onClearQueue={vi.fn()}
        onRemoveItem={vi.fn()}
        running={false}
        hasRun={false}
        cpu={12}
        ram={2.1}
        onStart={vi.fn()}
        onPause={vi.fn()}
        onDownloadZip={vi.fn()}
        crf={26}
        exportMode="copy"
        outputSettings={OUTPUT_SETTINGS}
        onOutputSettingsChange={vi.fn()}
      />
    );

    expect(screen.getByRole('button', { name: /photos & images/i })).toBeDefined();
    expect(screen.getByRole('button', { name: /vidéos/i })).toBeDefined();
    expect(screen.getByText('Glissez-déposez vos photos ici')).toBeDefined();
    expect(screen.getByText(/photo_test\.jpg/i)).toBeDefined();
    expect(screen.getByText(/CPU : 12% • RAM : 2.1 Go/i)).toBeDefined();
  });

  it('doit basculer vers le sous-onglet Vidéos et afficher la dropzone vidéo', () => {
    const onMediaTabChange = vi.fn();
    const { rerender } = render(
      <DashboardView
        items={MOCK_ITEMS}
        activeMediaTab="images"
        onMediaTabChange={onMediaTabChange}
        folders={[{ label: 'Dossier 1', value: '0' }]}
        destValue="0"
        onDestChange={vi.fn()}
        onBrowseFolder={vi.fn()}
        targetKo={1104}
        onTargetChange={vi.fn()}
        onAddBatch={vi.fn()}
        onClearQueue={vi.fn()}
        onRemoveItem={vi.fn()}
        running={false}
        hasRun={false}
        cpu={12}
        ram={2.1}
        onStart={vi.fn()}
        onPause={vi.fn()}
        onDownloadZip={vi.fn()}
        crf={26}
        exportMode="copy"
        outputSettings={OUTPUT_SETTINGS}
        onOutputSettingsChange={vi.fn()}
      />
    );

    const videoTabBtn = screen.getByRole('button', { name: /vidéos/i });
    fireEvent.click(videoTabBtn);
    expect(onMediaTabChange).toHaveBeenCalledWith('videos');

    rerender(
      <DashboardView
        items={MOCK_ITEMS}
        activeMediaTab="videos"
        onMediaTabChange={onMediaTabChange}
        folders={[{ label: 'Dossier 1', value: '0' }]}
        destValue="0"
        onDestChange={vi.fn()}
        onBrowseFolder={vi.fn()}
        targetKo={1104}
        onTargetChange={vi.fn()}
        onAddBatch={vi.fn()}
        onClearQueue={vi.fn()}
        onRemoveItem={vi.fn()}
        running={false}
        hasRun={false}
        cpu={12}
        ram={2.1}
        onStart={vi.fn()}
        onPause={vi.fn()}
        onDownloadZip={vi.fn()}
        crf={26}
        exportMode="copy"
        outputSettings={OUTPUT_SETTINGS}
        onOutputSettingsChange={vi.fn()}
      />
    );

    expect(screen.getByText('Glissez-déposez vos vidéos ici')).toBeDefined();
    expect(screen.getByText(/video_test\.mp4/i)).toBeDefined();
  });

  it("doit proposer l'enregistrement direct ou l'ouverture du dossier si moins de 5 fichiers sont traités", () => {
    const onOpenExportFolder = vi.fn();
    const onSaveSingleFile = vi.fn();
    render(
      <DashboardView
        items={MOCK_ITEMS}
        activeMediaTab="images"
        onMediaTabChange={vi.fn()}
        folders={[{ label: 'Dossier 1', value: '0' }]}
        destValue="0"
        onDestChange={vi.fn()}
        onBrowseFolder={vi.fn()}
        targetKo={1104}
        onTargetChange={vi.fn()}
        onAddBatch={vi.fn()}
        onClearQueue={vi.fn()}
        onRemoveItem={vi.fn()}
        running={false}
        hasRun={true}
        cpu={8}
        ram={2.4}
        onStart={vi.fn()}
        onPause={vi.fn()}
        onDownloadZip={vi.fn()}
        onOpenExportFolder={onOpenExportFolder}
        onSaveSingleFile={onSaveSingleFile}
        crf={26}
        exportMode="copy"
        outputSettings={OUTPUT_SETTINGS}
        onOutputSettingsChange={vi.fn()}
      />
    );

    const openBtn = screen.getByRole('button', { name: /ouvrir l'emplacement/i });
    const saveBtn = screen.getByRole('button', { name: /enregistrer sous…/i });

    expect(openBtn).toBeDefined();
    expect(saveBtn).toBeDefined();

    fireEvent.click(openBtn);
    expect(onOpenExportFolder).toHaveBeenCalledTimes(1);

    fireEvent.click(saveBtn);
    expect(onSaveSingleFile).toHaveBeenCalledTimes(1);
  });

  it('doit proposer le téléchargement groupé ZIP pour un lot de 5 fichiers ou plus', () => {
    const onDownloadZip = vi.fn();
    const fiveItems: MediaItem[] = Array.from({ length: 5 }, (_, i) => ({
      id: `img_${i}`,
      name: `photo_${i}.jpg`,
      path: `/path/photo_${i}.jpg`,
      kind: 'image',
      format: 'JPG',
      sizeBytes: 10 * 1024 * 1024,
      finalSizeBytes: 1.5 * 1024 * 1024,
      progress: 100,
      status: 'done',
    }));

    render(
      <DashboardView
        items={fiveItems}
        activeMediaTab="images"
        onMediaTabChange={vi.fn()}
        folders={[{ label: 'Dossier 1', value: '0' }]}
        destValue="0"
        onDestChange={vi.fn()}
        onBrowseFolder={vi.fn()}
        targetKo={1104}
        onTargetChange={vi.fn()}
        onAddBatch={vi.fn()}
        onClearQueue={vi.fn()}
        onRemoveItem={vi.fn()}
        running={false}
        hasRun={true}
        cpu={8}
        ram={2.4}
        onStart={vi.fn()}
        onPause={vi.fn()}
        onDownloadZip={onDownloadZip}
        crf={26}
        exportMode="copy"
        outputSettings={OUTPUT_SETTINGS}
        onOutputSettingsChange={vi.fn()}
      />
    );

    const downloadZipBtn = screen.getByRole('button', { name: /télécharger le lot \(\.zip\)/i });
    fireEvent.click(downloadZipBtn);

    expect(onDownloadZip).toHaveBeenCalledTimes(1);
  });

  it('doit afficher correctement les fichiers en échec avec leur message d’erreur et le KPI d’échec', () => {
    const failedItem: MediaItem = {
      id: 'err_1',
      name: 'corrupt_image.jpg',
      path: '/path/corrupt_image.jpg',
      kind: 'image',
      format: 'JPG',
      sizeBytes: 8 * 1024 * 1024,
      progress: 100,
      status: 'error',
      errorMessage: 'Fichier corrompu ou format invalide',
    };

    render(
      <DashboardView
        items={[failedItem]}
        activeMediaTab="images"
        onMediaTabChange={vi.fn()}
        folders={[{ label: 'Dossier 1', value: '0' }]}
        destValue="0"
        onDestChange={vi.fn()}
        onBrowseFolder={vi.fn()}
        targetKo={1104}
        onTargetChange={vi.fn()}
        onAddBatch={vi.fn()}
        onClearQueue={vi.fn()}
        onRemoveItem={vi.fn()}
        running={false}
        hasRun={true}
        cpu={15}
        ram={3.2}
        onStart={vi.fn()}
        onPause={vi.fn()}
        onDownloadZip={vi.fn()}
        crf={26}
        exportMode="copy"
        outputSettings={OUTPUT_SETTINGS}
        onOutputSettingsChange={vi.fn()}
      />
    );

    expect(screen.getByText(/corrupt_image\.jpg/i)).toBeDefined();
    expect(screen.getByText(/\[Échec : Fichier corrompu ou format invalide\]/i)).toBeDefined();
    expect(screen.getByText('Échecs')).toBeDefined();
    expect(screen.getByText('1')).toBeDefined();
  });

  it('doit indiquer si une vidéo est muette ou possède une piste audio', () => {
    const videoMute: MediaItem = {
      id: 'vid_mute',
      name: 'video_sans_son.mp4',
      path: '/path/video_sans_son.mp4',
      kind: 'video',
      format: 'MP4',
      sizeBytes: 20 * 1024 * 1024,
      progress: 0,
      status: 'wait',
      hasAudio: false,
    };
    const videoWithSound: MediaItem = {
      id: 'vid_sound',
      name: 'video_avec_son.mp4',
      path: '/path/video_avec_son.mp4',
      kind: 'video',
      format: 'MP4',
      sizeBytes: 25 * 1024 * 1024,
      progress: 0,
      status: 'wait',
      hasAudio: true,
    };

    render(
      <DashboardView
        items={[videoMute, videoWithSound]}
        activeMediaTab="videos"
        onMediaTabChange={vi.fn()}
        folders={[{ label: 'Dossier 1', value: '0' }]}
        destValue="0"
        onDestChange={vi.fn()}
        onBrowseFolder={vi.fn()}
        targetKo={1104}
        onTargetChange={vi.fn()}
        onAddBatch={vi.fn()}
        onClearQueue={vi.fn()}
        onRemoveItem={vi.fn()}
        running={false}
        hasRun={false}
        cpu={10}
        ram={2.0}
        onStart={vi.fn()}
        onPause={vi.fn()}
        onDownloadZip={vi.fn()}
        crf={26}
        exportMode="copy"
        outputSettings={OUTPUT_SETTINGS}
        onOutputSettingsChange={vi.fn()}
      />
    );

    expect(screen.getByText('Source muette')).toBeDefined();
    expect(screen.getByText('Audio stéréo')).toBeDefined();
  });

  it('déclenche onStart et onBrowseFolder lors des clics sur les boutons correspondants', () => {
    const onStart = vi.fn();
    const onBrowseFolder = vi.fn();
    const onDestChange = vi.fn();

    const pendingItem: MediaItem = {
      id: 'item_1',
      name: 'photo.jpg',
      path: '/path/photo.jpg',
      kind: 'image',
      format: 'JPG',
      sizeBytes: 5 * 1024 * 1024,
      progress: 0,
      status: 'wait',
    };

    render(
      <DashboardView
        items={[pendingItem]}
        activeMediaTab="images"
        onMediaTabChange={vi.fn()}
        folders={[
          { label: 'Dossier 1', value: '/dossier-1' },
          { label: 'Dossier 2', value: '/dossier-2' },
        ]}
        destValue="/dossier-1"
        onDestChange={onDestChange}
        onBrowseFolder={onBrowseFolder}
        targetKo={1104}
        onTargetChange={vi.fn()}
        onAddBatch={vi.fn()}
        onClearQueue={vi.fn()}
        onRemoveItem={vi.fn()}
        running={false}
        hasRun={false}
        cpu={0}
        ram={0}
        onStart={onStart}
        onPause={vi.fn()}
        onDownloadZip={vi.fn()}
        crf={26}
        exportMode="copy"
        outputSettings={OUTPUT_SETTINGS}
        onOutputSettingsChange={vi.fn()}
      />
    );

    const startBtn = screen.getByRole('button', { name: /LANCER LA COMPRESSION/i });
    fireEvent.click(startBtn);
    expect(onStart).toHaveBeenCalledTimes(1);

    const browseBtn = screen.getByRole('button', { name: /Parcourir/i });
    fireEvent.click(browseBtn);
    expect(onBrowseFolder).toHaveBeenCalledTimes(1);

    const select = screen.getByRole('combobox', { name: 'Dossier de destination' });
    fireEvent.change(select, { target: { value: '/dossier-2' } });
    expect(onDestChange).toHaveBeenCalledWith('/dossier-2');
  });
  const baseProps = {
    items: MOCK_ITEMS,
    onMediaTabChange: vi.fn(),
    folders: [{ label: 'Dossier 1', value: '0' }],
    destValue: '0',
    onDestChange: vi.fn(),
    onBrowseFolder: vi.fn(),
    targetKo: 1104,
    onTargetChange: vi.fn(),
    onAddBatch: vi.fn(),
    onClearQueue: vi.fn(),
    onRemoveItem: vi.fn(),
    running: false,
    hasRun: false,
    cpu: 0,
    ram: 0,
    onStart: vi.fn(),
    onPause: vi.fn(),
    onDownloadZip: vi.fn(),
    crf: 26,
    exportMode: 'copy' as const,
    outputSettings: OUTPUT_SETTINGS,
  };

  it('permet de choisir le format de sortie des images', () => {
    const onOutputSettingsChange = vi.fn();
    render(
      <DashboardView {...baseProps} activeMediaTab="images" onOutputSettingsChange={onOutputSettingsChange} />
    );

    fireEvent.change(screen.getByLabelText('Format de sortie'), { target: { value: 'webp' } });
    expect(onOutputSettingsChange).toHaveBeenCalledWith({ image_format: 'webp' });
  });

  it('réaligne codec et conteneur vidéo incompatibles', () => {
    const onOutputSettingsChange = vi.fn();
    render(
      <DashboardView {...baseProps} activeMediaTab="videos" onOutputSettingsChange={onOutputSettingsChange} />
    );

    // WebM n'accepte que VP9
    fireEvent.change(screen.getByLabelText('Format de sortie'), { target: { value: 'webm' } });
    expect(onOutputSettingsChange).toHaveBeenLastCalledWith({ video_container: 'webm', video_codec: 'vp9' });

    fireEvent.change(screen.getByLabelText('Codec vidéo'), { target: { value: 'h265' } });
    expect(onOutputSettingsChange).toHaveBeenLastCalledWith({ video_codec: 'h265' });

    fireEvent.change(screen.getByLabelText('Taille maximale du fichier compressé'), { target: { value: '30' } });
    expect(onOutputSettingsChange).toHaveBeenLastCalledWith({ video_max_size_percent: 30 });

    fireEvent.change(screen.getByLabelText('Résolution'), { target: { value: '720' } });
    expect(onOutputSettingsChange).toHaveBeenLastCalledWith({ video_max_height: 720 });
  });

  it('désactive les codecs absents du FFmpeg embarqué', () => {
    render(
      <DashboardView
        {...baseProps}
        activeMediaTab="videos"
        onOutputSettingsChange={vi.fn()}
        supportedVideoCodecs={['h264']}
      />
    );

    const vp9 = screen.getByRole('option', { name: 'VP9 (indisponible)' }) as HTMLOptionElement;
    expect(vp9.disabled).toBe(true);
  });

  it('affiche le format produit quand il diffère de la source', () => {
    render(
      <DashboardView
        {...baseProps}
        items={[{ ...MOCK_ITEMS[0], outputPath: '/export/photo_test.webp' }]}
        activeMediaTab="images"
        onOutputSettingsChange={vi.fn()}
      />
    );

    expect(screen.getByText(/\(WEBP\)/)).toBeDefined();
  });
});
