/**
 * Parser for recorded datalog files.
 *
 * Handles the two text formats the Load dialog offers:
 *
 * - **LibreTune CSV** — comma-separated, header on the first line, a
 *   `timestamp_ms` column in milliseconds.
 * - **TunerStudio `.msl`** — TAB-separated, and prefixed with a preamble that
 *   must be skipped before the real header:
 *
 *   ```
 *   "speeduino 202501: Speeduino 2025.01.4"      <- ECU signature
 *   "Capture Date: ..., File author: ..."        <- capture metadata
 *   #                                            <- separator
 *   Time<TAB>SecL<TAB>RPM<TAB>...                <- real header
 *   s<TAB>sec<TAB>rpm<TAB>...                    <- units row, NOT data
 *   <data rows>
 *   ```
 *
 * Rather than hard-coding those line offsets (TunerStudio's preamble length is
 * not guaranteed), the header is located by scanning the first few lines for
 * one that splits into multiple fields and carries a recognised time column.
 *
 * Two properties of real `.msl` exports drive the remaining decisions:
 *
 * - **Timestamps are not necessarily zero-based, positive, or monotonic.** A
 *   log exported from a longer `.mlg` recording carries offsets relative to
 *   the *original* recording, so values can be large and negative, and the
 *   first row may be a `0.0` marker that precedes them. Callers must derive
 *   duration from the data range rather than assuming `x` starts at zero.
 * - **Enum channels emit strings** (`Off` / `On`) in otherwise numeric
 *   columns, so a field that fails to parse is skipped individually instead of
 *   discarding the whole sample.
 */

/** One parsed sample: `x` is the timestamp in milliseconds. */
export interface LogSample {
  x: number;
  values: Record<string, number>;
}

export interface ParsedLog {
  data: LogSample[];
  channels: string[];
}

/** Header names accepted as the time column, lower-cased. */
const TIME_COLUMN_NAMES = ['time', 'timestamp_ms', 'timestamp'];

/** How far into the file to look for the header before giving up. */
const MAX_HEADER_SCAN_LINES = 10;

/** Fallback spacing when a log has no time column at all. */
const FALLBACK_SAMPLE_INTERVAL_MS = 100;

function splitFields(line: string, delimiter: string): string[] {
  return line.split(delimiter).map(f => f.trim().replace(/^"|"$/g, ''));
}

function isTimeColumn(name: string): boolean {
  return TIME_COLUMN_NAMES.includes(name.toLowerCase());
}

/**
 * Split one data row, honouring quoted fields so a delimiter inside quotes
 * does not split the field.
 */
function splitRow(line: string, delimiter: string): string[] {
  const values: string[] = [];
  let current = '';
  let inQuotes = false;

  for (const char of line) {
    if (char === '"') {
      inQuotes = !inQuotes;
    } else if (char === delimiter && !inQuotes) {
      values.push(current.trim());
      current = '';
    } else {
      current += char;
    }
  }
  values.push(current.trim());

  return values;
}

/**
 * Parse a recorded log. Returns empty `data` when no header or no readable
 * rows could be found, which callers should surface to the user rather than
 * treating as an empty log.
 */
export function parseLogFile(content: string): ParsedLog {
  const lines = content.trim().split('\n');
  if (lines.length < 2) return { data: [], channels: [] };

  // Locate the header and infer the delimiter together: the first line that
  // splits into multiple fields and contains a time column wins. Tab is tried
  // first so a `.msl` whose preamble happens to contain a comma is not
  // mistaken for CSV.
  let headerIdx = -1;
  let delimiter = ',';
  outer: for (let i = 0; i < Math.min(lines.length, MAX_HEADER_SCAN_LINES); i++) {
    for (const candidate of ['\t', ',']) {
      const fields = splitFields(lines[i], candidate);
      if (fields.length > 1 && fields.some(isTimeColumn)) {
        headerIdx = i;
        delimiter = candidate;
        break outer;
      }
    }
  }

  // No time column anywhere: fall back to treating line 0 as a CSV header so
  // headerless-but-tabular logs still load, with synthesised timestamps.
  if (headerIdx === -1) {
    headerIdx = 0;
    delimiter = lines[0].includes('\t') ? '\t' : ',';
  }

  const headers = splitFields(lines[headerIdx], delimiter);
  const timeColIndex = headers.findIndex(isTimeColumn);

  // `Time` is in seconds (TunerStudio); `timestamp_ms` is already milliseconds.
  const timeIsSeconds =
    timeColIndex >= 0 && headers[timeColIndex].toLowerCase() === 'time';

  // `.msl` follows the header with a units row (`s`, `rpm`, `kpa`, blank for
  // bitfields). Detect it by the time column failing to parse as a number,
  // which no real data row does.
  let firstDataIdx = headerIdx + 1;
  if (timeColIndex >= 0 && firstDataIdx < lines.length) {
    const next = splitFields(lines[firstDataIdx], delimiter);
    if (next.length > 1 && Number.isNaN(parseFloat(next[timeColIndex]))) {
      firstDataIdx++;
    }
  }

  const channels = headers.filter((_, i) => i !== timeColIndex);
  const data: LogSample[] = [];

  for (let i = firstDataIdx; i < lines.length; i++) {
    const line = lines[i].trim();
    if (!line) continue;

    const values = splitRow(line, delimiter);
    if (values.length < headers.length) continue;

    let timestamp: number;
    if (timeColIndex >= 0) {
      timestamp = parseFloat(values[timeColIndex]);
      if (timeIsSeconds) timestamp *= 1000;
    } else {
      timestamp = (i - firstDataIdx) * FALLBACK_SAMPLE_INTERVAL_MS;
    }
    if (Number.isNaN(timestamp)) continue;

    const entry: Record<string, number> = {};
    let channelIdx = 0;
    for (let j = 0; j < headers.length; j++) {
      if (j === timeColIndex) continue;
      // Enum channels log `Off`/`On` in numeric columns; skip just that field.
      const val = parseFloat(values[j]);
      if (!Number.isNaN(val)) entry[channels[channelIdx]] = val;
      channelIdx++;
    }

    // A row that yielded no numeric field is not a sample — this is what
    // arbitrary text degrades into, and emitting it would make an unreadable
    // file look like a log with content.
    if (Object.keys(entry).length === 0) continue;

    data.push({ x: timestamp, values: entry });
  }

  // Order by timestamp so playback and charting advance monotonically.
  //
  // Real exports are not reliably ordered. One observed TunerStudio `.msl`
  // (an export from a longer `.mlg`) carries a genuine first sample — 44
  // populated channels — stamped `0.000`, while every subsequent row uses an
  // offset near -11923s relative to the original recording. That single row is
  // real data, so it is kept rather than discarded on a timestamp heuristic;
  // sorting simply stops it from making the series jump backwards.
  //
  // Consumers must therefore derive duration from the min/max of `x` and
  // tolerate a large gap between the first and second samples. An outlier like
  // this makes a nominal-18.7-minute log span ~199 minutes end to end.
  data.sort((a, b) => a.x - b.x);

  return { data, channels };
}
