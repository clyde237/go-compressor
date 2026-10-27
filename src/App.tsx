import { useState } from 'react';
import { BatchConfig } from './types/ipc';
import { loadSavedSettings, saveSettings, resetSavedSettings } from './utils/storage';
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

export function App() {
  const [tab, setTab] = useState<TabId>('dash');
  const [activeMediaTab, setActiveMediaTab] = useState<MediaTab>('images');
  const [settings, setSettings] = useState<BatchConfig>(() => loadSavedSettings());
  const [targetKo, setTargetKo] = useState<number>(() =>
    Math.round(loadSavedSettings().target_bytes / 1024)
  );

  const {
    files,
    running,
    hasRun,
    cpu,
    ram,
    exportDirectory,
    alert,
    clearAlert,
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
  } = useMediaBatch();

  // Synchronisation du slider cible entre Dashboard et Settings avec persistance
  const handleTargetChange = (newKo: number) => {
    setTargetKo(newKo);
    setSettings((prev) => {
      const updated = {
        ...prev,
        target_bytes: newKo * 1024,
      };
      saveSettings(updated);
      return updated;
    });
  };

  const handleUpdateSettings = (partial: Partial<BatchConfig>) => {
    setSettings((prev) => {
      const updated = { ...prev, ...partial };
      if (partial.target_bytes) {
        setTargetKo(Math.round(partial.target_bytes / 1024));
      }
      saveSettings(updated);
      return updated;
    });
  };

  const handleResetSettings = () => {
    const defaults = resetSavedSettings();
    setSettings(defaults);
    setTargetKo(Math.round(defaults.target_bytes / 1024));
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
            onOpenExportFolder={openExportFolder}
            onSaveSingleFile={saveSingleFile}
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
              const partial: Partial<BatchConfig> = {};
              if (updated.targetMinBytes !== undefined) partial.target_min_bytes = updated.targetMinBytes;
              if (updated.targetMaxBytes !== undefined) partial.target_max_bytes = updated.targetMaxBytes;
              if (updated.targetBytes !== undefined) partial.target_bytes = updated.targetBytes;
              if (updated.qMin !== undefined) partial.q_min = updated.qMin;
              if (updated.qMax !== undefined) partial.q_max = updated.qMax;
              if (updated.threads !== undefined) partial.threads = updated.threads;
              if (updated.crf !== undefined) partial.crf = updated.crf;
              if (updated.mode !== undefined) partial.export_mode = updated.mode;
              if (updated.preserveExif !== undefined) partial.preserve_exif = updated.preserveExif;
              if (updated.outputDirectory !== undefined) partial.output_directory = updated.outputDirectory;
              handleUpdateSettings(partial);
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
