import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { ContentList } from "./features/content-list/ContentList";
import { SettingsView } from "./features/settings/SettingsView";
import { ReportView } from "./features/weekly-report/ReportView";
import { useSettingsStore } from "./stores/settingsStore";
import { useContentStore } from "./stores/contentStore";
// FloatingBubble is now a separate system-level window (see BubbleView.tsx)

type TabId = "content" | "report" | "settings";

interface TabItem {
  id: TabId;
  label: string;
  icon: string;
}

const TABS: TabItem[] = [
  { id: "content", label: "内容", icon: "📋" },
  { id: "report", label: "周报", icon: "📊" },
  { id: "settings", label: "设置", icon: "⚙️" },
];

function App() {
  const [activeTab, setActiveTab] = useState<TabId>("content");
  const loadFromDB = useSettingsStore((s) => s.loadFromDB);
  const captureEnabled = useSettingsStore((s) => s.captureEnabled);
  const setHighlightedIds = useContentStore((s) => s.setHighlightedIds);

  // Track scroll positions per tab for restore on switch-back
  const scrollPositions = useRef<Record<TabId, number>>({
    content: 0,
    report: 0,
    settings: 0,
  });

  // Load settings from database on startup
  useEffect(() => {
    loadFromDB();
  }, [loadFromDB]);

  // Save scroll position before switching away, then switch tab
  const switchTab = useCallback(
    (newTab: TabId, highlightIds?: string[]) => {
      // Save current scroll position
      scrollPositions.current[activeTab] = window.scrollY;

      // Set highlights if navigating to content with specific IDs
      if (newTab === "content" && highlightIds && highlightIds.length > 0) {
        setHighlightedIds(highlightIds);
      }

      setActiveTab(newTab);

      // Restore scroll position for the new tab
      // (skip restore if we have highlights — ContentList will handle scroll-to-item)
      if (!(newTab === "content" && highlightIds && highlightIds.length > 0)) {
        requestAnimationFrame(() => {
          window.scrollTo(0, scrollPositions.current[newTab]);
        });
      }
    },
    [activeTab, setHighlightedIds]
  );

  // Listen for tab navigation events from the tray menu
  useEffect(() => {
    const unlisten = listen<string>("navigate-tab", (event) => {
      const tab = event.payload as TabId;
      if (TABS.some((t) => t.id === tab)) {
        switchTab(tab);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [switchTab]);

  // Listen for "navigate-to-content" events from ReportCard's "跳转原文" button
  useEffect(() => {
    const handler = (e: Event) => {
      const customEvent = e as CustomEvent<{ contentIds?: string[] }>;
      const contentIds = customEvent.detail?.contentIds ?? [];
      switchTab("content", contentIds);
    };
    window.addEventListener("navigate-to-content", handler);
    return () => window.removeEventListener("navigate-to-content", handler);
  }, [switchTab]);

  return (
    <div className="min-h-screen relative overflow-hidden bg-gradient-to-br from-indigo-100/80 via-white to-purple-100/60 dark:from-slate-950 dark:via-slate-900 dark:to-indigo-950 transition-colors duration-300">
      {/* Floating gradient orbs — the key to glass effect */}
      <div className="orb orb-1" />
      <div className="orb orb-2" />
      <div className="orb orb-3" />
      <div className="orb orb-4" />

      {/* Header with tab navigation */}
      <header className="glass-heavy sticky top-0 z-10 border-b border-white/30 dark:border-white/[0.06]" style={{ borderTop: 'none', borderLeft: 'none', borderRight: 'none' }}>
        <div className="px-6 pt-4 pb-0">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-3">
              <div>
                <h1 className="text-xl font-bold bg-gradient-to-r from-indigo-600 to-purple-600 dark:from-indigo-400 dark:to-purple-400 bg-clip-text text-transparent">
                  小云
                </h1>
                <p className="text-xs text-gray-400 dark:text-slate-500">你的智能信息助手</p>
              </div>
            </div>
            {/* Capture status */}
            <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-full glass text-[11px]">
              <span className={`w-1.5 h-1.5 rounded-full ${captureEnabled ? "bg-green-400 shadow-[0_0_6px_rgba(74,222,128,0.6)] animate-pulse" : "bg-gray-300 dark:bg-slate-600"}`} />
              <span className="text-gray-500 dark:text-slate-400">
                {captureEnabled ? "运行中" : "已暂停"}
              </span>
            </div>
          </div>

          {/* Tab navigation bar */}
          <nav className="flex gap-1">
            {TABS.map((tab) => (
              <button
                key={tab.id}
                onClick={() => switchTab(tab.id)}
                className={`
                  relative flex items-center gap-1.5 px-4 py-2 text-sm font-medium
                  rounded-xl transition-all duration-200
                  ${
                    activeTab === tab.id
                      ? "bg-white/60 dark:bg-white/[0.12] text-indigo-600 dark:text-indigo-400 shadow-[0_2px_8px_rgba(99,102,241,0.1)] backdrop-blur-sm border border-white/60 dark:border-white/[0.1]"
                      : "text-gray-500 dark:text-slate-400 hover:text-gray-700 dark:hover:text-slate-300 hover:bg-white/30 dark:hover:bg-white/[0.04]"
                  }
                `}
              >
                <span className="text-base">{tab.icon}</span>
                <span>{tab.label}</span>
              </button>
            ))}
          </nav>
        </div>
      </header>

      {/* Tab content — relative z-index above orbs */}
      <main className="relative z-[1]">
        {activeTab === "content" && <ContentList />}
        {activeTab === "report" && <ReportView />}
        {activeTab === "settings" && <SettingsView />}
      </main>

      {/* Floating bubble is now a separate always-on-top window (BubbleView) */}
    </div>
  );
}

export default App;
