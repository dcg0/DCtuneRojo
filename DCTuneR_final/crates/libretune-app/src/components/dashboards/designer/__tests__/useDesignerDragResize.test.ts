import { act } from 'react';
import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useDesignerDragResize } from '../useDesignerDragResize';
import type { DashComponent, DashFile } from '../../dashTypes';

// Minimal gauge fixture: only the fields the drag/resize hook actually reads
// (id, relative_x/y/width/height) need real values. Cast covers the rest of
// TsGaugeConfig's many display/appearance fields, which this hook never
// touches — it only ever spreads `...c.Gauge`.
function makeGauge(overrides: Record<string, unknown> = {}): DashComponent {
  return {
    Gauge: {
      id: 'g1',
      relative_x: 0.3,
      relative_y: 0.3,
      relative_width: 0.2,
      relative_height: 0.2,
      ...overrides,
    },
  } as unknown as DashComponent;
}

function makeDashFile(component: DashComponent): DashFile {
  return {
    bibliography: {},
    version_info: {},
    gauge_cluster: {
      components: [component],
    },
  } as unknown as DashFile;
}

function makeContainerRef(width: number, height: number) {
  return {
    current: {
      getBoundingClientRect: () => ({
        width,
        height,
        top: 0,
        left: 0,
        right: width,
        bottom: height,
        x: 0,
        y: 0,
        toJSON() {
          return this;
        },
      }),
    },
  } as unknown as React.RefObject<HTMLDivElement>;
}

describe('useDesignerDragResize', () => {
  it('keeps the right edge fixed when a west-handle drag shrinks past the minimum size', () => {
    // Regression test: startRelativeX=0.3, startWidth=0.2, so the right edge
    // (the fixed point for a west-handle resize) is at 0.5. Before the fix,
    // dragging the west handle far enough right that width would clamp to
    // minSize (0.05) still moved newX from the raw unclamped delta, so the
    // right edge drifted from 0.5 to 0.60+ instead of staying put.
    const gauge = makeGauge();
    const dashFile = makeDashFile(gauge);
    const onDashFileChange = vi.fn();
    const containerRef = makeContainerRef(1000, 1000);

    const { result } = renderHook(() =>
      useDesignerDragResize({
        dashFile,
        containerRef,
        snapToGrid: (v) => v,
        pushHistory: vi.fn(),
        onDashFileChange,
        onSelectGauge: vi.fn(),
      }),
    );

    act(() => {
      result.current.onResizeMouseDown(
        {
          clientX: 500,
          clientY: 500,
          preventDefault: () => {},
          stopPropagation: () => {},
        } as unknown as React.MouseEvent,
        'w',
        'g1',
        gauge,
      );
    });

    // deltaX = (750 - 500) / 1000 = 0.25 → past the point where
    // startWidth (0.2) - deltaX would go negative.
    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 750, clientY: 500 }));
    });

    expect(onDashFileChange).toHaveBeenCalled();
    const lastCall = onDashFileChange.mock.calls[onDashFileChange.mock.calls.length - 1][0] as DashFile;
    const updatedGauge = lastCall.gauge_cluster.components[0] as { Gauge: { relative_x: number; relative_width: number } };

    expect(updatedGauge.Gauge.relative_width).toBeCloseTo(0.05, 5);
    // Right edge must stay at the original 0.3 + 0.2 = 0.5, not drift.
    expect(updatedGauge.Gauge.relative_x + updatedGauge.Gauge.relative_width).toBeCloseTo(0.5, 5);
  });

  it('keeps the bottom edge fixed when a north-handle drag shrinks past the minimum size', () => {
    const gauge = makeGauge();
    const dashFile = makeDashFile(gauge);
    const onDashFileChange = vi.fn();
    const containerRef = makeContainerRef(1000, 1000);

    const { result } = renderHook(() =>
      useDesignerDragResize({
        dashFile,
        containerRef,
        snapToGrid: (v) => v,
        pushHistory: vi.fn(),
        onDashFileChange,
        onSelectGauge: vi.fn(),
      }),
    );

    act(() => {
      result.current.onResizeMouseDown(
        {
          clientX: 500,
          clientY: 500,
          preventDefault: () => {},
          stopPropagation: () => {},
        } as unknown as React.MouseEvent,
        'n',
        'g1',
        gauge,
      );
    });

    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 500, clientY: 750 }));
    });

    const lastCall = onDashFileChange.mock.calls[onDashFileChange.mock.calls.length - 1][0] as DashFile;
    const updatedGauge = lastCall.gauge_cluster.components[0] as { Gauge: { relative_y: number; relative_height: number } };

    expect(updatedGauge.Gauge.relative_height).toBeCloseTo(0.05, 5);
    expect(updatedGauge.Gauge.relative_y + updatedGauge.Gauge.relative_height).toBeCloseTo(0.5, 5);
  });

  it('east-handle resize is unaffected (no x movement, sanity check)', () => {
    const gauge = makeGauge();
    const dashFile = makeDashFile(gauge);
    const onDashFileChange = vi.fn();
    const containerRef = makeContainerRef(1000, 1000);

    const { result } = renderHook(() =>
      useDesignerDragResize({
        dashFile,
        containerRef,
        snapToGrid: (v) => v,
        pushHistory: vi.fn(),
        onDashFileChange,
        onSelectGauge: vi.fn(),
      }),
    );

    act(() => {
      result.current.onResizeMouseDown(
        {
          clientX: 500,
          clientY: 500,
          preventDefault: () => {},
          stopPropagation: () => {},
        } as unknown as React.MouseEvent,
        'e',
        'g1',
        gauge,
      );
    });

    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 250, clientY: 500 }));
    });

    const lastCall = onDashFileChange.mock.calls[onDashFileChange.mock.calls.length - 1][0] as DashFile;
    const updatedGauge = lastCall.gauge_cluster.components[0] as { Gauge: { relative_x: number; relative_width: number } };

    expect(updatedGauge.Gauge.relative_width).toBeCloseTo(0.05, 5);
    // Left edge (x) must stay exactly where it started for an east drag.
    expect(updatedGauge.Gauge.relative_x).toBeCloseTo(0.3, 5);
  });
});
