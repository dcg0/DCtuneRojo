import { useEffect, useId, useRef, useState } from 'react';

interface PercentFieldProps {
  label: string;
  /** Stored value as a 0..1 fraction (e.g. 0.02 for "2.0"). */
  value: number;
  onChange: (fraction: number) => void;
}

/**
 * A percent-valued number input backed by a 0..1 fraction.
 *
 * A plain `value={(value * 100).toFixed(1)}` fights the user: every
 * keystroke re-renders with the canonical formatted string, discarding
 * in-progress edits (a trailing ".", a backspaced trailing zero, etc.) —
 * so Backspace visually does nothing. This keeps a local text buffer while
 * the field has focus, and only re-syncs from the external value on blur or
 * when it changes while the field isn't focused (e.g. dragging on canvas).
 */
export default function PercentField({ label, value, onChange }: PercentFieldProps) {
  const [text, setText] = useState(() => (value * 100).toFixed(1));
  const focused = useRef(false);
  const id = useId();

  useEffect(() => {
    if (!focused.current) {
      setText((value * 100).toFixed(1));
    }
  }, [value]);

  return (
    <div className="property-group half">
      <label htmlFor={id}>{label}</label>
      <input
        id={id}
        type="text"
        inputMode="decimal"
        value={text}
        onFocus={() => {
          focused.current = true;
        }}
        onChange={(e) => {
          const raw = e.target.value;
          setText(raw);
          const parsed = parseFloat(raw);
          if (Number.isFinite(parsed)) {
            onChange(parsed / 100);
          }
        }}
        onBlur={() => {
          focused.current = false;
          setText((value * 100).toFixed(1));
        }}
      />
    </div>
  );
}
