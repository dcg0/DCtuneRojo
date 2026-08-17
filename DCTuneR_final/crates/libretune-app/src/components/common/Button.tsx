/**
 * Shared Button primitive
 *
 * Standardized button with variants. Replaces ad-hoc button styling
 * sprinkled across dialogs and panels. Uses theme tokens for color.
 */

import React from 'react';
import './Button.css';

export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost' | 'link';
export type ButtonSize = 'sm' | 'md' | 'lg';

export interface ButtonProps
  extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'type'> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** Render an icon before the label. */
  leadingIcon?: React.ReactNode;
  /** Render an icon after the label. */
  trailingIcon?: React.ReactNode;
  /** Stretch to full width of parent. */
  block?: boolean;
  /** HTML button type. Default 'button' so it doesn't submit forms accidentally. */
  type?: 'button' | 'submit' | 'reset';
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    variant = 'secondary',
    size = 'md',
    leadingIcon,
    trailingIcon,
    block = false,
    type = 'button',
    className,
    children,
    ...rest
  },
  ref,
) {
  const cls = [
    'lt-btn',
    `lt-btn--${variant}`,
    `lt-btn--${size}`,
    block ? 'lt-btn--block' : null,
    className,
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <button ref={ref} type={type} className={cls} {...rest}>
      {leadingIcon ? <span className="lt-btn__icon">{leadingIcon}</span> : null}
      {children != null ? <span className="lt-btn__label">{children}</span> : null}
      {trailingIcon ? <span className="lt-btn__icon">{trailingIcon}</span> : null}
    </button>
  );
});

export default Button;
