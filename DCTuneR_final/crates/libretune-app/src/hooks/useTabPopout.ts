import { useCallback, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Tab } from "../components/tuner-ui";
import type { TabContent } from "../types/app";
import type { ToastType } from "../contexts/ToastContext";

export interface UseTabPopoutDeps {
  tabs: Tab[];
  tabContents: Record<string, TabContent>;
  handleTabClose: (tabId: string) => void;
  showToast: (msg: string, type?: ToastType, duration?: number) => void;
  setTabs: React.Dispatch<React.SetStateAction<Tab[]>>;
  setTabContents: React.Dispatch<React.SetStateAction<Record<string, TabContent>>>;
  setActiveTabId: React.Dispatch<React.SetStateAction<string | null>>;
}

/**
 * Manages multi-monitor pop-out window support:
 * - handleTabPopout: opens a tab in a separate WebviewWindow
 * - tab:dock listener: re-adds a tab when the popout requests dock-back
 * - table:updated listener: syncs table edits from popout to main window
 */
export function useTabPopout(deps: UseTabPopoutDeps): {
  handleTabPopout: (tabId: string) => Promise<void>;
} {
  const {
    tabs,
    tabContents,
    handleTabClose,
    showToast,
    setTabs,
    setTabContents,
    setActiveTabId,
  } = deps;

  const handleTabPopout = useCallback(
    async (tabId: string) => {
      const content = tabContents[tabId];
      const tab = tabs.find((t) => t.id === tabId);
      if (!content || !tab) return;

      // Store data in localStorage for the pop-out window to retrieve
      const storageKey = `popout-${tabId}`;
      localStorage.setItem(storageKey, JSON.stringify({ data: content.data }));

      // Create the pop-out window
      const label = `popout-${tabId.replace(/[^a-zA-Z0-9]/g, "_")}`;

      // Build URL for pop-out window
      const currentOrigin = window.location.origin;
      const hashParams = `#/popout?tabId=${encodeURIComponent(tabId)}&type=${encodeURIComponent(
        content.type,
      )}&title=${encodeURIComponent(tab.title)}`;
      const url = `${currentOrigin}/${hashParams}`;

      console.log("[handleTabPopout] Creating window with URL:", url);

      try {
        const webview = new WebviewWindow(label, {
          url,
          title: tab.title,
          width: 900,
          height: 700,
          center: true,
          decorations: true,
          devtools: true,
        });

        await webview.once("tauri://created", () => {
          console.log("Pop-out window created:", label, "url:", url);
        });

        webview.once("tauri://error", (e) => {
          console.error("Pop-out window error:", e);
        });

        // Remove tab from main window
        handleTabClose(tabId);
      } catch (e) {
        console.error("Failed to create pop-out window:", e);
        showToast("Failed to pop out tab: " + e, "error");
        localStorage.removeItem(storageKey);
      }
    },
    [tabs, tabContents, handleTabClose, showToast],
  );

  // Listen for dock events from pop-out windows
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    (async () => {
      try {
        unlisten = await listen<{
          tabId: string;
          type: TabContent["type"];
          title: string;
          data: TabContent["data"];
        }>("tab:dock", (event) => {
          const { tabId, type, title, data } = event.payload;
          console.log("Tab docking back:", tabId);

          setTabs((prev) => {
            if (prev.find((t) => t.id === tabId)) return prev;
            return [
              ...prev,
              { id: tabId, title, icon: type === "table" || type === "curve" ? "table" : type },
            ];
          });
          setTabContents((prev) => ({
            ...prev,
            [tabId]: { type, data } as TabContent,
          }));
          setActiveTabId(tabId);
        });
      } catch (e) {
        console.error("Failed to listen for tab:dock events:", e);
      }
    })();
    return () => {
      if (unlisten) unlisten();
    };
  }, [setTabs, setTabContents, setActiveTabId]);

  // Listen for table updates from pop-out windows
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    (async () => {
      try {
        unlisten = await listen<{
          tabId: string;
          type: TabContent["type"];
          data: TabContent["data"];
        }>("table:updated", (event) => {
          const { tabId, type, data } = event.payload;
          setTabContents((prev) => {
            if (!prev[tabId]) return prev;
            return {
              ...prev,
              [tabId]: { type, data } as TabContent,
            };
          });
        });
      } catch (e) {
        console.error("Failed to listen for table:updated events:", e);
      }
    })();
    return () => {
      if (unlisten) unlisten();
    };
  }, [setTabContents]);

  return { handleTabPopout };
}
