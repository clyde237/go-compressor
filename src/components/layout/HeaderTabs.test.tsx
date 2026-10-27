import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { HeaderTabs } from './HeaderTabs';

describe('HeaderTabs Component', () => {
  it('doit rendre les 2 onglets principaux (Tableau de bord et Paramètres)', () => {
    const onTabChange = vi.fn();
    render(<HeaderTabs activeTab="dash" onTabChange={onTabChange} />);

    expect(screen.getByText('Tableau de bord')).toBeDefined();
    expect(screen.getByText('Paramètres')).toBeDefined();
    expect(screen.getByAltText('GO COMPRESSOR')).toBeDefined();
    expect(screen.queryByText('Fichiers')).toBeNull();
    expect(screen.queryByText("Dossier d'export")).toBeNull();
  });

  it("doit marquer l'onglet actif avec aria-selected='true'", () => {
    const onTabChange = vi.fn();
    render(<HeaderTabs activeTab="settings" onTabChange={onTabChange} />);

    const settingsTab = screen.getByRole('tab', { name: /paramètres/i });
    expect(settingsTab.getAttribute('aria-selected')).toBe('true');

    const dashTab = screen.getByRole('tab', { name: /tableau de bord/i });
    expect(dashTab.getAttribute('aria-selected')).toBe('false');
  });

  it('doit appeler onTabChange lors du clic sur un onglet', () => {
    const onTabChange = vi.fn();
    render(<HeaderTabs activeTab="dash" onTabChange={onTabChange} />);

    const settingsTab = screen.getByRole('tab', { name: /paramètres/i });
    fireEvent.click(settingsTab);

    expect(onTabChange).toHaveBeenCalledWith('settings');
  });
});
