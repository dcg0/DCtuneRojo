import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import PercentField from '../PercentField';

describe('PercentField', () => {
  it('renders the fraction as a formatted percentage', () => {
    render(<PercentField label="X (%)" value={0.02} onChange={vi.fn()} />);
    expect(screen.getByLabelText('X (%)')).toHaveValue('2.0');
  });

  it('lets backspace shorten the displayed text instead of snapping back', () => {
    render(<PercentField label="X (%)" value={2 / 100} onChange={vi.fn()} />);
    const input = screen.getByLabelText('X (%)') as HTMLInputElement;

    fireEvent.focus(input);
    // Simulate the user backspacing the trailing "0" off "2.0" -> "2."
    fireEvent.change(input, { target: { value: '2.' } });

    expect(input.value).toBe('2.');
  });

  it('propagates a valid parsed fraction while typing', () => {
    const onChange = vi.fn();
    render(<PercentField label="X (%)" value={0} onChange={onChange} />);
    const input = screen.getByLabelText('X (%)');

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: '15' } });

    expect(onChange).toHaveBeenCalledWith(0.15);
  });

  it('does not call onChange for an in-progress non-numeric value', () => {
    const onChange = vi.fn();
    render(<PercentField label="X (%)" value={0.02} onChange={onChange} />);
    const input = screen.getByLabelText('X (%)');

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: '' } });

    expect(onChange).not.toHaveBeenCalled();
  });

  it('reformats to the canonical value on blur', () => {
    render(<PercentField label="X (%)" value={2 / 100} onChange={vi.fn()} />);
    const input = screen.getByLabelText('X (%)') as HTMLInputElement;

    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: '2.' } });
    expect(input.value).toBe('2.');

    fireEvent.blur(input);
    expect(input.value).toBe('2.0');
  });

  it('updates the displayed value when the prop changes while unfocused (e.g. canvas drag)', () => {
    const { rerender } = render(<PercentField label="X (%)" value={0.02} onChange={vi.fn()} />);
    rerender(<PercentField label="X (%)" value={0.5} onChange={vi.fn()} />);
    expect(screen.getByLabelText('X (%)')).toHaveValue('50.0');
  });
});
