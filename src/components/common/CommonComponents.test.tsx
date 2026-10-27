import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { Button } from './Button';
import { Badge } from './Badge';
import {
  IconDashboard,
  IconSettings,
  IconUpload,
  IconDownload,
  IconImage,
  IconVideo,
  IconPlay,
  IconFolder,
  IconClose,
  IconCheck,
  IconAlertTriangle,
} from './Icons';

describe('Common UI Components', () => {
  describe('Button', () => {
    it('rend correctement un bouton primaire et réagit au clic', () => {
      const onClick = vi.fn();
      render(<Button onClick={onClick}>Lancer</Button>);

      const btn = screen.getByRole('button', { name: 'Lancer' });
      expect(btn).toHaveClass('gc-btn');
      fireEvent.click(btn);
      expect(onClick).toHaveBeenCalledTimes(1);
    });

    it('supporte les variantes secondaires et la taille small', () => {
      render(
        <Button variant="secondary" size="small" disabled>
          Annuler
        </Button>
      );
      const btn = screen.getByRole('button', { name: 'Annuler' });
      expect(btn).toHaveClass('gc-btn--secondary');
      expect(btn).toHaveClass('gc-btn--small');
      expect(btn).toBeDisabled();
    });
  });

  describe('Badge', () => {
    it('rend le contenu du badge avec sa classe', () => {
      render(<Badge className="custom-badge">Max 20</Badge>);
      const badge = screen.getByText('Max 20');
      expect(badge).toHaveClass('gc-badge');
      expect(badge).toHaveClass('custom-badge');
    });
  });

  describe('Icons', () => {
    it('rend tous les glyphes vectoriels SVG', () => {
      const { container } = render(
        <div>
          <IconDashboard />
          <IconSettings />
          <IconUpload />
          <IconDownload />
          <IconImage />
          <IconVideo />
          <IconPlay />
          <IconFolder />
          <IconClose />
          <IconCheck />
          <IconAlertTriangle />
        </div>
      );
      const svgs = container.querySelectorAll('svg');
      expect(svgs.length).toBe(11);
    });
  });
});
