import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ModalAlert } from './ModalAlert';

describe('ModalAlert Component', () => {
  it('ne doit rien afficher lorsque isOpen est faux', () => {
    const onClose = vi.fn();
    render(<ModalAlert isOpen={false} title="Test" body="Corps" onClose={onClose} />);

    expect(screen.queryByRole('alertdialog')).toBeNull();
  });

  it('doit afficher le titre, le corps et le détail lorsque isOpen est vrai', () => {
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

    expect(screen.getByRole('alertdialog')).toBeDefined();
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
});
