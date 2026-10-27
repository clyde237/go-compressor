import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { DashboardView } from './DashboardView';
import { MediaItem } from '../../types/media';

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
      />
    );

    expect(screen.getByText(/corrupt_image\.jpg/i)).toBeDefined();
    expect(screen.getByText(/\[Échec : Fichier corrompu ou format invalide\]/i)).toBeDefined();
    expect(screen.getByText('Échecs')).toBeDefined();
    expect(screen.getByText('1')).toBeDefined();
  });
});
