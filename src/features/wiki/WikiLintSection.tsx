import { useState, useEffect } from "react";
import { AlertTriangle, Check, Trash2, RotateCcw, RefreshCw } from "lucide-react";
import type { WikiLintResult } from "../../types/wiki";
import { getWikiLintResults, triggerWikiLint, wikiLintKeep, wikiLintDelete, wikiLintRecompile } from "../../services/wikiService";

const SEVERITY_STYLES: Record<string, { bg: string; border: string; icon: string }> = {
  critical: { bg: "#DC262610", border: "#DC262630", icon: "#DC2626" },
  warning: { bg: "#CA8A0410", border: "#CA8A0430", icon: "#CA8A04" },
  info: { bg: "#2563EB10", border: "#2563EB30", icon: "#2563EB" },
};

interface WikiLintSectionProps {
  compact?: boolean;
}

export function WikiLintSection({ compact = false }: WikiLintSectionProps) {
  const [results, setResults] = useState<WikiLintResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [acting, setActing] = useState<number | null>(null);

  const load = async () => {
    try {
      const r = await getWikiLintResults();
      setResults(r);
    } catch (e) {
      console.error("Failed to load lint results:", e);
    }
  };

  useEffect(() => { load(); }, []);

  const handleRefresh = async () => {
    setLoading(true);
    try {
      await triggerWikiLint();
      await load();
    } catch (e) {
      console.error("Lint failed:", e);
    }
    setLoading(false);
  };

  const handleAction = async (id: number, action: "keep" | "delete" | "recompile") => {
    setActing(id);
    try {
      if (action === "keep") await wikiLintKeep(id);
      else if (action === "delete") await wikiLintDelete(id);
      else await wikiLintRecompile(id);
      setResults(results.filter((r) => r.id !== id));
    } catch (e) {
      console.error("Lint action failed:", e);
    }
    setActing(null);
  };

  if (results.length === 0 && compact) return null;

  return (
    <div className={compact ? "" : "mt-4"}>
      <div className="flex items-center justify-between mb-2">
        <h3 className="flex items-center gap-1.5" style={{ fontSize: 13, fontWeight: 600, color: "var(--color-text-primary)" }}>
          <AlertTriangle size={14} style={{ color: "#CA8A04" }} />
          知识库健康
          {results.length > 0 && (
            <span className="ml-1 px-1.5 py-0.5 rounded-full text-[10px] font-bold"
              style={{ backgroundColor: "#DC262615", color: "#DC2626" }}>
              {results.length}
            </span>
          )}
        </h3>
        <button
          onClick={handleRefresh}
          disabled={loading}
          className="p-1 rounded-lg text-stone-400 hover:text-stone-600 hover:bg-stone-100 dark:hover:bg-white/[0.08] transition-all disabled:opacity-40"
          title="运行健康检查"
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
        </button>
      </div>

      {results.length === 0 ? (
        <p style={{ fontSize: 12, color: "var(--color-text-muted)" }}>
          {compact ? "" : "知识库状态良好，暂无问题"}
        </p>
      ) : (
        <div className="space-y-2">
          {results.map((r) => {
            const style = SEVERITY_STYLES[r.severity] || SEVERITY_STYLES.info;
            const isActing = acting === r.id;
            return (
              <div
                key={r.id}
                className="rounded-lg p-3"
                style={{ backgroundColor: style.bg, border: `1px solid ${style.border}` }}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex-1 min-w-0">
                    <p className="font-medium" style={{ fontSize: 13, color: "var(--color-text-primary)" }}>
                      {r.title}
                    </p>
                    <p className="mt-0.5" style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
                      {r.description}
                    </p>
                  </div>
                  <div className="flex items-center gap-1 flex-shrink-0">
                    <button
                      onClick={() => handleAction(r.id, "keep")}
                      disabled={isActing}
                      className="p-1 rounded hover:bg-white/50 dark:hover:bg-white/[0.1] text-stone-400 hover:text-green-600 transition-colors"
                      title="保留"
                    >
                      <Check size={14} />
                    </button>
                    <button
                      onClick={() => handleAction(r.id, "recompile")}
                      disabled={isActing}
                      className="p-1 rounded hover:bg-white/50 dark:hover:bg-white/[0.1] text-stone-400 hover:text-orange-500 transition-colors"
                      title="重编"
                    >
                      <RotateCcw size={14} />
                    </button>
                    <button
                      onClick={() => handleAction(r.id, "delete")}
                      disabled={isActing}
                      className="p-1 rounded hover:bg-white/50 dark:hover:bg-white/[0.1] text-stone-400 hover:text-red-500 transition-colors"
                      title="删除"
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
