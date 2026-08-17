/**
 * Format an error from a Tauri command into a user-friendly message + details.
 * Detects common Rust panic patterns and parse errors.
 */
export function formatError(e: unknown): { message: string; details: string } {
  const errorStr = String(e);
  // Check for panic messages (Rust panics often contain "panicked" or stack traces)
  if (errorStr.includes("panicked") || errorStr.includes("overflow") || errorStr.includes("thread")) {
    return {
      message: "An internal error occurred while processing the tune file. This may indicate an incompatibility between the INI definition and the tune file.",
      details: errorStr,
    };
  }
  // Check for parse errors
  if (errorStr.includes("parse") || errorStr.includes("Parse") || errorStr.includes("invalid")) {
    return {
      message: "The tune file could not be parsed. It may be corrupted or use an unsupported format.",
      details: errorStr,
    };
  }
  // Default error format
  return {
    message: errorStr,
    details: "",
  };
}
