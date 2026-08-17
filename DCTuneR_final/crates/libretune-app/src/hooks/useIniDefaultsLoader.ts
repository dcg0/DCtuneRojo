import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ProtocolDefaults,
  IniCapabilities,
  ChannelInfo,
  ConnectionStatus,
} from "../types/app";

export interface UseIniDefaultsLoaderDeps {
  status: ConnectionStatus;
  baudUserSet: boolean;
  baudRate: number;
  setBaudRate: (n: number) => void;
  timeoutUserSet: boolean;
  timeoutMs: number;
  setTimeoutMs: (n: number) => void;
  setIniDefaults: (p: ProtocolDefaults | null) => void;
  setStatusBarChannels: (c: string[]) => void;
  setChannelInfoMap: (m: Record<string, ChannelInfo>) => void;
  setIniCapabilities: (c: IniCapabilities | null) => void;
}

/**
 * Loads INI-derived protocol defaults, status-bar channels, channel metadata,
 * and capability flags whenever the backend reports a new definition is loaded.
 * Auto-applies the default baud rate / timeout only if the user has not yet
 * customised them.
 */
export function useIniDefaultsLoader(deps: UseIniDefaultsLoaderDeps): void {
  const {
    status,
    baudUserSet,
    baudRate,
    setBaudRate,
    timeoutUserSet,
    timeoutMs,
    setTimeoutMs,
    setIniDefaults,
    setStatusBarChannels,
    setChannelInfoMap,
    setIniCapabilities,
  } = deps;

  useEffect(() => {
    if (!status.has_definition) {
      setIniCapabilities(null);
      return;
    }
    const inTauri = !!(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    if (!inTauri) return;

    (async () => {
      try {
        const proto = await invoke<ProtocolDefaults>("get_protocol_defaults");
        setIniDefaults(proto);

        if (!baudUserSet && baudRate === 115200 && proto.default_baud_rate && proto.default_baud_rate !== 0) {
          setBaudRate(proto.default_baud_rate);
        }
        if (!timeoutUserSet && timeoutMs === 2000 && proto.timeout_ms && proto.timeout_ms !== 0) {
          setTimeoutMs(proto.timeout_ms);
        }
      } catch (e) {
        console.warn("get_protocol_defaults failed:", e);
      }

      try {
        const defaults = await invoke<string[]>("get_status_bar_defaults");
        if (defaults && defaults.length > 0) {
          setStatusBarChannels(defaults);
        }
      } catch (e) {
        console.warn("get_status_bar_defaults failed:", e);
      }

      try {
        const channels = await invoke<ChannelInfo[]>("get_available_channels");
        const map: Record<string, ChannelInfo> = {};
        channels.forEach((ch) => {
          map[ch.name] = ch;
        });
        setChannelInfoMap(map);
      } catch (e) {
        console.warn("get_available_channels failed:", e);
      }

      try {
        const caps = await invoke<IniCapabilities>("get_ini_capabilities");
        setIniCapabilities(caps);
      } catch (e) {
        console.warn("get_ini_capabilities failed:", e);
        setIniCapabilities(null);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status.has_definition]);
}
