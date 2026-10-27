import { useState, useEffect } from 'react';
import { MediaItem } from './types/media';
import { BatchSettings, ExportMode } from './types/ipc';
import { HeaderTabs, TabId } from './components/layout/HeaderTabs';
import { DashboardView, FolderOption, MediaTab } from './components/views/DashboardView';
import { SettingsView } from './components/views/SettingsView';
import { ModalAlert } from './components/common/ModalAlert';

const INITIAL_FOLDERS: FolderOption[] = [
  { label: 'ex: /Utilisateurs/Media/Export', value: '0' },
  { label: 'ex: /Utilisateurs/Documents/Compressés', value: '1' },
  { label: 'ex: /Utilisateurs/Bureau/Optimisés', value: '2' },
];

const INITIAL_FILES: MediaItem[] = [
  {
    id: '1',
    name: 'photo_01.jpg',
    path: '/mock/photo_01.jpg',
    kind: 'image',
    format: 'JPG',
    sizeBytes: 15.2 * 1024 * 1024,
    finalSizeBytes: 1.1 * 1024 * 1024,
    progress: 100,
    status: 'done',
  },
  {
    id: '2',
    name: 'video_02.mp4',
    path: '/mock/video_02.mp4',
    kind: 'video',
    format: 'MP4',
    sizeBytes: 45.1 * 1024 * 1024,
    progress: 68,
    status: 'running',
  },
  {
    id: '3',
    name: 'photo_03.png',
    path: '/mock/photo_03.png',
    kind: 'image',
    format: 'PNG',
    sizeBytes: 14.8 * 1024 * 1024,
    progress: 0,
    status: 'wait',
  },
];

const DEFAULT_SETTINGS: BatchSettings = {
  targetMinBytes: 800 * 1024,
  targetMaxBytes: 2 * 1024 * 1024,
  targetBytes: 1104 * 1024, // 1104 Ko
  qMin: 65,
  qMax: 92,
  threads: 4,
  crf: 26,
  mode: 'copy' as ExportMode,
  preserveExif: true,
  outputDirectory: '0',
};

export function App() {
  const [tab, setTab] = useState<TabId>('dash');
  const [activeMediaTab, setActiveMediaTab] = useState<MediaTab>('images');
  const [destValue, setDestValue] = useState<string>('0');
  const [targetKo, setTargetKo] = useState<number>(1104);
  const [settings, setSettings] = useState<BatchSettings>(DEFAULT_SETTINGS);
  const [files, setFiles] = useState<MediaItem[]>(INITIAL_FILES);
  const [running, setRunning] = useState<boolean>(false);
  const [hasRun, setHasRun] = useState<boolean>(false);
  const [cpu, setCpu] = useState<number>(45);
  const [ram, setRam] = useState<number>(2.4);
  const [nextId, setNextId] = useState<number>(4);
  const [alert, setAlert] = useState<{ title: string; body: string; detail?: string } | null>(null);

  // Synchronisation du slider cible entre Dashboard et Settings
  const handleTargetChange = (newKo: number) => {
    setTargetKo(newKo);
    setSettings((prev) => ({
      ...prev,
      targetBytes: newKo * 1024,
    }));
  };

  const handleUpdateSettings = (partial: Partial<BatchSettings>) => {
    setSettings((prev) => {
      const updated = { ...prev, ...partial };
      if (partial.targetBytes) {
        setTargetKo(Math.round(partial.targetBytes / 1024));
      }
      return updated;
    });
  };

  const handleResetSettings = () => {
    setSettings(DEFAULT_SETTINGS);
    setTargetKo(1104);
  };

  // Ajout par lot selon le sous-onglet actif (Photos ou Vidéos)
  const handleAddBatch = (kind: MediaTab) => {
    const currentImgs = files.filter((f) => f.kind === 'image').length;
    const currentVids = files.filter((f) => f.kind === 'video').length;

    if (kind === 'videos') {
      if (currentVids + 1 > 20) {
        setAlert({
          title: 'Quota vidéo dépassé',
          body: 'Limite dépassée : 20 vidéos maximum par lot.',
          detail:
            "Pour préserver la mémoire vive (RAM) et éviter toute surchauffe du processeur lors du transcodage séquentiel, retirez des vidéos de la file avant d'en ajouter.",
        });
        return;
      }

      const n = nextId;
      const newVideo: MediaItem = {
        id: String(n),
        name: `clip_${n}.mp4`,
        path: `/mock/clip_${n}.mp4`,
        kind: 'video',
        format: 'MP4',
        sizeBytes: Math.round((30 + Math.random() * 120) * 10) / 10 * 1024 * 1024,
        progress: 0,
        status: 'wait',
      };
      setFiles((prev) => [...prev, newVideo]);
      setNextId((prev) => prev + 1);
    } else {
      if (currentImgs + 2 > 50) {
        setAlert({
          title: 'Quota photos dépassé',
          body: 'Limite dépassée : 50 images maximum par lot.',
          detail: "Retirez des photos de la file avant d'en importer de nouvelles.",
        });
        return;
      }

      const exts = ['jpg', 'png', 'webp'];
      const n = nextId;
      const newImages: MediaItem[] = [
        {
          id: String(n),
          name: `IMG_${2040 + n}.${exts[n % 3]}`,
          path: `/mock/IMG_${2040 + n}.${exts[n % 3]}`,
          kind: 'image',
          format: exts[n % 3].toUpperCase(),
          sizeBytes: Math.round((12 + Math.random() * 6) * 10) / 10 * 1024 * 1024,
          progress: 0,
          status: 'wait',
        },
        {
          id: String(n + 1),
          name: `IMG_${2041 + n}.${exts[(n + 1) % 3]}`,
          path: `/mock/IMG_${2041 + n}.${exts[(n + 1) % 3]}`,
          kind: 'image',
          format: exts[(n + 1) % 3].toUpperCase(),
          sizeBytes: Math.round((12 + Math.random() * 6) * 10) / 10 * 1024 * 1024,
          progress: 0,
          status: 'wait',
        },
      ];
      setFiles((prev) => [...prev, ...newImages]);
      setNextId((prev) => prev + 2);
    }
  };

  const handleRemoveItem = (id: string) => {
    setFiles((prev) => prev.filter((item) => item.id !== id));
  };

  const handleClearQueue = (kind: MediaTab) => {
    const targetKind = kind === 'images' ? 'image' : 'video';
    setFiles((prev) => prev.filter((item) => item.kind !== targetKind));
  };

  const handleBrowseFolder = () => {
    setDestValue((prev) => String((Number(prev) + 1) % INITIAL_FOLDERS.length));
  };

  const handleDownloadZip = () => {
    setAlert({
      title: 'Téléchargement de l’archive',
      body: 'Archive ZIP générée avec succès.',
      detail:
        'Tous les fichiers multimédias optimisés du lot ont été empaquetés et sont prêts dans votre répertoire de destination.',
    });
  };

  // Boucle de simulation d'avancement
  useEffect(() => {
    if (!running) return;

    const timer = setInterval(() => {
      setFiles((prevFiles) => {
        const nextFiles = prevFiles.map((f) => ({ ...f }));
        let cur = nextFiles.find((f) => f.status === 'running');

        if (!cur) {
          cur = nextFiles.find((f) => f.status === 'wait');
          if (!cur) {
            setRunning(false);
            setCpu(8);
            setRam(2.4);
            return nextFiles;
          }
          cur.status = 'running';
          cur.progress = 0;
        }

        const step = cur.kind === 'video' ? 1.8 + Math.random() * 2.2 : 5 + Math.random() * 7;
        cur.progress = Math.min(100, cur.progress + step);

        if (cur.progress >= 100) {
          cur.status = 'done';
          cur.progress = 100;
          if (cur.kind === 'image') {
            const ko = Math.max(800, Math.min(2048, targetKo * (0.92 + Math.random() * 0.16)));
            cur.finalSizeBytes = Math.min(ko * 1024, cur.sizeBytes * 0.15);
          } else {
            const factor = 0.5 - (settings.crf - 23) * 0.05;
            cur.finalSizeBytes = cur.sizeBytes * factor * (0.9 + Math.random() * 0.2);
          }
        }

        const stillPending = nextFiles.some((f) => f.status !== 'done');
        const isVid = cur.kind === 'video';

        setCpu(stillPending ? Math.round(isVid ? 70 + Math.random() * 25 : 55 + Math.random() * 30) : 8);
        setRam(
          stillPending
            ? Math.round((isVid ? 3.2 + Math.random() * 1.4 : 2.6 + Math.random() * 0.8) * 10) / 10
            : 2.4
        );

        if (!stillPending) {
          setRunning(false);
        }

        return nextFiles;
      });
    }, 110);

    return () => clearInterval(timer);
  }, [running, targetKo, settings.crf]);

  return (
    <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column', background: '#F8FAFC', color: 'var(--ink)' }}>
      {/* Barre d'onglets principale : Tableau de bord & Paramètres */}
      <HeaderTabs activeTab={tab} onTabChange={setTab} />

      {/* Conteneur principal */}
      <main
        style={{
          flex: '1 1 auto',
          width: '100%',
          maxWidth: '1040px',
          margin: '0 auto',
          boxSizing: 'border-box',
          padding: '24px',
          display: 'flex',
          flexDirection: 'column',
          gap: '16px',
        }}
      >
        {tab === 'dash' && (
          <DashboardView
            items={files}
            activeMediaTab={activeMediaTab}
            onMediaTabChange={setActiveMediaTab}
            folders={INITIAL_FOLDERS}
            destValue={destValue}
            onDestChange={setDestValue}
            onBrowseFolder={handleBrowseFolder}
            targetKo={targetKo}
            onTargetChange={handleTargetChange}
            onAddBatch={handleAddBatch}
            onClearQueue={handleClearQueue}
            onRemoveItem={handleRemoveItem}
            running={running}
            hasRun={hasRun}
            cpu={cpu}
            ram={ram}
            onStart={() => {
              setRunning(true);
              setHasRun(true);
            }}
            onPause={() => setRunning(false)}
            onDownloadZip={handleDownloadZip}
            crf={settings.crf}
            exportMode={settings.mode}
          />
        )}

        {tab === 'settings' && (
          <SettingsView
            settings={settings}
            onUpdateSettings={handleUpdateSettings}
            onResetDefaults={handleResetSettings}
          />
        )}
      </main>

      {/* Alerte modale bloquante de quota */}
      <ModalAlert
        isOpen={alert !== null}
        title={alert?.title ?? ''}
        body={alert?.body ?? ''}
        detail={alert?.detail}
        onClose={() => setAlert(null)}
      />
    </div>
  );
}

export default App;
