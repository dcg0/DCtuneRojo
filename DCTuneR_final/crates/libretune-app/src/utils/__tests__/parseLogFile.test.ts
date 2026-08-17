//! Tests for the recorded-datalog parser.
//!
//! The `.msl` cases encode the shape of real TunerStudio exports: a preamble
//! before the header, a units row after it, tab delimiters, timestamps that
//! are neither zero-based nor monotonic, and enum channels emitting strings in
//! otherwise numeric columns. Each fixture below exists because a parser that
//! assumed otherwise silently produced an empty log.

import { describe, it, expect } from 'vitest';
import { parseLogFile } from '../parseLogFile';

/**
 * A minimal but structurally faithful `.msl`: two preamble lines, a `#`
 * separator, a tab-separated header, a units row, then data. `Time` is in
 * seconds and negative, as it is in logs exported from a longer `.mlg`.
 */
const MSL_FIXTURE = [
  '"speeduino 202501: Speeduino 2025.01.4"',
  '"Capture Date: Sat Sep 20 12:10:33 BST 2025, File author: TunerStudio MS version 3.2.05"',
  '#',
  'Time\tRPM\tMAP\tAFR\tDFCO',
  's\trpm\tkpa\tO2\t',
  '-11923.118\t1450\t38.0\t14.7\tOff',
  '-11923.050\t1502\t41.5\t14.6\tOff',
  '-11922.921\t1610\t44.0\t13.9\tOn',
].join('\n');

const CSV_FIXTURE = [
  'timestamp_ms,RPM,MAP,AFR',
  '0,1450,38.0,14.7',
  '100,1502,41.5,14.6',
].join('\n');

describe('parseLogFile — TunerStudio .msl', () => {
  it('skips the preamble and units row and reads every data sample', () => {
    const { data, channels } = parseLogFile(MSL_FIXTURE);

    // Three data rows — the units row must not be counted as a sample, which
    // is what an off-by-one in the preamble skip would produce.
    expect(data).toHaveLength(3);
    expect(channels).toEqual(['RPM', 'MAP', 'AFR', 'DFCO']);
  });

  it('parses tab-delimited fields rather than treating the row as one column', () => {
    const { data } = parseLogFile(MSL_FIXTURE);
    expect(data[0].values.RPM).toBe(1450);
    expect(data[0].values.MAP).toBeCloseTo(38.0);
    expect(data[2].values.AFR).toBeCloseTo(13.9);
  });

  it('preserves negative, non-zero-based timestamps and converts seconds to ms', () => {
    const { data } = parseLogFile(MSL_FIXTURE);
    // Real exports carry offsets relative to the original recording, so the
    // parser must not normalise, clamp, or reject them.
    expect(data[0].x).toBeCloseTo(-11923118);
    expect(data[2].x).toBeCloseTo(-11922921);
    // Duration must come from the range, never from `x` starting at zero.
    expect(data[2].x - data[0].x).toBeCloseTo(197);
  });

  it('keeps numeric fields on a row where an enum channel logs a string', () => {
    const { data } = parseLogFile(MSL_FIXTURE);
    // `DFCO` is `Off`/`On`; the row must survive with its numeric channels
    // intact rather than being discarded wholesale.
    expect(data[0].values.RPM).toBe(1450);
    expect(data[0].values.DFCO).toBeUndefined();
  });
});

describe('parseLogFile — LibreTune .csv', () => {
  it('still parses comma-separated logs with a millisecond timestamp', () => {
    const { data, channels } = parseLogFile(CSV_FIXTURE);
    expect(channels).toEqual(['RPM', 'MAP', 'AFR']);
    expect(data).toHaveLength(2);
    // `timestamp_ms` is already milliseconds and must not be scaled again.
    expect(data[0].x).toBe(0);
    expect(data[1].x).toBe(100);
  });
});

describe('parseLogFile — malformed input', () => {
  it('returns no data for content with no recognisable header', () => {
    // The caller surfaces this to the user; silently returning an empty log
    // is what made a failed load look like the button had done nothing.
    const { data } = parseLogFile('not a log\njust text\n');
    expect(data).toHaveLength(0);
  });

  it('returns no data for an empty file', () => {
    expect(parseLogFile('').data).toHaveLength(0);
  });
});
