import { describe, expect, it } from 'vitest';
import { isHidden, withHidden } from '../LayerPanel';
import type { DashComponent } from '../../dashTypes';

function makeGauge(overrides: Record<string, unknown> = {}): DashComponent {
  return {
    Gauge: {
      id: 'g1',
      ...overrides,
    },
  } as unknown as DashComponent;
}

function makeIndicator(overrides: Record<string, unknown> = {}): DashComponent {
  return {
    Indicator: {
      id: 'i1',
      ...overrides,
    },
  } as unknown as DashComponent;
}

describe('LayerPanel hide/show', () => {
  it('hides a gauge with no prior condition and shows it again with no condition restored', () => {
    const gauge = makeGauge({ enabled_condition: null });

    const hidden = withHidden(gauge, true);
    expect(isHidden(hidden)).toBe(true);
    expect((hidden as { Gauge: { enabled_condition: string | null } }).Gauge.enabled_condition).toBe('false');

    const shown = withHidden(hidden, false);
    expect(isHidden(shown)).toBe(false);
    expect((shown as { Gauge: { enabled_condition: string | null } }).Gauge.enabled_condition).toBeNull();
  });

  it('preserves a real enabled_condition across hide, and restores it exactly on show', () => {
    // Regression test: before this fix, hiding a gauge that already had a
    // real conditional-visibility expression (set via PropertyEditor)
    // silently overwrote it with the literal "false", and showing it again
    // reset it to null instead of the original expression -- permanently
    // destroying it with no warning.
    const gauge = makeGauge({ enabled_condition: 'rpm > 0' });

    const hidden = withHidden(gauge, true);
    expect(isHidden(hidden)).toBe(true);
    const hiddenGauge = (hidden as { Gauge: { enabled_condition: string | null; extra_attrs: Record<string, string> } }).Gauge;
    expect(hiddenGauge.enabled_condition).toBe('false');
    // The original expression must survive somewhere, not be discarded.
    expect(hiddenGauge.extra_attrs.lt_prev_enabled_condition).toBe('rpm > 0');

    const shown = withHidden(hidden, false);
    const shownGauge = (shown as { Gauge: { enabled_condition: string | null; extra_attrs: Record<string, string> } }).Gauge;
    expect(isHidden(shown)).toBe(false);
    // The real expression is restored exactly, not reset to null.
    expect(shownGauge.enabled_condition).toBe('rpm > 0');
    // The stash key doesn't linger around after being restored.
    expect(shownGauge.extra_attrs.lt_prev_enabled_condition).toBeUndefined();
  });

  it('does the same preserve/restore for indicators', () => {
    const indicator = makeIndicator({ enabled_condition: 'hasLambdaSensor' });

    const hidden = withHidden(indicator, true);
    const shown = withHidden(hidden, false);
    const shownIndicator = (shown as { Indicator: { enabled_condition: string | null } }).Indicator;

    expect(shownIndicator.enabled_condition).toBe('hasLambdaSensor');
  });

  it('hiding an already-hidden component does not clobber a previously-stashed condition', () => {
    // If withHidden(true) is ever called twice in a row (e.g. a stale
    // double-click), the second call must not treat the hide marker itself
    // as "the real condition to preserve" and overwrite the actual stash.
    const gauge = makeGauge({ enabled_condition: 'rpm > 0' });
    const hiddenOnce = withHidden(gauge, true);
    const hiddenTwice = withHidden(hiddenOnce, true);

    const attrs = (hiddenTwice as { Gauge: { extra_attrs: Record<string, string> } }).Gauge.extra_attrs;
    expect(attrs.lt_prev_enabled_condition).toBe('rpm > 0');

    const shown = withHidden(hiddenTwice, false);
    expect((shown as { Gauge: { enabled_condition: string | null } }).Gauge.enabled_condition).toBe('rpm > 0');
  });

  it('preserves other extra_attrs entries untouched (e.g. trend series config)', () => {
    const gauge = makeGauge({
      enabled_condition: 'rpm > 0',
      extra_attrs: { lt_series2_channel: 'boost' },
    });

    const hidden = withHidden(gauge, true);
    const shown = withHidden(hidden, false);
    const attrs = (shown as { Gauge: { extra_attrs: Record<string, string> } }).Gauge.extra_attrs;

    expect(attrs.lt_series2_channel).toBe('boost');
    expect(attrs.lt_prev_enabled_condition).toBeUndefined();
  });
});
