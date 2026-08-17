import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(), save: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import { ToastProvider } from '../../../contexts/ToastContext';
import { AutoTune } from '../AutoTune';

const TABLE_DATA = {
  name: 'veTable1Tbl',
  title: 'VE Table',
  x_bins: [1000, 2000],
  y_bins: [20, 80],
  z_values: [
    [50, 60],
    [70, 80],
  ],
  x_output_channel: 'rpm',
  y_output_channel: 'map',
};

function mockInvoke() {
  (invoke as unknown as any).mockImplementation((cmd: string) => {
    if (cmd === 'get_ve_analyze_config') return Promise.resolve(null);
    if (cmd === 'get_tables') return Promise.resolve([{ name: 'veTable1Tbl', title: 'VE Table' }]);
    if (cmd === 'get_table_data') return Promise.resolve(TABLE_DATA);
    if (cmd === 'get_available_channels') return Promise.resolve([]);
    return Promise.resolve();
  });
}

// Regression test: AutoTune previously had no isConnected awareness at all
// (TabContentRouter.tsx never passed it, unlike every other connection-aware
// component). Clicking Start while disconnected silently called
// start_autotune anyway -- it "succeeded" but no live data ever streamed in,
// so nothing visible ever happened, matching GitHub issue #132 ("when I hit
// Start, nothing happens -- it is as if it isn't connected").
describe('AutoTune connection awareness', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mockInvoke();
  });

  it('disables Start and shows a DISCONNECTED badge when not connected', async () => {
    render(
      <ToastProvider>
        <AutoTune tableName="veTable1Tbl" isConnected={false} />
      </ToastProvider>
    );

    await waitFor(() => expect(screen.getByText('DISCONNECTED')).toBeInTheDocument());

    const startBtn = screen.getByRole('button', { name: /start/i });
    expect(startBtn).toBeDisabled();

    (invoke as unknown as any).mockClear();
    await userEvent.click(startBtn);
    expect(invoke).not.toHaveBeenCalledWith('start_autotune', expect.anything());
  });

  it('enables Start and calls start_autotune when connected', async () => {
    render(
      <ToastProvider>
        <AutoTune tableName="veTable1Tbl" isConnected={true} />
      </ToastProvider>
    );

    await waitFor(() => expect(screen.queryByText('DISCONNECTED')).not.toBeInTheDocument());

    const startBtn = screen.getByRole('button', { name: /start/i });
    expect(startBtn).not.toBeDisabled();

    await userEvent.click(startBtn);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('start_autotune', expect.objectContaining({ tableName: 'veTable1Tbl' }))
    );
  });
});

// The lambda delay is a measured per-engine fact (about 470 ms at idle on the
// reference NA6), not view state. It used to reset on every launch, and the
// default of 0 does not mean "no delay" - it means "fall back to the built-in
// RPM curve", which caps at 200 ms. A whole 59-minute drive was tuned against
// that fallback because the setting had silently reverted, inflating every
// low-load correction.
//
// Queried by row rather than by label: the settings rows render <label> and
// <input> as siblings with no htmlFor, so getByLabelText cannot resolve them.
describe('AutoTune settings persistence', () => {
  const delayInput = async () => {
    const label = await screen.findByText(/(Lambda|Idle) Delay \(ms\):/);
    const input = label.parentElement?.querySelector('input[type="number"]');
    if (!input) throw new Error('delay input not found next to its label');
    return input as HTMLInputElement;
  };

  beforeEach(() => {
    localStorage.clear();
    mockInvoke();
  });

  it('restores a measured lambda delay across a remount', async () => {
    const { unmount } = render(
      <ToastProvider><AutoTune isConnected onClose={() => {}} /></ToastProvider>,
    );

    const delay = await delayInput();
    await userEvent.clear(delay);
    await userEvent.type(delay, '470');
    await waitFor(() =>
      expect(localStorage.getItem('libretune.autotune.settings.v1.settings'))
        .toContain('470'),
    );

    unmount();
    render(<ToastProvider><AutoTune isConnected onClose={() => {}} /></ToastProvider>);
    expect((await delayInput()).value).toBe('470');
  });

  it('falls back to defaults when stored state is corrupt', async () => {
    localStorage.setItem('libretune.autotune.settings.v1.settings', '{not json');
    render(<ToastProvider><AutoTune isConnected onClose={() => {}} /></ToastProvider>);
    expect((await delayInput()).value).toBe('0');
  });
});
