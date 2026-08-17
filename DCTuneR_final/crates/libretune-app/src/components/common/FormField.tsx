/**
 * Shared FormField primitive
 *
 * Wraps an input/select/textarea with a label, optional help text, and
 * error message. Standardizes form layout across dialogs.
 */

import React, { useId } from 'react';
import './FormField.css';

export interface FormFieldProps {
  label: React.ReactNode;
  /** Optional helper text shown below the field. */
  help?: React.ReactNode;
  /** Optional error message. Replaces help when present. */
  error?: React.ReactNode;
  /** Mark as required (renders an asterisk). */
  required?: boolean;
  /** Layout: stacked (label above) or inline (label beside). */
  layout?: 'stacked' | 'inline';
  /** Render the actual input. Receives an id to wire to the label. */
  children: (id: string) => React.ReactNode;
  className?: string;
}

export function FormField({
  label,
  help,
  error,
  required = false,
  layout = 'stacked',
  children,
  className,
}: FormFieldProps) {
  const id = useId();
  return (
    <div
      className={`lt-field lt-field--${layout}${error ? ' lt-field--error' : ''}${className ? ` ${className}` : ''}`}
    >
      <label htmlFor={id} className="lt-field__label">
        {label}
        {required && <span className="lt-field__required" aria-hidden>*</span>}
      </label>
      <div className="lt-field__control">
        {children(id)}
        {error ? (
          <div className="lt-field__error" role="alert">
            {error}
          </div>
        ) : help ? (
          <div className="lt-field__help">{help}</div>
        ) : null}
      </div>
    </div>
  );
}

export default FormField;
