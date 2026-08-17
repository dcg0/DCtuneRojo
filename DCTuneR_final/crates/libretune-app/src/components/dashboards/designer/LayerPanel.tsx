/**
 * LayerPanel — Plan v2 / D-7b.
 *
 * Lists every component on the dashboard and lets the user:
 *  - select/focus a component (single-click)
 *  - reorder z-stack (▲/▼ buttons; later index = drawn on top)
 *  - toggle visibility via `enabled_condition`
 *  - delete
 *
 * Z-ordering uses the array order in `gauge_cluster.components` since
 * the dashboard renders strictly in that sequence.
 *
 * `enabled_condition` is also a real, user-facing field (set via
 * PropertyEditor's "Enabled Condition" input, e.g. "hasLambdaSensor" or
 * "rpm > 0") for conditional visibility at runtime. The Hide/Show toggle
 * here reuses that same field as a simple boolean by overwriting it to the
 * literal string "false" — so hiding a component that already had a real
 * condition must stash the original away first (round-tripped through
 * `extra_attrs`, same mechanism PropertyEditor's TrendSeriesEditor uses for
 * its own non-schema fields) and restore it on Show, instead of silently
 * destroying it.
 */

import { DashFile, DashComponent, isGauge, isIndicator } from '../dashTypes';

interface Props {
  dashFile: DashFile;
  selectedGaugeId: string | null;
  onSelect: (id: string | null) => void;
  onChange: (file: DashFile) => void;
}

function componentId(c: DashComponent): string {
  if (isGauge(c)) return c.Gauge.id;
  if (isIndicator(c)) return c.Indicator.id;
  return '';
}

function componentLabel(c: DashComponent): string {
  if (isGauge(c)) return c.Gauge.title || c.Gauge.output_channel || c.Gauge.id;
  if (isIndicator(c)) return c.Indicator.on_text || c.Indicator.output_channel || c.Indicator.id;
  return '?';
}

function componentKind(c: DashComponent): string {
  if (isGauge(c)) return c.Gauge.gauge_painter;
  if (isIndicator(c)) return c.Indicator.indicator_painter;
  return '';
}

const HIDDEN_MARKER = 'false';
const SAVED_CONDITION_KEY = 'lt_prev_enabled_condition';

function getCondition(c: DashComponent): string | null {
  if (isGauge(c)) return c.Gauge.enabled_condition ?? null;
  if (isIndicator(c)) return c.Indicator.enabled_condition ?? null;
  return null;
}

function getExtraAttrs(c: DashComponent): Record<string, string> {
  if (isGauge(c)) return c.Gauge.extra_attrs ?? {};
  if (isIndicator(c)) return c.Indicator.extra_attrs ?? {};
  return {};
}

export function isHidden(c: DashComponent): boolean {
  return getCondition(c)?.trim().toLowerCase() === HIDDEN_MARKER;
}

export function withHidden(c: DashComponent, hidden: boolean): DashComponent {
  const currentCondition = getCondition(c);
  const attrs = { ...getExtraAttrs(c) };

  let nextCondition: string | null;
  if (hidden) {
    // Stash whatever real condition (if any) was set before overwriting it
    // with the hide marker, so Show can restore it exactly instead of
    // always resetting to "always visible". Skip this if it's already the
    // hide marker (already hidden -- nothing new to preserve).
    if (currentCondition?.trim().toLowerCase() !== HIDDEN_MARKER) {
      if (currentCondition) {
        attrs[SAVED_CONDITION_KEY] = currentCondition;
      } else {
        delete attrs[SAVED_CONDITION_KEY];
      }
    }
    nextCondition = HIDDEN_MARKER;
  } else {
    nextCondition = attrs[SAVED_CONDITION_KEY] ?? null;
    delete attrs[SAVED_CONDITION_KEY];
  }

  if (isGauge(c)) return { Gauge: { ...c.Gauge, enabled_condition: nextCondition, extra_attrs: attrs } };
  if (isIndicator(c)) return { Indicator: { ...c.Indicator, enabled_condition: nextCondition, extra_attrs: attrs } };
  return c;
}

export default function LayerPanel({ dashFile, selectedGaugeId, onSelect, onChange }: Props) {
  const components = dashFile.gauge_cluster.components;

  const replaceComponents = (next: DashComponent[]) => {
    onChange({
      ...dashFile,
      gauge_cluster: { ...dashFile.gauge_cluster, components: next },
    });
  };

  const move = (from: number, to: number) => {
    if (to < 0 || to >= components.length || from === to) return;
    const next = [...components];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    replaceComponents(next);
  };

  const toggleHidden = (i: number) => {
    const next = [...components];
    next[i] = withHidden(next[i], !isHidden(next[i]));
    replaceComponents(next);
  };

  const remove = (i: number) => {
    const next = [...components];
    next.splice(i, 1);
    replaceComponents(next);
    if (selectedGaugeId && componentId(components[i]) === selectedGaugeId) {
      onSelect(null);
    }
  };

  return (
    <div className="layer-panel">
      <h4>Layers</h4>
      {components.length === 0 ? (
        <p className="no-selection">No components yet</p>
      ) : (
        <ul className="layer-list">
          {/* Render top-down (last item = top of z-stack). */}
          {components.map((_c, i) => i).reverse().map((i) => {
            const c = components[i];
            const id = componentId(c);
            const isSel = id === selectedGaugeId;
            const hidden = isHidden(c);
            return (
              <li
                key={id || `idx-${i}`}
                className={`layer-row ${isSel ? 'selected' : ''} ${hidden ? 'hidden' : ''}`}
                onClick={() => onSelect(id || null)}
                title={`${componentKind(c)} — index ${i}`}
              >
                <span className="layer-name">{componentLabel(c)}</span>
                <span className="layer-kind">{componentKind(c)}</span>
                <button
                  type="button"
                  title="Move up (toward top of stack)"
                  onClick={(e) => { e.stopPropagation(); move(i, i + 1); }}
                  disabled={i === components.length - 1}
                >▲</button>
                <button
                  type="button"
                  title="Move down (toward bottom of stack)"
                  onClick={(e) => { e.stopPropagation(); move(i, i - 1); }}
                  disabled={i === 0}
                >▼</button>
                <button
                  type="button"
                  title={hidden ? 'Show' : 'Hide'}
                  onClick={(e) => { e.stopPropagation(); toggleHidden(i); }}
                >{hidden ? '○' : '●'}</button>
                <button
                  type="button"
                  title="Delete"
                  onClick={(e) => { e.stopPropagation(); remove(i); }}
                >✕</button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
