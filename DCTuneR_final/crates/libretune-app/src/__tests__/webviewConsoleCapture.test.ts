// Verifies the D6 diagnostics bridge: webview console errors/warnings are
// forwarded to the Rust log via `log_webview_message`, WITHOUT suppressing the
// original console output (so the browser devtools still show them too).
import { describe, it, expect, vi, beforeAll, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(() => Promise.resolve()) }));

import { installWebviewConsoleCapture } from '../webviewConsoleCapture';

const invokeMock = vi.mocked(invoke);
// Stub the console BEFORE install so the capture wraps these (and we can assert
// the original was still called). install() is idempotent and patches the
// module-global console once, so this ordering matters.
const errStub = vi.fn();
const warnStub = vi.fn();

beforeAll(() => {
  console.error = errStub;
  console.warn = warnStub;
  installWebviewConsoleCapture();
});
beforeEach(() => {
  invokeMock.mockClear();
  errStub.mockClear();
  warnStub.mockClear();
});

describe('webviewConsoleCapture', () => {
  it('forwards console.error to log_webview_message and still calls the original', () => {
    console.error('boom', 42);
    expect(errStub).toHaveBeenCalled(); // output not swallowed
    expect(invokeMock).toHaveBeenCalledWith('log_webview_message', {
      level: 'error',
      message: expect.stringContaining('boom'),
    });
  });

  it('forwards console.warn at warn level', () => {
    console.warn('heads up');
    expect(warnStub).toHaveBeenCalled();
    expect(invokeMock).toHaveBeenCalledWith('log_webview_message', {
      level: 'warn',
      message: expect.stringContaining('heads up'),
    });
  });
});
