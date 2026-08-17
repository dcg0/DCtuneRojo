import { describe, expect, it } from 'vitest';
import { resolveGaugeValue } from '../resolveGaugeValue';

const gauge = { output_channel: 'rpm', min: 0, value: 6.833777777777774 };

describe('resolveGaugeValue', () => {
  it('falls back to gauge.min, not the stale design-time gauge.value, when there is no active preview', () => {
    const result = resolveGaugeValue(gauge, {
      sweepActive: false,
      sweepValues: {},
      gaugeDemoActive: false,
      demoValues: {},
    });
    expect(result).toBe(0);
  });

  it('uses the sweep value when sweep is active', () => {
    const result = resolveGaugeValue(gauge, {
      sweepActive: true,
      sweepValues: { rpm: 3500 },
      gaugeDemoActive: false,
      demoValues: {},
    });
    expect(result).toBe(3500);
  });

  it('falls back to gauge.min (not gauge.value) when sweep is active but this channel has no sweep value yet', () => {
    const result = resolveGaugeValue(gauge, {
      sweepActive: true,
      sweepValues: {},
      gaugeDemoActive: false,
      demoValues: {},
    });
    expect(result).toBe(0);
  });

  it('uses the demo value when demo mode is active', () => {
    const result = resolveGaugeValue(gauge, {
      sweepActive: false,
      sweepValues: {},
      gaugeDemoActive: true,
      demoValues: { rpm: 4200 },
    });
    expect(result).toBe(4200);
  });

  it('falls back to gauge.value (the design-time preview) when demo mode is active but this channel has no demo value yet', () => {
    const result = resolveGaugeValue(gauge, {
      sweepActive: false,
      sweepValues: {},
      gaugeDemoActive: true,
      demoValues: {},
    });
    expect(result).toBe(gauge.value);
  });

  it('prefers sweep over demo when both are somehow active', () => {
    const result = resolveGaugeValue(gauge, {
      sweepActive: true,
      sweepValues: { rpm: 1000 },
      gaugeDemoActive: true,
      demoValues: { rpm: 2000 },
    });
    expect(result).toBe(1000);
  });
});
