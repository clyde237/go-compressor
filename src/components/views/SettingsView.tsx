import React from 'react';
import { ExportMode, BatchSettings } from '../../types/ipc';
import { RangeSlider } from '../common/RangeSlider';
import { Button } from '../common/Button';
import { Badge } from '../common/Badge';
import { formatBytes } from '../../utils/formatters';

export interface SettingsViewProps {
  settings: BatchSettings;
  onUpdateSettings: (newSettings: Partial<BatchSettings>) => void;
  onResetDefaults: () => void;
}

interface BisectStep {
  n: number;
  q: number;
  sizeStr: string;
  verdict: string;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  settings,
  onUpdateSettings,
  onResetDefaults,
}) => {
  // Calcul de la simulation de recherche dichotomique pour une image de ~15 Mo
  const targetMo = settings.targetBytes / (1024 * 1024);
  const sizeAt = (q: number) => 0.8 * Math.exp(0.0365 * (q - 65));

  let lo = settings.qMin;
  let hi = settings.qMax;
  let i = 0;
  const bisectSteps: BisectStep[] = [];

  while (hi - lo > 1 && i < 5) {
    const mid = Math.round((lo + hi) / 2);
    const sz = sizeAt(mid);
    const isUnder = sz <= targetMo;
    bisectSteps.push({
      n: i + 1,
      q: mid,
      sizeStr: formatBytes(sz * 1024 * 1024),
      verdict: isUnder ? 'sous la cible : on monte Q' : 'au-dessus de la cible : on baisse Q',
    });
    if (isUnder) {
      lo = mid;
    } else {
      hi = mid;
    }
    i++;
  }

  if (bisectSteps.length === 0) {
    bisectSteps.push({
      n: 1,
      q: lo,
      sizeStr: formatBytes(sizeAt(lo) * 1024 * 1024),
      verdict: 'bornes identiques',
    });
  }

  const handleQMinChange = (v: number) => {
    onUpdateSettings({ qMin: Math.min(v, settings.qMax) });
  };

  const handleQMaxChange = (v: number) => {
    onUpdateSettings({ qMax: Math.max(v, settings.qMin) });
  };

  return (
    <div role="tabpanel" id="panel-settings" aria-labelledby="tab-settings" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
      {/* En-tête */}
      <div>
        <h1 className="gc-h1">Paramètres du moteur</h1>
        <p className="gc-muted" style={{ margin: '4px 0 0', fontSize: '15px', lineHeight: '22px' }}>
          Réglages du backend Rust : images (crate rayon) et vidéos (sidecar FFmpeg).
        </p>
      </div>

      {/* Moteur d'images */}
      <div className="gc-card" style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
        <div>
          <h2 className="gc-h2">Moteur d'images — recherche dichotomique</h2>
          <p className="gc-muted" style={{ margin: '4px 0 0', fontSize: '15px', lineHeight: '22px' }}>
            Le facteur de qualité Q est ajusté par dichotomie entre Qmin et Qmax jusqu'à obtenir un fichier dans la cible.
          </p>
        </div>

        <RangeSlider
          id="s-qmin"
          label="Qualité minimale (Qmin)"
          displayValue={`Q = ${settings.qMin}`}
          min={65}
          max={92}
          value={settings.qMin}
          minLabel="65 (plus léger)"
          maxLabel="92 (plus fidèle)"
          helpText="Borne basse de la recherche : en dessous, la dégradation devient visible."
          onChange={handleQMinChange}
        />

        <RangeSlider
          id="s-qmax"
          label="Qualité maximale (Qmax)"
          displayValue={`Q = ${settings.qMax}`}
          min={65}
          max={92}
          value={settings.qMax}
          minLabel="65 (plus léger)"
          maxLabel="92 (plus fidèle)"
          helpText="Borne haute de la recherche : un Q plus élevé donne un fichier plus lourd."
          onChange={handleQMaxChange}
        />

        <RangeSlider
          id="s-threads"
          label="Threads parallèles (rayon)"
          displayValue={`${settings.threads} thread${settings.threads > 1 ? 's' : ''}`}
          min={1}
          max={8}
          value={settings.threads}
          minLabel="1"
          maxLabel="8"
          helpText="Chaque image est traitée sur un thread du pool rayon : plus de threads, plus de CPU."
          onChange={(v) => onUpdateSettings({ threads: v })}
        />

        {/* Banc interactif de simulation */}
        <div style={{ background: '#F8FAFC', border: '1px solid var(--border-subtle)', borderRadius: '12px', padding: '16px' }}>
          <div style={{ fontWeight: 600, fontSize: '15px', lineHeight: '22px' }}>
            Simulation pour une image de ~15 Mo, cible {formatBytes(settings.targetBytes)}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', marginTop: '10px' }}>
            {bisectSteps.map((b) => (
              <div key={b.n} style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', fontSize: '14px', lineHeight: '20px' }}>
                <span style={{ fontWeight: 600, minWidth: '72px' }}>Étape {b.n}</span>
                <span>Q = {b.q} → {b.sizeStr}</span>
                <span className="gc-muted">({b.verdict})</span>
              </div>
            ))}
          </div>
          <div style={{ marginTop: '12px', fontWeight: 600, fontSize: '15px', lineHeight: '22px', color: 'var(--success-700)' }}>
            Q retenu : {lo}
          </div>
        </div>
      </div>

      {/* Moteur Vidéo */}
      <div className="gc-card" style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', alignItems: 'flex-start', justifyContent: 'space-between' }}>
          <div style={{ flex: '1 1 280px', minWidth: 0 }}>
            <h2 className="gc-h2">Moteur vidéo — FFmpeg</h2>
            <p className="gc-muted" style={{ margin: '4px 0 0', fontSize: '15px', lineHeight: '22px' }}>
              Encodage H.264, H.265 ou VP9 asynchrone, une vidéo à la fois, pour éviter toute saturation mémoire ou thermique.
              Le format, le codec et la taille maximale se règlent dans l'onglet Vidéos du tableau de bord.
            </p>
          </div>
          <Badge>Max 20 vidéos par lot</Badge>
        </div>

        <RangeSlider
          id="s-crf"
          label="Facteur d'encodage CRF"
          displayValue={`CRF ${settings.crf}`}
          min={18}
          max={32}
          value={settings.crf}
          minLabel="18 (qualité max)"
          maxLabel="32 (compression max)"
          helpText="Qualité plancher (échelle H.264, convertie pour H.265 et VP9). Plus le CRF est bas, plus la qualité est haute. Le plafond de taille du tableau de bord reste prioritaire."
          onChange={(v) => onUpdateSettings({ crf: v })}
        />
      </div>

      {/* Écriture non-destructive */}
      <div className="gc-card" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
        <div>
          <h2 className="gc-h2">Écriture non-destructive</h2>
          <p className="gc-muted" style={{ margin: '4px 0 0', fontSize: '15px', lineHeight: '22px' }}>
            Où écrire les fichiers compressés. Les originaux restent toujours intacts.
          </p>
        </div>

        <div role="radiogroup" aria-label="Comportement d'écriture" style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {[
            {
              id: 'copy' as ExportMode,
              title: "Copier dans le dossier d'export",
              desc: 'Les fichiers compressés sont écrits dans le dossier de destination (recommandé).',
            },
            {
              id: 'suffix' as ExportMode,
              title: 'Ajouter le suffixe _opt',
              desc: 'Les fichiers compressés sont écrits à côté des originaux, avec le suffixe _opt.',
            },
            {
              id: 'folder' as ExportMode,
              title: 'Créer un sous-dossier horodaté',
              desc: "Chaque lot est rangé dans un sous-dossier daté du dossier d'export.",
            },
          ].map((m) => {
            const isChecked = settings.mode === m.id;
            return (
              <label key={m.id} className={`gc-opt ${isChecked ? 'gc-opt--on' : ''}`}>
                <input
                  type="radio"
                  name="write-mode"
                  checked={isChecked}
                  onChange={() => onUpdateSettings({ mode: m.id })}
                />
                <span>
                  <span style={{ display: 'block', fontWeight: 500, fontSize: '15px', lineHeight: '22px' }}>
                    {m.title}
                  </span>
                  <span className="gc-caption" style={{ display: 'block' }}>
                    {m.desc}
                  </span>
                </span>
              </label>
            );
          })}
        </div>

        <label className="gc-check-row">
          <input
            type="checkbox"
            checked={settings.preserveExif}
            onChange={(e) => onUpdateSettings({ preserveExif: e.target.checked })}
          />
          <span>
            <span style={{ display: 'block', fontWeight: 500, fontSize: '15px', lineHeight: '22px' }}>
              Conserver les métadonnées (EXIF)
            </span>
            <span className="gc-caption" style={{ display: 'block' }}>
              Date, appareil et orientation des images sont reportés dans le fichier de sortie.
            </span>
          </span>
        </label>
      </div>

      {/* Bouton réinitialiser */}
      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <Button variant="secondary" onClick={onResetDefaults}>
          Rétablir les valeurs par défaut
        </Button>
      </div>
    </div>
  );
};
