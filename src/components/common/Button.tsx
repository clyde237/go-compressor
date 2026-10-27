import React from 'react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary';
  size?: 'normal' | 'small';
  children: React.ReactNode;
}

export const Button: React.FC<ButtonProps> = ({
  variant = 'primary',
  size = 'normal',
  className = '',
  children,
  disabled,
  ...props
}) => {
  const variantClass = variant === 'secondary' ? 'gc-btn--secondary' : '';
  const sizeClass = size === 'small' ? 'gc-btn--small' : '';
  const combinedClass = `gc-btn ${variantClass} ${sizeClass} ${className}`.trim();

  return (
    <button
      type="button"
      className={combinedClass}
      disabled={disabled}
      {...props}
    >
      {children}
    </button>
  );
};
