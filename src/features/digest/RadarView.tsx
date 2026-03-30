import { useEffect } from "react";
import { RefreshCw, Target, Key, Search } from "lucide-react";
import { useRadarStore } from "../../stores/radarStore";
import { InsightCard } from "./InsightCard";
import { InsightDetail } from "./InsightDetail";
import type {
  RecurringThread,
  UnexpectedConnection,
  NewObsession,
} from "../../services/radarService";

export function RadarView() {
  const {
    status,
    analysis,
    contentCount,
    hasNewContent,
    errorMessage,
    isLoading,
    selectedInsight,
    loadRadar,
    triggerAnalysis,
    selectInsight,
    clearSelection,
    setupEventListener,
  } = useRadarStore();

  useEffect(() => {
    loadRadar();
    let unlisten: (() => void) | undefined;
    setupEventListener().then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [loadRadar, setupEventListener]);

  const threads = analysis?.analysis.recurring_threads ?? [];
  const connections = analysis?.analysis.unexpected_connections ?? [];
  const obsessions = analysis?.analysis.new_obsessions ?? [];
  const hasFindings = threads.length > 0 || connections.length > 0 || obsessions.length > 0;
  const isAnalyzing = status === "analyzing";

  // Sort threads by evidence count descending
  const sortedThreads = [...threads].sort(
    (a, b) => b.evidence.length - a.evidence.length
  );

  // Detail view
  if (selectedInsight) {
    let detailType: "thread" | "connection" | "obsession";
    let detailData: RecurringThread | UnexpectedConnection | NewObsession | null = null;

    if (selectedInsight.type === "thread" && sortedThreads[selectedInsight.index]) {
      detailType = "thread";
      detailData = sortedThreads[selectedInsight.index];
    } else if (selectedInsight.type === "connection" && connections[selectedInsight.index]) {
      detailType = "connection";
      detailData = connections[selectedInsight.index];
    } else if (selectedInsight.type === "obsession" && obsessions[selectedInsight.index]) {
      detailType = "obsession";
      detailData = obsessions[selectedInsight.index];
    } else {
      detailType = "thread";
    }

    if (detailData) {
      return (
        <div className="px-5 py-4 overflow-y-auto" style={{ height: "calc(100vh - 44px)" }}>
          <InsightDetail
            type={detailType}
            data={detailData}
            onBack={clearSelection}
          />
        </div>
      );
    }
  }

  return (
    <div className="px-5 py-4 overflow-y-auto" style={{ height: "calc(100vh - 44px)" }}>
      {/* Header */}
      <div className="flex items-center justify-between mb-1">
        <h2
          className="font-bold text-stone-900"
          style={{ fontSize: 24, fontFamily: "'Cabinet Grotesk', sans-serif", fontWeight: 700 }}
        >
          注意力雷达
        </h2>
        <button
          onClick={() => triggerAnalysis()}
          disabled={isAnalyzing || !hasNewContent}
          className="p-2 rounded-lg text-stone-400 hover:text-stone-600 hover:bg-stone-100
                     disabled:opacity-40 disabled:cursor-not-allowed transition-all"
          title="刷新分析"
        >
          <RefreshCw
            size={18}
            strokeWidth={2}
            className={isAnalyzing ? "animate-spin" : ""}
          />
        </button>
      </div>

      {/* Subtitle */}
      {!isLoading && hasFindings && (
        <p className="text-stone-500 mb-6" style={{ fontSize: 13 }}>
          最近 14 天 · 基于 {contentCount} 条内容分析
        </p>
      )}

      {/* Loading skeleton */}
      {isLoading && (
        <div className="space-y-6 mt-6">
          {/* Stats skeleton */}
          <div className="grid grid-cols-4 gap-3">
            {[...Array(4)].map((_, i) => (
              <div key={i} className="h-16 bg-stone-100 rounded-xl animate-pulse" />
            ))}
          </div>
          {/* Card skeletons */}
          {[...Array(2)].map((_, i) => (
            <div key={i} className="h-40 bg-stone-100 rounded-xl animate-pulse" />
          ))}
        </div>
      )}

      {/* Empty states */}
      {!isLoading && status === "no_api_key" && (
        <div className="flex-1 flex flex-col items-center justify-center text-center py-20">
          <Key size={48} className="text-stone-300 mb-4" strokeWidth={1.5} />
          <p className="text-base font-medium text-stone-700 mb-1">需要配置 AI 服务</p>
          <p className="text-stone-500 mb-4" style={{ fontSize: 13 }}>
            注意力雷达需要 AI 来分析你的内容
          </p>
          <button
            onClick={() => {/* navigate to settings - handled by parent */}}
            className="text-orange-500 font-medium hover:underline"
            style={{ fontSize: 13 }}
          >
            前往设置 →
          </button>
        </div>
      )}

      {!isLoading && status === "not_enough_content" && (
        <div className="flex-1 flex flex-col items-center justify-center text-center py-20">
          <Target size={48} className="text-stone-300 mb-4" strokeWidth={1.5} />
          <p className="text-base font-medium text-stone-700 mb-1">你离洞察只差几步</p>
          <p className="text-stone-500" style={{ fontSize: 13 }}>
            继续保存你感兴趣的内容，积累到 5 条就能开始分析
          </p>
        </div>
      )}

      {!isLoading && !isAnalyzing && !hasFindings &&
       status !== "no_api_key" && status !== "not_enough_content" &&
       status !== "error" && (
        <div className="flex-1 flex flex-col items-center justify-center text-center py-20">
          <Search size={48} className="text-stone-300 mb-4" strokeWidth={1.5} />
          <p className="text-base font-medium text-stone-700 mb-1">暂时没有发现模式</p>
          <p className="text-stone-500" style={{ fontSize: 13 }}>
            继续保存内容，小云会持续寻找你的注意力模式
          </p>
        </div>
      )}

      {/* Error state */}
      {!isLoading && status === "error" && (
        <div className="rounded-xl p-4 mt-4" style={{ backgroundColor: "#FEF2F2" }}>
          <p className="text-red-700 mb-2" style={{ fontSize: 13 }}>
            {errorMessage || "分析时出现错误"}
          </p>
          <button
            onClick={() => loadRadar()}
            className="text-red-500 font-medium hover:underline"
            style={{ fontSize: 13 }}
          >
            重试
          </button>
        </div>
      )}

      {/* Content: stats + sections */}
      {!isLoading && hasFindings && (
        <>
          {/* Stats bar */}
          <div className="grid grid-cols-4 gap-3 mb-8">
            <StatItem label="保存内容" value={contentCount} color="#1C1917" />
            <StatItem label="反复线索" value={threads.length} color="#F97316" />
            <StatItem label="意外联系" value={connections.length} color="#2563EB" />
            <StatItem label="新痴迷" value={obsessions.length} color="#16A34A" />
          </div>

          {/* Threads section */}
          {sortedThreads.length > 0 && (
            <Section icon={<Target size={16} strokeWidth={2} className="text-orange-500" />} title="反复出现的线索">
              {sortedThreads.map((t, i) => (
                <InsightCard
                  key={i}
                  type="thread"
                  title={t.title}
                  whyNow={t.why_now}
                  evidence={t.evidence}
                  onExpand={() => selectInsight("thread", i)}
                />
              ))}
            </Section>
          )}

          {/* Connections section */}
          {connections.length > 0 && (
            <Section icon={<span className="text-blue-600">⚡</span>} title="意外联系">
              {connections.map((c, i) => (
                <InsightCard
                  key={i}
                  type="connection"
                  title={c.title}
                  whyNow={c.why_now}
                  groupA={c.group_a}
                  groupB={c.group_b}
                  onExpand={() => selectInsight("connection", i)}
                />
              ))}
            </Section>
          )}

          {/* Obsessions section */}
          {obsessions.length > 0 && (
            <Section icon={<span className="text-green-600">★</span>} title="新痴迷">
              {obsessions.map((o, i) => (
                <InsightCard
                  key={i}
                  type="obsession"
                  title={o.title}
                  whyNow={o.why_now}
                  evidence={o.evidence}
                  sinceDays={o.since_days}
                  onExpand={() => selectInsight("obsession", i)}
                />
              ))}
            </Section>
          )}

          {/* Analyzing overlay when updating existing data */}
          {isAnalyzing && (
            <div className="text-center py-6">
              <RefreshCw size={16} className="animate-spin text-stone-400 mx-auto mb-2" />
              <p className="text-stone-400" style={{ fontSize: 13 }}>正在更新分析...</p>
            </div>
          )}
        </>
      )}

      {/* Analyzing with no existing data */}
      {!isLoading && isAnalyzing && !hasFindings && (
        <div className="space-y-6 mt-6">
          <div className="grid grid-cols-4 gap-3">
            {[...Array(4)].map((_, i) => (
              <div key={i} className="h-16 bg-stone-100 rounded-xl animate-pulse" />
            ))}
          </div>
          {[...Array(2)].map((_, i) => (
            <div key={i} className="h-40 bg-stone-100 rounded-xl animate-pulse" />
          ))}
          <div className="text-center py-4">
            <RefreshCw size={16} className="animate-spin text-stone-400 mx-auto mb-2" />
            <p className="text-stone-400" style={{ fontSize: 13 }}>正在分析你的内容...</p>
          </div>
        </div>
      )}
    </div>
  );
}

function StatItem({ label, value, color }: { label: string; value: number; color: string }) {
  return (
    <div className="bg-white border border-stone-200 rounded-xl p-3 text-center">
      <p
        className="font-bold font-mono"
        style={{ fontSize: 20, color }}
      >
        {value}
      </p>
      <p className="text-stone-500" style={{ fontSize: 11 }}>
        {label}
      </p>
    </div>
  );
}

function Section({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="mb-8">
      <div className="flex items-center gap-2 mb-3">
        {icon}
        <h3 className="font-semibold text-stone-700" style={{ fontSize: 15 }}>
          {title}
        </h3>
      </div>
      <div className="space-y-3">{children}</div>
    </div>
  );
}
