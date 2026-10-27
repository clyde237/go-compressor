import React, { useEffect } from 'react';
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
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div className="gc-overlay" onClick={onClose} role="presentation">
      <div
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
