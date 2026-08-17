import { describe, expect, it } from 'vitest';
import { clampDropPosition } from '../useDesignerDrop';

describe('clampDropPosition', () => {
  it('centers a drop in the middle of the canvas without hitting either bound', () => {
    const { relX, relY } = clampDropPosition(0.5, 0.5);
    expect(relX).toBeCloseTo(0.4, 5);
    expect(relY).toBeCloseTo(0.4, 5);
  });

  it('keeps the gauge fully on-canvas when dropped at the right/bottom edge', () => {
    // Regression test: a drop right at the canvas edge (raw relX/relY = 1)
    // previously clamped the near edge to 0.9 regardless of the gauge's
    // 0.2 width/height, leaving the far edge at 1.1 -- 10% off-canvas.
    const { relX, relY } = clampDropPosition(1, 1);
    const DEFAULT_DROP_SIZE = 0.2;
    expect(relX).toBeCloseTo(1 - DEFAULT_DROP_SIZE, 5);
    expect(relY).toBeCloseTo(1 - DEFAULT_DROP_SIZE, 5);
    // The far edge must not exceed the canvas.
    expect(relX + DEFAULT_DROP_SIZE).toBeLessThanOrEqual(1);
    expect(relY + DEFAULT_DROP_SIZE).toBeLessThanOrEqual(1);
  });

  it('keeps the gauge fully on-canvas when dropped at the left/top edge', () => {
    const { relX, relY } = clampDropPosition(0, 0);
    expect(relX).toBe(0);
    expect(relY).toBe(0);
  });

  it('never produces a negative position for a drop just inside the edge', () => {
    const { relX, relY } = clampDropPosition(0.05, 0.05);
    expect(relX).toBeGreaterThanOrEqual(0);
    expect(relY).toBeGreaterThanOrEqual(0);
  });
});
