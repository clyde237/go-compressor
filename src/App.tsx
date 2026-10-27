import { useState, useEffect } from 'react';
import { BatchConfig, ExportMode } from './types/ipc';
import { HeaderTabs, TabId } from './components/layout/HeaderTabs';
import { DashboardView, FolderOption, MediaTab } from './components/views/DashboardView';
import { SettingsView } from './components/views/SettingsView';
import { ModalAlert } from './components/common/ModalAlert';
import { useMediaBatch } from './hooks/useMediaBatch';

const INITIAL_FOLDERS: FolderOption[] = [
  { label: 'ex: /Utilisateurs/Media/Export', value: '0' },
  { label: 'ex: /Utilisateurs/Documents/Compressés', value: '1' },
  { label: 'ex: /Utilisateurs/Bureau/Optimisés', value: '2' },
];

const DEFAULT_SETTINGS: BatchConfig = {
  target_min_bytes: 800 * 1024,
  target_max_bytes: 2 * 1024 * 1024,
  target_bytes: 1104 * 1024, // 1104 Ko
  q_min: 65,
  q_max: 92,
  threads: 4,
  crf: 26,
  export_mode: 'copy' as ExportMode,
  preserve_exif: true,
  output_directory: '0',
};

export function App() {
  const [tab, setTab] = useState<TabId>('dash');
  const [activeMediaTab, setActiveMediaTab] = useState<MediaTab>('images');
  const [targetKo, setTargetKo] = useState<number>(1104);
  const [settings, setSettings] = useState<BatchConfig>(DEFAULT_SETTINGS);

  const {
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
    removeItem,
    clearQueue,
    startCompression,
    pauseCompression,
    setExportDirectory,
  } = useMediaBatch();

  // Chargement d'exemples initiaux au premier démarrage si la file est vide
  useEffect(() => {
    addFilesFromPaths(
      ['/exemples/photo_01.jpg', '/exemples/photo_02.png'],
      'images'
    );
    addFilesFromPaths(
      ['/exemples/clip_01.mp4'],
      'videos'
    );
  }, [addFilesFromPaths]);

  // Synchronisation du slider cible entre Dashboard et Settings
  const handleTargetChange = (newKo: number) => {
    setTargetKo(newKo);
    setSettings((prev) => ({
      ...prev,
      target_bytes: newKo * 1024,
    }));
  };

  const handleUpdateSettings = (partial: Partial<BatchConfig>) => {
    setSettings((prev) => {
      const updated = { ...prev, ...partial };
      if (partial.target_bytes) {
        setTargetKo(Math.round(partial.target_bytes / 1024));
      }
      return updated;
    });
  };

  const handleResetSettings = () => {
    setSettings(DEFAULT_SETTINGS);
    setTargetKo(1104);
  };

  // Adapter les options de dossier avec le dossier sélectionné s'il existe
  const displayFolders: FolderOption[] = exportDirectory
    ? [{ label: exportDirectory, value: exportDirectory }, ...INITIAL_FOLDERS]
    : INITIAL_FOLDERS;

  const currentDestValue = exportDirectory || settings.output_directory || '0';

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
            folders={displayFolders}
            destValue={currentDestValue}
            onDestChange={(dir) => {
              setExportDirectory(dir);
              handleUpdateSettings({ output_directory: dir });
            }}
            onBrowseFolder={selectNativeExportDirectory}
            targetKo={targetKo}
            onTargetChange={handleTargetChange}
            onAddBatch={openNativeFileDialog}
            onClearQueue={clearQueue}
            onRemoveItem={removeItem}
            running={running}
            hasRun={hasRun}
            cpu={cpu}
            ram={ram}
            onStart={() => startCompression(settings)}
            onPause={pauseCompression}
            onDownloadZip={downloadZipArchive}
            crf={settings.crf}
            exportMode={settings.export_mode}
          />
        )}

        {tab === 'settings' && (
          <SettingsView
            settings={{
              targetMinBytes: settings.target_min_bytes,
              targetMaxBytes: settings.target_max_bytes,
              targetBytes: settings.target_bytes,
              qMin: settings.q_min,
              qMax: settings.q_max,
              threads: settings.threads,
              crf: settings.crf,
              mode: settings.export_mode,
              preserveExif: settings.preserve_exif,
              outputDirectory: settings.output_directory,
            }}
            onUpdateSettings={(updated) => {
              handleUpdateSettings({
                target_min_bytes: updated.targetMinBytes,
                target_max_bytes: updated.targetMaxBytes,
                target_bytes: updated.targetBytes,
                q_min: updated.qMin,
                q_max: updated.qMax,
                threads: updated.threads,
                crf: updated.crf,
                export_mode: updated.mode,
                preserve_exif: updated.preserveExif,
                output_directory: updated.outputDirectory,
              });
            }}
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
        onClose={clearAlert}
      />
    </div>
  );
}

export default App;
