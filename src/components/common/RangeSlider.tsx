import React from 'react';

export interface RangeSliderProps {
  id: string;
  min: number;
  max: number;
  step?: number;
  value: number;
  onChange: (value: number) => void;
  label?: string;
  displayValue?: string;
  minLabel?: string;
  maxLabel?: string;
  helpText?: string;
  className?: string;
}

export const RangeSlider: React.FC<RangeSliderProps> = ({
  id,
  min,
  max,
  step = 1,
  value,
  onChange,
  label,
  displayValue,
  minLabel,
  maxLabel,
  helpText,
  className = '',
}) => {
  const percent = Math.max(0, Math.min(1, (value - min) / (max - min)));
  const fillWidth = `calc(10px + (100% - 20px) * ${percent})`;

  return (
    <div className={`range-slider-wrapper ${className}`.trim()}>
      {(label || displayValue) && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', alignItems: 'center', justifyContent: 'space-between' }}>
          {label && (
            <label htmlFor={id} className="gc-label" style={{ margin: 0, fontWeight: 600 }}>
              {label}
            </label>
          )}
          {displayValue && <span className="gc-badge">{displayValue}</span>}
        </div>
      )}

      <div style={{ position: 'relative', height: '20px', marginTop: '14px' }}>
        <div style={{ position: 'absolute', left: 0, right: 0, top: '6px', height: '8px', borderRadius: '9999px', background: 'var(--track)' }} />
        <div
          style={{
            position: 'absolute',
            left: 0,
            top: '6px',
            height: '8px',
            borderRadius: '9999px',
            background: 'linear-gradient(90deg, var(--azure-500), var(--cyan-400))',
            width: fillWidth,
          }}
        />
        <input
          id={id}
          className="gc-range"
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
        />
      </div>

      {(minLabel || maxLabel) && (
        <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '8px' }}>
          <span className="gc-caption">{minLabel ?? min}</span>
          <span className="gc-caption">{maxLabel ?? max}</span>
        </div>
      )}

      {helpText && <div className="gc-caption" style={{ marginTop: '2px' }}>{helpText}</div>}
    </div>
  );
};
