import React from 'react';
import { MediaItem } from '../../types/media';
import { ExportMode } from '../../types/ipc';
import { Button } from '../common/Button';
import { Badge } from '../common/Badge';
import { ProgressBar } from '../common/ProgressBar';
import { RangeSlider } from '../common/RangeSlider';
import {
  IconUpload,
  IconDownload,
  IconImage,
  IconVideo,
  IconPlay,
  IconCheck,
  IconClose,
} from '../common/Icons';
import { formatBytes, calculateGainPercent } from '../../utils/formatters';

export type MediaTab = 'images' | 'videos';

export interface FolderOption {
  label: string;
  value: string;
}

export interface DashboardViewProps {
  items: MediaItem[];
  activeMediaTab: MediaTab;
  onMediaTabChange: (tab: MediaTab) => void;
  folders: FolderOption[];
  destValue: string;
  onDestChange: (dest: string) => void;
  onBrowseFolder: () => void;
  targetKo: number;
  onTargetChange: (targetKo: number) => void;
  onAddBatch: (kind: MediaTab) => void;
  onClearQueue: (kind: MediaTab) => void;
  onRemoveItem: (id: string) => void;
  running: boolean;
  hasRun: boolean;
  cpu: number;
  ram: number;
  onStart: () => void;
  onPause: () => void;
  onDownloadZip: () => void;
  crf: number;
  exportMode: ExportMode;
}

const MODE_TITLES: Record<ExportMode, string> = {
  copy: "Copie dans le dossier d'export",
  suffix: 'Suffixe _opt',
  folder: 'Sous-dossier horodaté',
};

export const DashboardView: React.FC<DashboardViewProps> = ({
  items,
  activeMediaTab,
  onMediaTabChange,
  folders,
  destValue,
  onDestChange,
  onBrowseFolder,
  targetKo,
  onTargetChange,
  onAddBatch,
  onClearQueue,
  onRemoveItem,
  running,
  hasRun,
  cpu,
  ram,
  onStart,
  onPause,
  onDownloadZip,
  crf,
  exportMode,
}) => {
  const images = items.filter((f) => f.kind === 'image');
  const videos = items.filter((f) => f.kind === 'video');

  const currentItems = activeMediaTab === 'images' ? images : videos;
  const currentQuotaMax = activeMediaTab === 'images' ? 50 : 20;
  const currentQuotaPct = Math.min(100, Math.round((currentItems.length / currentQuotaMax) * 100));

  const pendingCount = items.filter((f) => f.status !== 'done').length;
  const doneItems = items.filter((f) => f.status === 'done');

  // Calculs KPI pour le rapport de compression direct
  const sumBefore = doneItems.reduce((acc, f) => acc + f.sizeBytes, 0);
  const sumAfter = doneItems.reduce((acc, f) => acc + (f.finalSizeBytes ?? f.sizeBytes), 0);
  const totalSavedBytes = Math.max(0, sumBefore - sumAfter);
  const averageGain =
    doneItems.length > 0
      ? Math.round(
          doneItems.reduce(
            (acc, f) => acc + calculateGainPercent(f.sizeBytes, f.finalSizeBytes ?? f.sizeBytes),
            0
          ) / doneItems.length
        )
      : 0;

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    onAddBatch(activeMediaTab);
  };

  const targetLabel = formatBytes(targetKo * 1024) + ` (${targetKo} Ko)`;

  const startLabel = running
    ? 'COMPRESSION EN COURS…'
    : pendingCount > 0
    ? 'LANCER LA COMPRESSION'
    : 'COMPRESSION TERMINÉE';

  const startDisabled = running || pendingCount === 0;

  return (
    <div role="tabpanel" id="panel-dash" aria-labelledby="tab-dash" style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      {/* Sélecteur de sous-onglets : Images vs Vidéos */}
      <div
        role="group"
        aria-label="Sélection du type de média à traiter"
        style={{
          display: 'flex',
          gap: '12px',
          background: 'var(--surface-100)',
          padding: '8px',
          borderRadius: '16px',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <button
          type="button"
          className="gc-chip"
          aria-pressed={activeMediaTab === 'images'}
          onClick={() => onMediaTabChange('images')}
          style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}
        >
          <IconImage />
          <span>Photos & Images ({images.length}/50)</span>
        </button>

        <button
          type="button"
          className="gc-chip"
          aria-pressed={activeMediaTab === 'videos'}
          onClick={() => onMediaTabChange('videos')}
          style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}
        >
          <IconVideo />
          <span>Vidéos ({videos.length}/20)</span>
        </button>
      </div>

      {/* Zone Drag & Drop Spécifique */}
      {activeMediaTab === 'images' ? (
        <button
          type="button"
          className="gc-drop"
          onClick={() => onAddBatch('images')}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
          aria-label="Zone de glisser-déposer photos : cliquez pour ajouter des images"
        >
          <IconUpload size={56} style={{ stroke: '#0a243d' }} />
          <span className="gc-drop-title">Glissez-déposez vos photos ici</span>
          <span className="gc-drop-hint">Formats JPG, PNG, WebP • Jusqu'à 50 photos (~15 Mo) par lot</span>
        </button>
      ) : (
        <button
          type="button"
          className="gc-drop"
          onClick={() => onAddBatch('videos')}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
          aria-label="Zone de glisser-déposer vidéos : cliquez pour ajouter des vidéos"
          style={{ background: 'linear-gradient(135deg, #1b6ca8, #0a243d)', color: '#ffffff' }}
        >
          <IconUpload size={56} style={{ stroke: '#ffffff' }} />
          <span className="gc-drop-title" style={{ color: '#ffffff' }}>Glissez-déposez vos vidéos ici</span>
          <span className="gc-drop-hint" style={{ color: '#cbd9e1' }}>
            Formats MP4, MOV, MKV, AVI • Limite stricte : 20 vidéos maximum
          </span>
        </button>
      )}

      {/* Carte Réglages contextuels (Cible pour images, Garde-fou pour vidéos) */}
      {activeMediaTab === 'images' ? (
        <div className="gc-card">
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', alignItems: 'center', justifyContent: 'space-between' }}>
            <div>
              <h2 className="gc-h2" style={{ margin: 0 }}>Calibration Cible : 800 Ko – 2 Mo</h2>
              <p className="gc-muted" style={{ margin: '4px 0 0', fontSize: '14px' }}>
                Recherche dichotomique du facteur Q pour une réduction de 85 % à 95 % sans perte visible.
              </p>
            </div>
            <Badge>Rayon Multithread</Badge>
          </div>

          <RangeSlider
            id="target-slider-dash"
            min={800}
            max={2048}
            step={16}
            value={targetKo}
            onChange={onTargetChange}
            helpText={`Poids cible par image : ${targetLabel}`}
          />
        </div>
      ) : (
        <div className="gc-card" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '12px' }}>
          <div>
            <h2 className="gc-h2" style={{ margin: 0 }}>Garde-Fou & Transcodage Séquentiel</h2>
            <p className="gc-muted" style={{ margin: '4px 0 0', fontSize: '14px' }}>
              Traitement asynchrone FFmpeg (CRF {crf}) une vidéo à la fois pour protéger la RAM et le processeur.
            </p>
          </div>
          <Badge>Max 20 vidéos</Badge>
        </div>
      )}

      {/* Carte File d'attente du média actif */}
      <div className="gc-card" style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '12px', alignItems: 'center', justifyContent: 'space-between' }}>
          <div>
            <h2 className="gc-h2">
              File d'attente : {activeMediaTab === 'images' ? 'Photos & Images' : 'Vidéos'}
            </h2>
            <div className="gc-caption" style={{ marginTop: '2px' }}>
              Quota du lot : {currentItems.length} / {currentQuotaMax} fichiers
            </div>
          </div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '12px', alignItems: 'center' }}>
            <Button
              variant="secondary"
              size="small"
              onClick={() => onClearQueue(activeMediaTab)}
              disabled={currentItems.length === 0}
            >
              Vider la file {activeMediaTab === 'images' ? 'images' : 'vidéos'}
            </Button>
          </div>
        </div>

        {/* Barre de quota */}
        <ProgressBar
          progress={currentQuotaPct}
          label={`Quota ${activeMediaTab}`}
          style={{ height: '8px' }}
        />

        {currentItems.length === 0 ? (
          <p className="gc-muted" style={{ margin: '8px 0', fontSize: '15px', lineHeight: '22px' }}>
            Aucun fichier dans cette file. Glissez-déposez ou cliquez sur la zone ci-dessus pour importer vos{' '}
            {activeMediaTab === 'images' ? 'photos' : 'vidéos'}.
          </p>
        ) : (
          <div className="gc-queue">
            {currentItems.map((item) => {
              const isDone = item.status === 'done';
              const isRunning = item.status === 'running';
              const initialStr = formatBytes(item.sizeBytes);
              const finalStr = item.finalSizeBytes ? formatBytes(item.finalSizeBytes) : '';
              const gainPercent =
                isDone && item.finalSizeBytes
                  ? calculateGainPercent(item.sizeBytes, item.finalSizeBytes)
                  : 0;

              return (
                <div key={item.id}>
                  <div className="gc-item-row">
                    {isRunning ? (
                      <IconPlay />
                    ) : item.kind === 'video' ? (
                      <IconVideo />
                    ) : (
                      <IconImage />
                    )}

                    <span style={{ minWidth: 0, overflowWrap: 'anywhere' }}>
                      <span style={{ fontWeight: 500 }}>{item.name}</span>
                      <span className="gc-muted"> — {initialStr}</span>
                      {isDone && finalStr ? (
                        <span style={{ color: 'var(--brand-text)' }}> → {finalStr}</span>
                      ) : null}
                      {isDone ? (
                        <span className="gc-status gc-status--done">
                          {` (-${gainPercent}%) [Terminé]`}
                        </span>
                      ) : isRunning ? (
                        <span className="gc-status">
                          {` (Traitement en cours... ${Math.round(item.progress)}%)`}
                        </span>
                      ) : (
                        <span className="gc-status gc-status--wait">
                          {' (En attente)'}
                        </span>
                      )}
                    </span>

                    {isDone && (
                      <span className="gc-check" style={{ marginLeft: 'auto' }}>
                        <IconCheck />
                      </span>
                    )}

                    <button
                      type="button"
                      className="gc-iconbtn"
                      aria-label={`Retirer ${item.name}`}
                      onClick={() => onRemoveItem(item.id)}
                      style={{ marginLeft: isDone ? undefined : 'auto' }}
                    >
                      <IconClose />
                    </button>
                  </div>

                  <ProgressBar
                    progress={item.progress}
                    isDone={isDone}
                    label={`Progression de ${item.name}`}
                  />
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Destination & Téléchargement / Export */}
      <div className="gc-card" style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '8px' }}>
          <h2 className="gc-h2" style={{ margin: 0 }}>Dossier de destination & Export</h2>
          <Badge>Écriture : {MODE_TITLES[exportMode]}</Badge>
        </div>

        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '12px', alignItems: 'stretch' }}>
          <div style={{ flex: '1 1 320px', minWidth: 0 }}>
            <select
              id="dest-dash-select"
              className="gc-field"
              value={destValue}
              onChange={(e) => onDestChange(e.target.value)}
              style={{ minHeight: '44px' }}
            >
              {folders.map((f) => (
                <option key={f.value} value={f.value}>
                  {f.label}
                </option>
              ))}
            </select>
          </div>
          <Button variant="secondary" size="small" onClick={onBrowseFolder}>
            Parcourir…
          </Button>
        </div>

        {/* Rapport KPI lorsque la compression a produit des résultats */}
        {doneItems.length > 0 && (
          <div
            style={{
              display: 'flex',
              flexWrap: 'wrap',
              gap: '12px',
              background: 'var(--tint-50)',
              padding: '16px',
              borderRadius: '12px',
              alignItems: 'center',
              justifyContent: 'space-between',
            }}
          >
            <div style={{ display: 'flex', gap: '20px', flexWrap: 'wrap' }}>
              <div>
                <div className="gc-caption">Fichiers traités</div>
                <div style={{ fontWeight: 700, fontSize: '18px' }}>{doneItems.length}</div>
              </div>
              <div>
                <div className="gc-caption">Gain moyen</div>
                <div style={{ fontWeight: 700, fontSize: '18px', color: 'var(--success-700)' }}>
                  -{averageGain}%
                </div>
              </div>
              <div>
                <div className="gc-caption">Poids économisé</div>
                <div style={{ fontWeight: 700, fontSize: '18px', color: 'var(--brand-text)' }}>
                  {formatBytes(totalSavedBytes)}
                </div>
              </div>
            </div>

            {/* Bouton de Téléchargement du lot */}
            <Button
              onClick={onDownloadZip}
              style={{
                display: 'inline-flex',
                gap: '8px',
                alignItems: 'center',
                padding: '12px 20px',
              }}
            >
              <IconDownload />
              <span>Télécharger le lot (.ZIP)</span>
            </Button>
          </div>
        )}
      </div>

      {/* Barre d'action inférieure */}
      <div
        style={{
          display: 'flex',
          flexWrap: 'wrap',
          gap: '16px',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '4px 0',
        }}
      >
        <span className="gc-caption" style={{ fontSize: '14px', lineHeight: '20px' }}>
          {(hasRun || running ? '' : 'ex. ') + `CPU : ${cpu}% • RAM : ${ram.toFixed(1)} Go`}
        </span>

        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '12px', alignItems: 'center' }}>
          {running && (
            <Button variant="secondary" onClick={onPause}>
              Mettre en pause
            </Button>
          )}
          <Button disabled={startDisabled} onClick={onStart} style={{ minWidth: '280px' }}>
            {startLabel}
          </Button>
        </div>
      </div>
    </div>
  );
};
