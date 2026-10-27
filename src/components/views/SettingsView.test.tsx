import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { SettingsView } from './SettingsView';
import { BatchSettings } from '../../types/ipc';

describe('SettingsView Component', () => {
  const defaultSettings: BatchSettings = {
    targetMinBytes: 800 * 1024,
    targetMaxBytes: 2000 * 1024,
    targetBytes: 1104 * 1024,
    qMin: 65,
    qMax: 92,
    threads: 4,
    crf: 26,
    preserveExif: true,
    mode: 'copy',
    outputDirectory: '/export',
  };

  it('rend correctement tous les paramètres et la simulation dichotomique', () => {
    const onUpdateSettings = vi.fn();
    const onResetDefaults = vi.fn();

    render(
      <SettingsView
        settings={defaultSettings}
        onUpdateSettings={onUpdateSettings}
        onResetDefaults={onResetDefaults}
      />
    );

    expect(screen.getByText('Paramètres du moteur')).toBeInTheDocument();
    expect(screen.getByText("Moteur d'images — recherche dichotomique")).toBeInTheDocument();
    expect(screen.getByText('Moteur vidéo — FFmpeg')).toBeInTheDocument();
    expect(screen.getByText('Écriture non-destructive')).toBeInTheDocument();
    expect(screen.getByText(/Simulation pour une image de ~15 Mo/)).toBeInTheDocument();
  });

  it('permet de changer Qmin et Qmax', () => {
    const onUpdateSettings = vi.fn();
    const onResetDefaults = vi.fn();

    render(
      <SettingsView
        settings={defaultSettings}
        onUpdateSettings={onUpdateSettings}
        onResetDefaults={onResetDefaults}
      />
    );

    const qMinInput = screen.getByLabelText(/Qualité minimale/);
    fireEvent.change(qMinInput, { target: { value: '70' } });
    expect(onUpdateSettings).toHaveBeenCalledWith({ qMin: 70 });

    const qMaxInput = screen.getByLabelText(/Qualité maximale/);
    fireEvent.change(qMaxInput, { target: { value: '88' } });
    expect(onUpdateSettings).toHaveBeenCalledWith({ qMax: 88 });
  });

  it('permet de changer les threads et le CRF', () => {
    const onUpdateSettings = vi.fn();
    const onResetDefaults = vi.fn();

    render(
      <SettingsView
        settings={defaultSettings}
        onUpdateSettings={onUpdateSettings}
        onResetDefaults={onResetDefaults}
      />
    );

    const threadsInput = screen.getByLabelText(/Threads parallèles/);
    fireEvent.change(threadsInput, { target: { value: '8' } });
    expect(onUpdateSettings).toHaveBeenCalledWith({ threads: 8 });

    const crfInput = screen.getByLabelText(/Facteur d'encodage CRF/);
    fireEvent.change(crfInput, { target: { value: '24' } });
    expect(onUpdateSettings).toHaveBeenCalledWith({ crf: 24 });
  });

  it("permet de changer le mode d'export et le toggle EXIF", () => {
    const onUpdateSettings = vi.fn();
    const onResetDefaults = vi.fn();

    render(
      <SettingsView
        settings={defaultSettings}
        onUpdateSettings={onUpdateSettings}
        onResetDefaults={onResetDefaults}
      />
    );

    const suffixRadio = screen.getByLabelText(/Ajouter le suffixe _opt/);
    fireEvent.click(suffixRadio);
    expect(onUpdateSettings).toHaveBeenCalledWith({ mode: 'suffix' });

    const folderRadio = screen.getByLabelText(/Créer un sous-dossier horodaté/);
    fireEvent.click(folderRadio);
    expect(onUpdateSettings).toHaveBeenCalledWith({ mode: 'folder' });

    const exifCheckbox = screen.getByLabelText(/Conserver les métadonnées/);
    fireEvent.click(exifCheckbox);
    expect(onUpdateSettings).toHaveBeenCalledWith({ preserveExif: false });
  });

  it('appelle onResetDefaults lors du clic sur le bouton de réinitialisation', () => {
    const onUpdateSettings = vi.fn();
    const onResetDefaults = vi.fn();

    render(
      <SettingsView
        settings={defaultSettings}
        onUpdateSettings={onUpdateSettings}
        onResetDefaults={onResetDefaults}
      />
    );

    const resetBtn = screen.getByRole('button', { name: /Rétablir les valeurs par défaut/ });
    fireEvent.click(resetBtn);
    expect(onResetDefaults).toHaveBeenCalledTimes(1);
  });

  it('gère le cas où qMin == qMax dans la simulation dichotomique', () => {
    const onUpdateSettings = vi.fn();
    const onResetDefaults = vi.fn();

    render(
      <SettingsView
        settings={{ ...defaultSettings, qMin: 80, qMax: 80 }}
        onUpdateSettings={onUpdateSettings}
        onResetDefaults={onResetDefaults}
      />
    );

    expect(screen.getByText('(bornes identiques)')).toBeInTheDocument();
  });
});
