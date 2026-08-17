import { invoke } from "@tauri-apps/api/core";
import type { ReactNode } from "react";
import type { TFunction } from "i18next";
import type { ToolbarItem } from "../components/tuner-ui";
import ConnectionMetrics from "../components/layout/ConnectionMetrics";
import type { ConnectionStatus, IniCapabilities } from "../types/app";

export interface BuildToolbarItemsDeps {
  /** i18n translation function bound to the `common` namespace. */
  t: TFunction;
  status: ConnectionStatus;
  tuneModified: boolean;
  iniCapabilities: IniCapabilities | null;
  isLogging: boolean;
  connectionRuntimePacketMode: string | null;
  defaultRuntimePacketMode: string;
  setLoadDialogOpen: (open: boolean) => void;
  setSaveDialogOpen: (open: boolean) => void;
  setBurnDialogOpen: (open: boolean) => void;
  setConnectionDialogOpen: (open: boolean) => void;
  setSettingsDialogOpen: (open: boolean) => void;
  setActiveTabId: (id: string) => void;
  setIsLogging: (logging: boolean) => void;
}

export function buildToolbarItems(deps: BuildToolbarItemsDeps): ToolbarItem[] {
  const {
    t, status, tuneModified, iniCapabilities, isLogging, connectionRuntimePacketMode, defaultRuntimePacketMode,
    setLoadDialogOpen, setSaveDialogOpen, setBurnDialogOpen, setConnectionDialogOpen,
    setSettingsDialogOpen, setActiveTabId, setIsLogging,
  } = deps;

  const connected = status.state === 'Connected';
  const canBurn = connected && tuneModified;

  const items: ToolbarItem[] = [
    { id: "open", icon: "open", tooltip: t('toolbar.openTune'), onClick: () => setLoadDialogOpen(true) },
    { id: "save", icon: "save", tooltip: t('toolbar.saveTune'), onClick: () => setSaveDialogOpen(true), disabled: !status.has_definition },
    {
      id: "burn",
      icon: "burn",
      tooltip: !connected
        ? t('toolbar.burnDisconnected')
        : tuneModified
          ? t('toolbar.burnPending')
          : t('toolbar.burnNone'),
      onClick: () => setBurnDialogOpen(true),
      disabled: !canBurn,
      variant: canBurn ? 'burn-pending' : undefined,
    },
    { id: "sep1", icon: "", tooltip: "", separator: true },
    {
      id: "connect",
      icon: status.state === "Connected" ? "disconnect" : "connect",
      tooltip: status.state === "Connected" ? t('toolbar.disconnect') : t('toolbar.connect'),
      active: status.state === "Connected",
      onClick: () => setConnectionDialogOpen(true),
    },
    {
      id: 'connection-info',
      icon: 'connection-info',
      tooltip: t('toolbar.connectionInfo'),
      content: (
        <div className="toolbar-connection-info">
          <ConnectionMetrics compact />
          <span className="packet-mode">{status.state === 'Connected' ? (connectionRuntimePacketMode || defaultRuntimePacketMode) : '—'}</span>
        </div>
      ) as ReactNode,
    },
  ];

  if (iniCapabilities?.has_frontpage || iniCapabilities?.has_gauges) {
    items.push({ id: "realtime", icon: "realtime", tooltip: t('toolbar.realtime'), onClick: () => setActiveTabId("dashboard") });
  }

  if (iniCapabilities?.has_datalog_entries || iniCapabilities?.has_output_channels) {
    items.push(
      { id: "sep2", icon: "", tooltip: "", separator: true },
      {
        id: "log-start",
        icon: isLogging ? "log-stop" : "log-start",
        tooltip: isLogging ? t('toolbar.stopLogging') : t('toolbar.startLogging'),
        active: isLogging,
        onClick: async () => {
          try {
            if (isLogging) {
              await invoke('stop_logging');
              setIsLogging(false);
            } else {
              await invoke('start_logging', { sampleRate: 10 });
              setIsLogging(true);
            }
          } catch (err) {
            console.error('Logging toggle failed:', err);
          }
        },
      }
    );
  }

  items.push(
    { id: "sep3", icon: "", tooltip: "", separator: true },
    { id: "settings", icon: "settings", tooltip: t('toolbar.settings'), onClick: () => setSettingsDialogOpen(true) }
  );

  return items;
}
