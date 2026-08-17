import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { vi } from "vitest";

// Mock Tauri APIs before importing the component under test, matching the
// pattern used by App.integration.test.tsx.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { PluginPanel } from "../PluginPanel";
import { ToastProvider } from "../../contexts/ToastContext";

/**
 * Manual verification (2026-07-31) that the WASM plugin load -> consent ->
 * execute -> result-summary flow shipped in
 * fix/wasm-plugin-host-api-wiring-and-consent and
 * feat/wasm-plugin-real-data-snapshot actually renders and behaves as
 * intended. This repo has no chromium-cli/Playwright/tauri-driver set up to
 * click through the real native (WebView2) window, so this drives the real
 * component tree with React Testing Library and a stateful Tauri `invoke`
 * mock standing in for the Rust backend — genuine render+click+assert
 * against the DOM, not a unit test of an internal function.
 */
describe("PluginPanel manual-flow verification", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it("shows an unchecked-by-default consent dialog, grants only what's checked, and shows the real execution result", async () => {
    // Stateful backend stand-in: tracks whether a plugin has been "loaded"
    // and what permissions it was actually granted, so list_wasm_plugins'
    // response changes across the flow exactly like the real Tauri command
    // would.
    let loadedPlugin: any = null;
    let lastLoadCallArgs: any = null;

    (invoke as unknown as any).mockImplementation((cmd: string, args: any) => {
      switch (cmd) {
        case "list_wasm_plugins":
          return Promise.resolve(loadedPlugin ? [loadedPlugin] : []);
        case "load_wasm_plugin": {
          lastLoadCallArgs = args;
          const manifest = JSON.parse(args.manifestJson);
          loadedPlugin = {
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            author: manifest.author,
            state: "Ready",
            permissions: args.approvedPermissions,
            exec_count: 0,
          };
          return Promise.resolve(manifest.name);
        }
        case "execute_wasm_plugin":
          loadedPlugin = { ...loadedPlugin, exec_count: loadedPlugin.exec_count + 1 };
          return Promise.resolve({
            exec_count: loadedPlugin.exec_count,
            result_code: 0,
            applied_constants: [{ name: "rpmMin", value: 1234.5 }],
            unapplied_actions: [],
          });
        default:
          return Promise.resolve();
      }
    });

    (open as unknown as any).mockResolvedValue("C:\\fake\\test_plugin.wasm");

    render(
      <ToastProvider>
        <PluginPanel isConnected={true} />
      </ToastProvider>
    );

    // Empty state on mount.
    await screen.findByText("No plugins loaded");

    // Open the file picker -> consent dialog appears.
    fireEvent.click(screen.getByText("+ Load Plugin"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText('Grant permissions to "test_plugin"?')).toBeInTheDocument();

    // All four permission checkboxes must default to UNCHECKED — nothing
    // silently pre-granted.
    const checkboxes = within(dialog).getAllByRole("checkbox") as HTMLInputElement[];
    expect(checkboxes).toHaveLength(4);
    checkboxes.forEach((cb) => expect(cb.checked).toBe(false));

    // Approve only "Write Constants".
    fireEvent.click(within(dialog).getByText("Write Constants"));
    expect((within(dialog).getByText("Write Constants").closest("label") as HTMLElement).querySelector("input")).toBeChecked();

    // Confirm the load.
    fireEvent.click(within(dialog).getByText("Load Plugin"));

    await waitFor(() => expect(lastLoadCallArgs).not.toBeNull());
    // Only the checked permission was sent as approved, regardless of what
    // the (self-authored) manifest itself listed.
    expect(lastLoadCallArgs.approvedPermissions).toEqual(["WriteConstants"]);

    // Consent dialog closes; plugin now shows in the list.
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    const card = await screen.findByText("test_plugin");
    fireEvent.click(card);

    // Selecting the plugin shows its granted-permission count (1, not 4).
    await screen.findByText("Permissions (1)");

    // Execute it and confirm the real (mocked-backend) result renders.
    fireEvent.click(screen.getByText("Execute"));
    await screen.findByText("Last Run Result");
    const resultItem = document.querySelector(".plugin-permission-item");
    expect(resultItem?.textContent).toBe("Set rpmMin = 1234.5");
  });

  it("shows a visible error toast when the backend rejects the file, instead of failing silently", async () => {
    // Regression test: manual testing surfaced that loading an invalid file
    // (e.g. a .txt renamed to .wasm — real wasmtime Module::new() correctly
    // rejects it) closed the consent dialog with zero on-screen feedback,
    // only a console.error. Confirms load_wasm_plugin failures now produce
    // a visible toast.
    (invoke as unknown as any).mockImplementation((cmd: string) => {
      if (cmd === "list_wasm_plugins") return Promise.resolve([]);
      if (cmd === "load_wasm_plugin") {
        return Promise.reject("Failed to load WASM module: invalid wasm magic number");
      }
      return Promise.resolve();
    });
    (open as unknown as any).mockResolvedValue("C:\\fake\\not_really_wasm.wasm");

    render(
      <ToastProvider>
        <PluginPanel isConnected={true} />
      </ToastProvider>
    );

    await screen.findByText("No plugins loaded");
    fireEvent.click(screen.getByText("+ Load Plugin"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByText("Load Plugin"));

    // Dialog closes either way (matches the existing finally-block
    // behavior), but now a visible error toast explains why nothing loaded.
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await screen.findByText(/Failed to load plugin/i);
    expect(screen.getByText("No plugins loaded")).toBeInTheDocument();
  });
});
