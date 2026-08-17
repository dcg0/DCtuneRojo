import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import TableEditor2D from '../TableEditor2D';
import { ToastProvider } from '../../../contexts/ToastContext';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

const X_BINS = [500, 1000, 1500];
const Y_BINS = [20, 40, 60];
const Z_VALUES = [
  [10, 11, 12],
  [20, 21, 22],
  [30, 31, 32],
];

/** Tauri 2 converts camelCase invoke keys to snake_case Rust params; sending
 *  snake_case makes the command reject the payload before it ever runs. */
const TABLE_OP_COMMANDS = new Set([
  'set_cells_equal',
  'scale_cells',
  'smooth_table',
  'interpolate_cells',
  'interpolate_linear',
  'add_offset',
  'fill_region',
  'rebin_table',
]);

function renderEditor() {
  const result = render(
    <ToastProvider>
      <TableEditor2D
        title="VE Table 1"
        table_name="veTable1Tbl"
        x_axis_name="RPM"
        y_axis_name="MAP"
        x_bins={X_BINS}
        y_bins={Y_BINS}
        z_values={Z_VALUES}
      />
    </ToastProvider>
  );

  // Operations no-op without a selection, so select the first cell.
  const cells = result.container.querySelectorAll('.table-cell');
  fireEvent.mouseDown(cells[0]);

  return result;
}

function lastTableOpCall() {
  const call = [...invokeMock.mock.calls].reverse().find(([cmd]) => TABLE_OP_COMMANDS.has(cmd as string));
  if (!call) throw new Error('no table operation was invoked');
  return { command: call[0] as string, args: (call[1] ?? {}) as Record<string, unknown> };
}

describe('TableEditor2D table operations', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({
      table_name: 'veTable1Tbl',
      x_bins: X_BINS,
      y_bins: Y_BINS,
      z_values: Z_VALUES,
    });
  });

  it('sends camelCase args for smooth', async () => {
    renderEditor();
    fireEvent.click(screen.getByTitle('Smooth selected cells (s)'));

    await waitFor(() => expect(lastTableOpCall().command).toBe('smooth_table'));
    expect(lastTableOpCall().args).toEqual({
      tableName: 'veTable1Tbl',
      selectedCells: [[0, 0]],
      factor: 1.0,
    });
  });

  it('sends camelCase args for interpolate', async () => {
    renderEditor();
    fireEvent.click(screen.getByTitle('Interpolate between corners (/)'));

    await waitFor(() => expect(lastTableOpCall().command).toBe('interpolate_cells'));
    expect(lastTableOpCall().args).toEqual({
      tableName: 'veTable1Tbl',
      selectedCells: [[0, 0]],
    });
  });

  it('sends camelCase args for set equal', async () => {
    renderEditor();
    fireEvent.click(screen.getByTitle('Set Equal (=) - Set selected cells to average'));

    await waitFor(() => expect(lastTableOpCall().command).toBe('set_cells_equal'));
    expect(lastTableOpCall().args).toEqual({
      tableName: 'veTable1Tbl',
      selectedCells: [[0, 0]],
      value: Z_VALUES[0][0],
    });
  });

  it('prompts for a scale factor and sends camelCase args', async () => {
    renderEditor();
    fireEvent.click(screen.getByTitle('Scale selected cells (*)'));

    const input = await screen.findByLabelText('Multiplier');
    fireEvent.change(input, { target: { value: '1.1' } });
    fireEvent.click(screen.getByText('Apply'));

    await waitFor(() => expect(lastTableOpCall().command).toBe('scale_cells'));
    expect(lastTableOpCall().args).toEqual({
      tableName: 'veTable1Tbl',
      selectedCells: [[0, 0]],
      scaleFactor: 1.1,
    });
  });

  it('nudges by a percentage rather than multiplying', () => {
    const { container } = renderEditor();
    fireEvent.click(screen.getByTitle('Increase by 1% (> or .)'));

    const firstCell = container.querySelector('.table-cell .cell-value');
    expect(firstCell?.textContent).toBe((Z_VALUES[0][0] * 1.01).toFixed(1));
  });

  it('resolves the right-clicked cell instead of defaulting to (0, 0)', async () => {
    const { container } = renderEditor();

    // Right-click the value span (the real event target) of cell x=1, y=1.
    const cell = container.querySelectorAll('.table-cell')[4];
    fireEvent.contextMenu(cell.querySelector('.cell-value')!);

    expect(screen.getByText('Cell [1, 1]')).toBeInTheDocument();
    expect(screen.getByText(`Val: ${Z_VALUES[1][1].toFixed(2)}`)).toBeInTheDocument();

    // The menu acts on the selection, so it must have moved to the clicked cell.
    fireEvent.click(screen.getByText('Set Equal'));
    await waitFor(() => expect(lastTableOpCall().command).toBe('set_cells_equal'));
    expect(lastTableOpCall().args).toEqual({
      tableName: 'veTable1Tbl',
      selectedCells: [[1, 1]],
      value: Z_VALUES[1][1],
    });
  });
});
