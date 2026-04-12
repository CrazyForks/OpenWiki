import { useEffect, useCallback, useState, useMemo, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { useContentStore } from "../../stores/contentStore";
import { getAllContent, getStorageInfo, getContentsByIds } from "../../services/storageService";
import { exportAllSingle, exportRangeSingle } from "../../services/dataHubService";
import { useSettingsStore, containsSensitiveData } from "../../stores/settingsStore";
import { ContentCard } from "./ContentCard";
import type { ContentType } from "../../types/content";

type FilterType = "all" | ContentType;
type DateRange = "all" | "today" | "week" | "half-month";

const FILTER_TABS: { value: FilterType; labelKey: string; icon: string }[] = [
  { value: "all", labelKey: "filter.all", icon: "📋" },
  { value: "text", labelKey: "filter.text", icon: "📝" },
  { value: "image", labelKey: "filter.image", icon: "🖼️" },
  { value: "url", labelKey: "filter.url", icon: "🔗" },
];

const PAGE_SIZE = 50;

export function ContentList() {
  const { t } = useTranslation("content");
  const { contents, isLoading, setContents, setIsLoading } = useContentStore();
  const hasMore = useContentStore((s) => s.hasMore);
  const totalCount = useContentStore((s) => s.totalCount);
  const isLoadingMore = useContentStore((s) => s.isLoadingMore);
  const setHasMore = useContentStore((s) => s.setHasMore);
  const setTotalCount = useContentStore((s) => s.setTotalCount);
  const setIsLoadingMore = useContentStore((s) => s.setIsLoadingMore);
  const appendContents = useContentStore((s) => s.appendContents);
  const highlightedIds = useContentStore((s) => s.highlightedIds);
  const scrollToId = useContentStore((s) => s.scrollToId);
  const setScrollToId = useContentStore((s) => s.setScrollToId);
  const clearHighlights = useContentStore((s) => s.clearHighlights);
  const captureEnabled = useSettingsStore((s) => s.captureEnabled);
  const sensitiveFilterEnabled = useSettingsStore((s) => s.sensitiveFilterEnabled);
  const setStorageInfo = useSettingsStore((s) => s.setStorageInfo);
  const [filter, setFilter] = useState<FilterType>("all");
  const [dateRange, setDateRange] = useState<DateRange>("all");
  const [exportStatus, setExportStatus] = useState<"idle" | "confirm" | "exporting" | "done">("idle");
  const confirmTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Refs for scroll-to-item and infinite scroll sentinel
  const cardRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // Load initial page (first 50 items)
  const loadInitial = useCallback(async () => {
    setIsLoading(true);
    try {
      const info = await getStorageInfo();
      setStorageInfo(info.total_items, info.disk_usage_mb);
      setTotalCount(info.total_items);
      const data = await getAllContent(PAGE_SIZE, 0);
      setContents(data);
      setHasMore(data.length < info.total_items);
    } catch (e) {
      console.error("Failed to load content:", e);
    } finally {
      setIsLoading(false);
    }
  }, [setContents, setIsLoading, setStorageInfo, setTotalCount, setHasMore]);

  // Load more items (append next batch)
  const loadMore = useCallback(async () => {
    if (isLoadingMore || !hasMore) return;
    setIsLoadingMore(true);
    try {
      const offset = contents.length;
      const data = await getAllContent(PAGE_SIZE, offset);
      appendContents(data);
      if (data.length < PAGE_SIZE) setHasMore(false);
    } catch (e) {
      console.error("Failed to load more:", e);
    } finally {
      setIsLoadingMore(false);
    }
  }, [contents.length, isLoadingMore, hasMore, appendContents, setIsLoadingMore, setHasMore]);

  useEffect(() => {
    loadInitial();
  }, [loadInitial]);

  // Scroll listener: trigger loadMore when near bottom of scroll container
  useEffect(() => {
    const el = scrollContainerRef.current;
    if (!el || !hasMore) return;
    const handleScroll = () => {
      if (el.scrollTop + el.clientHeight >= el.scrollHeight - 300) {
        loadMore();
      }
    };
    el.addEventListener("scroll", handleScroll, { passive: true });
    return () => el.removeEventListener("scroll", handleScroll);
  }, [hasMore, loadMore]);

  useEffect(() => {
    const handleFocus = () => { loadInitial(); };
    window.addEventListener("focus", handleFocus);
    return () => { window.removeEventListener("focus", handleFocus); };
  }, [loadInitial]);

  // Listen for content updates — reload single item instead of full list
  const reloadSingleItem = useCallback(async (id: string) => {
    if (!id) return;
    try {
      const items = await getContentsByIds([id]);
      if (items.length > 0) {
        useContentStore.getState().updateContent(items[0]);
      }
    } catch (e) { console.error("Failed to reload item:", e); }
  }, []);

  useEffect(() => {
    const unlisten = listen<{ id: string; reorder?: boolean }>(
      "content:url-fetched",
      (event) => { reloadSingleItem(event.payload.id); }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [reloadSingleItem]);

  useEffect(() => {
    const unlisten = listen<string>(
      "content:clean-ready",
      (event) => { reloadSingleItem(event.payload); }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [reloadSingleItem]);

  useEffect(() => {
    const unlisten = listen<string>(
      "content-summary-ready",
      (event) => { reloadSingleItem(event.payload); }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [reloadSingleItem]);

  useEffect(() => {
    const unlisten = listen<{ id: string }>(
      "content:ocr-done",
      (event) => { reloadSingleItem(event.payload.id); }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [reloadSingleItem]);

  // Handle scroll-to-item when scrollToId changes
  useEffect(() => {
    if (!scrollToId) return;

    // Reset filter to "all" so the target item is visible
    setFilter("all");

    // Wait for render, then scroll to the item
    const timer = setTimeout(() => {
      const el = cardRefs.current[scrollToId];
      if (el) {
        el.scrollIntoView({ behavior: "smooth", block: "center" });
        setScrollToId(null);
      }
    }, 150);

    return () => clearTimeout(timer);
  }, [scrollToId, setScrollToId, contents]);

  // Auto-clear highlights after 4 seconds
  useEffect(() => {
    if (highlightedIds.length === 0) return;
    const timer = setTimeout(() => {
      clearHighlights();
    }, 4000);
    return () => clearTimeout(timer);
  }, [highlightedIds, clearHighlights]);

  const filteredContents = useMemo(() => {
    let result = contents;
    if (sensitiveFilterEnabled) {
      result = result.filter((c) => !c.raw_text || !containsSensitiveData(c.raw_text));
    }
    if (filter !== "all") {
      result = result.filter((c) => c.content_type === filter);
    }
    if (dateRange !== "all") {
      const now = new Date();
      const cutoff = new Date();
      if (dateRange === "today") {
        cutoff.setHours(0, 0, 0, 0);
      } else if (dateRange === "week") {
        cutoff.setDate(now.getDate() - 7);
      } else if (dateRange === "half-month") {
        cutoff.setDate(now.getDate() - 15);
      }
      result = result.filter((c) => new Date(c.captured_at) >= cutoff);
    }
    return result;
  }, [contents, filter, sensitiveFilterEnabled, dateRange]);

  const typeCounts = useMemo(() => {
    const counts: Record<string, number> = { all: totalCount };
    for (const c of contents) {
      counts[c.content_type] = (counts[c.content_type] || 0) + 1;
    }
    return counts;
  }, [contents, totalCount]);

  if (isLoading) {
    return (
      <div className="p-4 space-y-3">
        <div className="flex items-center justify-between px-1">
          <div className="h-6 w-32 bg-white/50 dark:bg-white/[0.06] rounded-lg animate-pulse" />
          <div className="h-5 w-16 bg-white/50 dark:bg-white/[0.06] rounded-full animate-pulse" />
        </div>
        {[1, 2, 3].map((i) => (
          <div key={i} className="glass rounded-2xl p-4">
            <div className="flex items-start gap-3">
              <div className="w-8 h-8 bg-orange-500/10 dark:bg-orange-500/10 rounded-xl animate-pulse" />
              <div className="flex-1 space-y-2">
                <div className="h-4 bg-gray-200/50 dark:bg-white/[0.06] rounded w-3/4 animate-pulse" />
                <div className="h-3 bg-gray-200/30 dark:bg-white/[0.04] rounded w-1/2 animate-pulse" />
                <div className="h-3 bg-gray-200/30 dark:bg-white/[0.04] rounded w-1/3 animate-pulse" />
              </div>
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (contents.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-80">
        <div className="w-20 h-20 rounded-2xl glass flex items-center justify-center mb-5">
          <span className="text-4xl">📭</span>
        </div>
        <div className="font-medium text-gray-600 dark:text-slate-300 mb-2">
          {t("emptyTitle")}
        </div>
        <div className="text-sm text-gray-400 dark:text-slate-500 text-center max-w-xs">
          {t("emptyHint")}
        </div>
        <div className="mt-4 flex items-center gap-1.5 text-xs">
          <span className={`w-2 h-2 rounded-full ${captureEnabled ? "bg-green-400 animate-pulse" : "bg-gray-300 dark:bg-slate-600"}`} />
          <span className="text-gray-400 dark:text-slate-500">
            {captureEnabled ? t("captureOn") : t("captureOff")}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div ref={scrollContainerRef} className="overflow-y-auto p-4 space-y-3" style={{ height: "calc(100vh - 44px)" }}>
      {/* Header with filter tabs */}
      <div className="flex items-center justify-between px-1">
        <div className="flex items-center gap-1 p-0.5 rounded-xl glass">
          {FILTER_TABS.map((tab) => {
            const count = typeCounts[tab.value] || 0;
            if (tab.value !== "all" && count === 0) return null;
            const isActive = filter === tab.value;
            return (
              <button
                key={tab.value}
                onClick={() => setFilter(tab.value)}
                className={`
                  flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium rounded-lg transition-all
                  ${isActive
                    ? "bg-white/80 dark:bg-white/[0.1] text-orange-600 dark:text-orange-400 shadow-sm"
                    : "text-gray-500 dark:text-slate-400 hover:text-gray-700 dark:hover:text-slate-300"
                  }
                `}
              >
                <span className="text-sm">{tab.icon}</span>
                <span>{t(tab.labelKey)}</span>
                <span className={`
                  ml-0.5 px-1.5 py-0.5 rounded-full text-[10px]
                  ${isActive
                    ? "bg-orange-500/10 dark:bg-orange-500/20 text-orange-600 dark:text-orange-400"
                    : "bg-gray-200/50 dark:bg-white/[0.06] text-gray-400 dark:text-slate-500"
                  }
                `}>
                  {count}
                </span>
              </button>
            );
          })}
        </div>
        <div className="flex items-center gap-1.5">
          {/* Date range filters */}
          {(["all", "today", "week", "half-month"] as DateRange[]).map((range) => {
            const labelKey = range === "all" ? "dateRange.all" : range === "today" ? "dateRange.today" : range === "week" ? "dateRange.week" : "dateRange.halfMonth";
            const label = t(labelKey);
            const isActive = dateRange === range;
            return (
              <button
                key={range}
                onClick={() => setDateRange(isActive && range !== "all" ? "all" : range)}
                className={`text-[11px] px-2.5 py-1 rounded-md border transition-all
                  ${isActive
                    ? "text-white bg-orange-500 border-orange-500"
                    : "text-gray-400 dark:text-slate-500 border-gray-200/60 dark:border-white/[0.08] bg-white/60 dark:bg-white/[0.04] hover:border-orange-300 hover:text-orange-500"
                  }`}
              >
                {label}
              </button>
            );
          })}

          {/* Separator */}
          <div className="w-px h-4 bg-gray-200/60 dark:bg-white/[0.08] mx-0.5" />

          {/* Export current view */}
          <button
            onClick={async () => {
              if (exportStatus === "idle") {
                // First click: show confirm
                setExportStatus("confirm");
                confirmTimer.current = setTimeout(() => setExportStatus("idle"), 3000);
                return;
              }
              if (exportStatus === "confirm") {
                // Second click: do export
                if (confirmTimer.current) clearTimeout(confirmTimer.current);
                setExportStatus("exporting");
                try {
                  if (dateRange === "all") {
                    await exportAllSingle();
                  } else {
                    const now = new Date();
                    const end = now.toISOString().slice(0, 10);
                    const start = new Date();
                    if (dateRange === "today") start.setHours(0, 0, 0, 0);
                    else if (dateRange === "week") start.setDate(now.getDate() - 7);
                    else if (dateRange === "half-month") start.setDate(now.getDate() - 15);
                    await exportRangeSingle(start.toISOString().slice(0, 10), end);
                  }
                  setExportStatus("done");
                  setTimeout(() => setExportStatus("idle"), 3000);
                } catch (e) { console.error(e); setExportStatus("idle"); }
              }
            }}
            disabled={exportStatus === "exporting"}
            className={`text-[11px] px-2.5 py-1 rounded-md border transition-all flex items-center gap-1
              ${exportStatus === "confirm"
                ? "text-orange-600 border-orange-400 bg-orange-100 dark:bg-orange-500/20"
                : exportStatus === "done"
                ? "text-green-600 border-green-300 bg-green-50"
                : exportStatus === "exporting"
                ? "text-orange-500 border-orange-300 bg-orange-50 animate-pulse"
                : "text-gray-400 dark:text-slate-500 border-gray-200/60 dark:border-white/[0.08] bg-white/60 dark:bg-white/[0.04] hover:border-orange-300 hover:text-orange-500"
              }`}
          >
            {exportStatus === "confirm" ? t("export.confirm") : exportStatus === "exporting" ? t("export.exporting") : exportStatus === "done" ? `✓ ${t("export.done")}` : `↗ ${t("export.button")}`}
          </button>

          {/* Capture status */}
          <div className="flex items-center gap-1 text-[11px] text-gray-400 dark:text-slate-500 ml-1">
            <span className={`w-1.5 h-1.5 rounded-full ${captureEnabled ? "bg-green-400" : "bg-gray-300 dark:bg-slate-600"}`} />
            {captureEnabled ? t("capture.active") : t("capture.paused")}
          </div>
        </div>
      </div>

      {/* Content cards */}
      {filteredContents.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <span className="text-3xl mb-3">🔍</span>
          <p className="text-sm text-gray-500 dark:text-slate-400">
            {t("emptyFilter", { type: t(FILTER_TABS.find((tab) => tab.value === filter)?.labelKey ?? "filter.all") })}
          </p>
        </div>
      ) : (
        <div className="space-y-2.5">
          {filteredContents.map((content) => (
            <ContentCard
              key={content.id}
              content={content}
              isHighlighted={highlightedIds.includes(content.id)}
              ref={(el) => { cardRefs.current[content.id] = el; }}
            />
          ))}
          {hasMore && isLoadingMore && (
            <div className="flex justify-center py-4">
              <span className="text-xs text-gray-400 dark:text-slate-500 animate-pulse">
                {t("loading", "加载中...")}
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
