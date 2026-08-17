import { useEffect, useId, useRef, useState } from 'react';

interface NumberFieldProps {
  label: string;
  /** Current stored value. `null`/`undefined` renders as an empty field. */
  value: number | null | undefined;
  onChange: (value: number | null) => void;
  /**
   * When true, leaving the field empty commits `null` (e.g. an optional
   * warning/critical threshold with no value configured). When false
   * (default), leaving the field empty on blur reverts the display back to
   * the last valid value instead of committing anything — matching how a
   * required numeric field (Min, Max, Digits) should behave when abandoned
   * mid-edit rather than snapping to an arbitrary fallback number.
   */
  nullable?: boolean;
  integer?: boolean;
  step?: number;
  min?: number;
  max?: number;
  placeholder?: string;
}

function formatValue(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(value)) return '';
  return String(value);
}

/**
 * A plain (non-percent) numeric input that keeps a local text buffer while
 * focused, so in-progress edits (a trailing "-", a trailing ".", clearing
 * the field to retype) aren't discarded by the controlled re-render on
 * every keystroke.
 *
 * Same bug class and fix shape as PercentField (see its doc comment), but
 * for fields that store a raw value instead of a 0..1 fraction, and that
 * may be nullable. Before this existed, PropertyEditor's Min/Max/Warning/
 * Critical/Digits/Hysteresis fields bound `value={rawNumber}` directly and
 * parsed on every keystroke: typing "-" into a nullable field parsed to
 * NaN and rendered the input as the literal text "NaN"; typing a trailing
 * "." into Min/Max got silently stripped every keystroke, so "1.5" could
 * never be typed digit-by-digit.
 */
export default function NumberField({
  label,
  value,
  onChange,
  nullable = false,
  integer = false,
  step,
  min,
  max,
  placeholder,
}: NumberFieldProps) {
  const [text, setText] = useState(() => formatValue(value));
  const focused = useRef(false);
  const id = useId();

  useEffect(() => {
    if (!focused.current) {
      setText(formatValue(value));
    }
  }, [value]);

  return (
    <input
      id={id}
      aria-label={label}
      type="text"
      inputMode={integer ? 'numeric' : 'decimal'}
      step={step}
      min={min}
      max={max}
      placeholder={placeholder}
      value={text}
      onFocus={() => {
        focused.current = true;
      }}
      onChange={(e) => {
        const raw = e.target.value;
        setText(raw);
        if (raw.trim() === '') {
          if (nullable) onChange(null);
          return;
        }
        const parsed = integer ? parseInt(raw, 10) : parseFloat(raw);
        if (Number.isFinite(parsed)) {
          onChange(parsed);
        }
      }}
      onBlur={() => {
        focused.current = false;
        setText(formatValue(value));
      }}
    />
  );
}
