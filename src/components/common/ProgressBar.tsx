import React from 'react';

export interface ProgressBarProps {
  progress: number; // 0 to 100
  isDone?: boolean;
  isError?: boolean;
  label?: string;
  className?: string;
  style?: React.CSSProperties;
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
  progress,
  isDone = false,
  isError = false,
  label = 'Progression',
  className = '',
  style,
}) => {
  const clampedProgress = Math.max(0, Math.min(100, Math.round(progress)));
  const statusClass = isError ? 'gc-progress--error' : isDone ? 'gc-progress--done' : '';

  return (
    <div
      className={`gc-progress ${statusClass} ${className}`.trim()}
      role="progressbar"
      aria-label={label}
      aria-invalid={isError ? 'true' : undefined}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={clampedProgress}
      style={style}
    >
      <span style={{ width: `${clampedProgress}%` }} />
    </div>
  );
};
