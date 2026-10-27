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

    // Rerender avec le sous-onglet vidéos actif
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

  it("doit appeler onDownloadZip lorsqu'on clique sur le bouton de téléchargement", () => {
    const onDownloadZip = vi.fn();
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
        onDownloadZip={onDownloadZip}
        crf={26}
        exportMode="copy"
      />
    );

    const downloadBtn = screen.getByRole('button', { name: /télécharger le lot \(\.zip\)/i });
    fireEvent.click(downloadBtn);

    expect(onDownloadZip).toHaveBeenCalledTimes(1);
  });
});
