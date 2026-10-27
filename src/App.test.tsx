import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { App } from './App';

// Mock Tauri API calls
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    total_files: 0,
    successful_files: 0,
    failed_files: 0,
    original_total_bytes: 0,
    compressed_total_bytes: 0,
    average_reduction_percent: 0,
    duration_secs: 0,
    items: [],
  }),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn().mockResolvedValue(null),
  save: vi.fn().mockResolvedValue(null),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openPath: vi.fn().mockResolvedValue(undefined),
}));

describe('App Component', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("s'affiche initialement sur l'onglet Tableau de bord", () => {
    render(<App />);
    expect(screen.getByRole('tab', { name: /Tableau de bord/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText(/Glissez-déposez vos photos ici/)).toBeInTheDocument();
  });

  it('permet de basculer vers les Paramètres et revenir au Tableau de bord', () => {
    render(<App />);

    const settingsTab = screen.getByRole('tab', { name: /Paramètres/ });
    fireEvent.click(settingsTab);

    expect(settingsTab).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('Paramètres du moteur')).toBeInTheDocument();

    const dashTab = screen.getByRole('tab', { name: /Tableau de bord/ });
    fireEvent.click(dashTab);

    expect(dashTab).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText(/Glissez-déposez vos photos ici/)).toBeInTheDocument();
  });

  it('permet de modifier les paramètres depuis SettingsView', () => {
    render(<App />);

    fireEvent.click(screen.getByRole('tab', { name: /Paramètres/ }));

    const threadsInput = screen.getByLabelText(/Threads parallèles/);
    fireEvent.change(threadsInput, { target: { value: '6' } });

    const resetBtn = screen.getByRole('button', { name: /Rétablir les valeurs par défaut/ });
    fireEvent.click(resetBtn);
  });

  it('permet de modifier le curseur de poids cible sur le tableau de bord', () => {
    render(<App />);

    const slider = screen.getByRole('slider');
    fireEvent.change(slider, { target: { value: '1400' } });

    expect(screen.getByText(/1400 Ko/)).toBeInTheDocument();
  });
});
