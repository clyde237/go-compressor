import React, { useEffect, useRef } from 'react';
import { IconAlertTriangle } from './Icons';
import { Button } from './Button';

export interface ModalAlertProps {
  isOpen: boolean;
  title: string;
  body: string;
  detail?: string;
  onClose: () => void;
  confirmLabel?: string;
}

export const ModalAlert: React.FC<ModalAlertProps> = ({
  isOpen,
  title,
  body,
  detail,
  onClose,
  confirmLabel = 'Compris',
}) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  const previouslyFocusedElementRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!isOpen) return;

    // Sauvegarde du focus précédent pour restauration à la fermeture (WAI-ARIA)
    previouslyFocusedElementRef.current = document.activeElement as HTMLElement | null;

    // Focus automatique sur le premier élément actionnable de la modale
    const timer = setTimeout(() => {
      const focusable = dialogRef.current?.querySelector<HTMLElement>(
        'button:not([disabled]), [tabindex]:not([tabindex="-1"])'
      );
      if (focusable) {
        focusable.focus();
      }
    }, 20);

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
        return;
      }

      // Piège à focus (Focus Trap) WAI-ARIA
      if (e.key === 'Tab') {
        if (!dialogRef.current) return;
        const focusables = Array.from(
          dialogRef.current.querySelectorAll<HTMLElement>(
            'button:not([disabled]), [tabindex]:not([tabindex="-1"])'
          )
        );
        if (focusables.length === 0) return;

        const first = focusables[0];
        const last = focusables[focusables.length - 1];

        if (e.shiftKey) {
          if (document.activeElement === first) {
            e.preventDefault();
            last.focus();
          }
        } else {
          if (document.activeElement === last) {
            e.preventDefault();
            first.focus();
          }
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      clearTimeout(timer);
      window.removeEventListener('keydown', handleKeyDown);
      // Restauration du focus à la fermeture
      if (
        previouslyFocusedElementRef.current &&
        typeof previouslyFocusedElementRef.current.focus === 'function'
      ) {
        previouslyFocusedElementRef.current.focus();
      }
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div className="gc-overlay" onClick={onClose} role="presentation">
      <div
        ref={dialogRef}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="alert-title"
        aria-describedby="alert-body"
        onClick={(e) => e.stopPropagation()}
        style={{
          background: 'var(--surface-100)',
          borderRadius: '16px',
          maxWidth: '460px',
          width: '100%',
          boxSizing: 'border-box',
          padding: '24px',
          display: 'flex',
          flexDirection: 'column',
          gap: '12px',
          border: '1px solid var(--border-subtle)',
          boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)',
        }}
      >
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
          <span
            aria-hidden="true"
            style={{
              width: '44px',
              height: '44px',
              borderRadius: '50%',
              background: 'var(--tint-50)',
              display: 'grid',
              placeItems: 'center',
              flex: 'none',
              color: 'var(--brand-text)',
            }}
          >
            <IconAlertTriangle />
          </span>
          <h2 id="alert-title" className="gc-h2" style={{ fontSize: '19px' }}>
            {title}
          </h2>
        </div>

        <p id="alert-body" style={{ margin: 0, fontWeight: 600, fontSize: '15px', lineHeight: '22px' }}>
          {body}
        </p>

        {detail && (
          <p className="gc-muted" style={{ margin: 0, fontSize: '15px', lineHeight: '22px' }}>
            {detail}
          </p>
        )}

        <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '8px' }}>
          <Button onClick={onClose} autoFocus>
            {confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
};
