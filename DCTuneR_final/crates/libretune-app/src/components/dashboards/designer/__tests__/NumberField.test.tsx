import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import NumberField from '../NumberField';

describe('NumberField', () => {
  it('renders the raw value as text', () => {
    render(<NumberField label="Min" value={12} onChange={vi.fn()} />);
    expect(screen.getByLabelText('Min')).toHaveValue('12');
  });

  it('renders an empty field for null/undefined', () => {
    render(<NumberField label="Warning" value={null} onChange={vi.fn()} nullable />);
    expect(screen.getByLabelText('Warning')).toHaveValue('');
  });

  it('lets a trailing decimal point stay while typing instead of snapping back', () => {
    render(<NumberField label="Min" value={1} onChange={vi.fn()} />);
    const input = screen.getByLabelText('Min') as HTMLInputElement;

    fireEvent.focus(input);
    // Simulate typing "1.5" one character at a time -- the old plain
    // <input type="number" value={gauge.min}> bug stripped the trailing "."
    // on every keystroke, turning "1.5" into "15".
    fireEvent.change(input, { target: { value: '1.' } });
    expect(input.value).toBe('1.');
    fireEvent.change(input, { target: { value: '1.5' } });
    expect(input.value).toBe('1.5');
  });

  it('propagates a valid parsed number while typing', () => {
    const onChange = vi.fn();
    render(<NumberField label="Min" value={0} onChange={onChange} />);
    const input = screen.getByLabelText('Min');

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: '42' } });

    expect(onChange).toHaveBeenCalledWith(42);
  });

  it('does not call onChange for an in-progress non-numeric value when not nullable', () => {
    const onChange = vi.fn();
    render(<NumberField label="Min" value={5} onChange={onChange} />);
    const input = screen.getByLabelText('Min');

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: '' } });

    expect(onChange).not.toHaveBeenCalled();
  });

  it('never displays "NaN": typing a lone "-" stays as "-" locally, and commits null once nullable', () => {
    // This is the exact corruption the old high_warning/high_critical fields
    // hit: value={gauge.high_warning ?? ''} with
    // onChange={(e) => updateGauge({ high_warning: e.target.value ? parseFloat(e.target.value) : null })}
    // parsed "-" (truthy) via parseFloat to NaN, and NaN ?? '' does not
    // fall back to '', so the input rendered the literal text "NaN".
    const onChange = vi.fn();
    render(<NumberField label="Warning" value={null} onChange={onChange} nullable />);
    const input = screen.getByLabelText('Warning') as HTMLInputElement;

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: '-' } });

    expect(input.value).toBe('-');
    expect(input.value).not.toBe('NaN');
    // A lone "-" isn't a finite number yet, so nothing was committed for it
    // specifically -- but it also must not have committed NaN.
    expect(onChange).not.toHaveBeenCalledWith(NaN);

    fireEvent.change(input, { target: { value: '-5' } });
    expect(input.value).toBe('-5');
    expect(onChange).toHaveBeenCalledWith(-5);
  });

  it('commits null when a nullable field is cleared', () => {
    const onChange = vi.fn();
    render(<NumberField label="Warning" value={5} onChange={onChange} nullable />);
    const input = screen.getByLabelText('Warning');

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: '' } });

    expect(onChange).toHaveBeenCalledWith(null);
  });

  it('reformats to the canonical value on blur', () => {
    render(<NumberField label="Min" value={1} onChange={vi.fn()} />);
    const input = screen.getByLabelText('Min') as HTMLInputElement;

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: '1.' } });
    expect(input.value).toBe('1.');

    fireEvent.blur(input);
    expect(input.value).toBe('1');
  });

  it('updates the displayed value when the prop changes while unfocused', () => {
    const { rerender } = render(<NumberField label="Min" value={1} onChange={vi.fn()} />);
    rerender(<NumberField label="Min" value={7} onChange={vi.fn()} />);
    expect(screen.getByLabelText('Min')).toHaveValue('7');
  });

  it('parses as an integer when integer is set', () => {
    const onChange = vi.fn();
    render(<NumberField label="Digits" value={1} onChange={onChange} integer />);
    const input = screen.getByLabelText('Digits');

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: '3.7' } });

    expect(onChange).toHaveBeenCalledWith(3);
  });
});
