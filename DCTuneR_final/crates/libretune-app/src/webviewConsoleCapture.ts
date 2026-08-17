// Forward webview console errors/warnings and uncaught errors to the Rust
// log via the `log_webview_message` command.
//
// The session logs capture Rust `tracing` only, so webview `console.error`,
// uncaught exceptions and unhandled promise rejections are otherwise invisible
// — the blind spot that made frontend-side failures like the Engine Constants
// dialog silently not dispatching `update_constant` (D6) impossible to
// diagnose from a full-debug session. This keeps the original console
// behaviour intact and only *also* forwards to the backend.

import { invoke } from "@tauri-apps/api/core";

type Level = "error" | "warn" | "log" | "info" | "debug";

function formatArg(arg: unknown): string {
  if (arg instanceof Error) {
    return arg.stack ? `${arg.message}\n${arg.stack}` : arg.message;
  }
  if (typeof arg === "string") return arg;
  try {
    return JSON.stringify(arg);
  } catch {
    return String(arg);
  }
}

function forward(level: Level, message: string): void {
  // Fire-and-forget; never let logging failures affect the app, and never
  // recurse back into the patched console on error.
  invoke("log_webview_message", { level, message }).catch(() => {});
}

let installed = false;

/**
 * Patch console.error/warn and install global error handlers so webview
 * diagnostics reach the Rust log. Idempotent; safe to call once at startup.
 */
export function installWebviewConsoleCapture(): void {
  if (installed) return;
  installed = true;

  for (const level of ["error", "warn"] as const) {
    const original = console[level].bind(console);
    console[level] = (...args: unknown[]) => {
      original(...args);
      forward(level, args.map(formatArg).join(" "));
    };
  }

  window.addEventListener("error", (e) => {
    const where = e.filename ? ` (${e.filename}:${e.lineno}:${e.colno})` : "";
    forward("error", `Uncaught: ${e.message}${where}`);
  });

  window.addEventListener("unhandledrejection", (e) => {
    forward("error", `Unhandled promise rejection: ${formatArg(e.reason)}`);
  });
}
