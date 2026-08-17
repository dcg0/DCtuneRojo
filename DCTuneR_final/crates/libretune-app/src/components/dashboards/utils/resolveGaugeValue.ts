import { TsGaugeConfig } from '../dashTypes';

interface ResolveGaugeValueArgs {
  sweepActive: boolean;
  sweepValues: Record<string, number>;
  gaugeDemoActive: boolean;
  demoValues: Record<string, number>;
}

/**
 * Picks the value a gauge should render with, outside of its own live
 * store subscription (sweep/demo preview modes, or the fallback for when
 * there's no live data at all).
 *
 * `gauge.value` is a static, design-time preview number saved with the
 * dashboard file — not live data. Using it as the live/no-data fallback
 * meant a disconnected gauge (or one that just remounted, e.g. after
 * switching tabs) could show a plausible-looking but entirely fake
 * reading with no indication it wasn't real (Issue #73). `gauge.min`
 * matches the convention already used one branch up for sweep mode.
 */
export function resolveGaugeValue(
  gauge: Pick<TsGaugeConfig, 'output_channel' | 'min' | 'value'>,
  { sweepActive, sweepValues, gaugeDemoActive, demoValues }: ResolveGaugeValueArgs,
): number {
  if (sweepActive) {
    return sweepValues[gauge.output_channel] ?? gauge.min;
  }
  if (gaugeDemoActive) {
    return demoValues[gauge.output_channel] ?? gauge.value;
  }
  return gauge.min;
}
