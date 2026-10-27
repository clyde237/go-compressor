import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ModalAlert } from './ModalAlert';

describe('ModalAlert Component (Accessibilité WAI-ARIA)', () => {
  it('ne doit rien afficher lorsque isOpen est faux', () => {
    const onClose = vi.fn();
    render(<ModalAlert isOpen={false} title="Test" body="Corps" onClose={onClose} />);

    expect(screen.queryByRole('alertdialog')).toBeNull();
  });

  it('doit respecter strictement les attributs WAI-ARIA alertdialog', () => {
    const onClose = vi.fn();
    render(
      <ModalAlert
        isOpen={true}
        title="Quota vidéo dépassé"
        body="Limite dépassée : 20 vidéos maximum par lot."
        detail="Retirez des vidéos avant d'en ajouter."
        onClose={onClose}
      />
    );

    const dialog = screen.getByRole('alertdialog');
    expect(dialog).toBeDefined();
    expect(dialog.getAttribute('aria-modal')).toBe('true');
    expect(dialog.getAttribute('aria-labelledby')).toBe('alert-title');
    expect(dialog.getAttribute('aria-describedby')).toBe('alert-body');

    expect(screen.getByText('Quota vidéo dépassé')).toBeDefined();
    expect(screen.getByText('Limite dépassée : 20 vidéos maximum par lot.')).toBeDefined();
    expect(screen.getByText("Retirez des vidéos avant d'en ajouter.")).toBeDefined();
  });

  it('doit fermer la modale au clic sur le bouton Compris', () => {
    const onClose = vi.fn();
    render(
      <ModalAlert
        isOpen={true}
        title="Alerte"
        body="Message"
        onClose={onClose}
      />
    );

    const button = screen.getByRole('button', { name: /compris/i });
    fireEvent.click(button);

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('doit fermer la modale lors de la pression sur la touche Échap (Escape)', () => {
    const onClose = vi.fn();
    render(
      <ModalAlert
        isOpen={true}
        title="Alerte"
        body="Message à fermer avec Échap"
        onClose={onClose}
      />
    );

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('doit piéger le focus avec la touche Tab', () => {
    const onClose = vi.fn();
    render(
      <ModalAlert
        isOpen={true}
        title="Alerte Focus"
        body="Test du focus trap"
        onClose={onClose}
      />
    );

    const button = screen.getByRole('button', { name: /compris/i });
    button.focus();
    expect(document.activeElement).toBe(button);

    // Appui sur Tab : doit reboucler sur le bouton
    fireEvent.keyDown(window, { key: 'Tab' });
    expect(document.activeElement).toBe(button);
  });
});
