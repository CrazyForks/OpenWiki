import { Target, Zap, Sparkles } from "lucide-react";
import type { EvidenceItem, EvidenceGroup } from "../../services/radarService";

const TYPE_CONFIG = {
  thread: {
    color: "#F97316",
    bg: "#FFF7ED",
    icon: Target,
    label: "反复线索",
    cta: "深入了解这个线索 →",
  },
  connection: {
    color: "#2563EB",
    bg: "#EFF6FF",
    icon: Zap,
    label: "意外联系",
    cta: "探索这个联系 →",
  },
  obsession: {
    color: "#16A34A",
    bg: "#F0FDF4",
    icon: Sparkles,
    label: "新痴迷",
    cta: "深入了解这个线索 →",
  },
} as const;

interface InsightCardProps {
  type: "thread" | "connection" | "obsession";
  title: string;
  whyNow: string;
  evidence?: EvidenceItem[];
  groupA?: EvidenceGroup;
  groupB?: EvidenceGroup;
  sinceDays?: number;
  onExpand: () => void;
}

export function InsightCard({
  type,
  title,
  whyNow,
  evidence,
  groupA,
  groupB,
  sinceDays,
  onExpand,
}: InsightCardProps) {
  const config = TYPE_CONFIG[type];
  const Icon = config.icon;
  const maxEvidence = 4;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") onExpand();
  };

  const renderEvidence = (items: EvidenceItem[], max: number) => {
    const shown = items.slice(0, max);
    const extra = items.length - max;
    return (
      <div className="space-y-1.5">
        {shown.map((item, i) => (
          <div key={i} className="flex items-baseline gap-2">
            <span
              className="w-1.5 h-1.5 rounded-full flex-shrink-0 mt-1"
              style={{ backgroundColor: config.color }}
            />
            <span
              className="font-mono flex-shrink-0"
              style={{ fontSize: 11, color: "#A8A29E" }}
            >
              {item.date}
            </span>
            <span className="truncate" style={{ fontSize: 13, color: "var(--color-text-secondary)" }}>
              {item.title}
            </span>
          </div>
        ))}
        {extra > 0 && (
          <span className="text-stone-400 pl-3.5" style={{ fontSize: 12 }}>
            +{extra} more
          </span>
        )}
      </div>
    );
  };

  return (
    <div
      tabIndex={0}
      role="button"
      onClick={onExpand}
      onKeyDown={handleKeyDown}
      className="rounded-xl p-4 cursor-pointer transition-colors outline-none
                 focus-visible:ring-2 focus-visible:ring-orange-300"
      style={{ backgroundColor: "var(--color-surface)", border: "1px solid var(--color-border)" }}
    >
      {/* Header */}
      <div className="flex items-center gap-2 mb-2">
        <span
          className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full"
          style={{ backgroundColor: config.bg, color: config.color, fontSize: 12 }}
        >
          <Icon size={13} strokeWidth={2} />
          {config.label}
        </span>
        {type === "obsession" && sinceDays !== undefined && (
          <span className="text-stone-400" style={{ fontSize: 11 }}>
            {sinceDays} 天前开始
          </span>
        )}
      </div>

      {/* Title */}
      <h3 className="font-semibold mb-2" style={{ fontSize: 15, color: "var(--color-text-primary)" }}>
        {title}
      </h3>

      {/* AI explanation */}
      <p
        className="pl-3 mb-3"
        style={{ fontSize: 13, color: "var(--color-text-secondary)", borderLeft: "2px solid var(--color-border)" }}
      >
        {whyNow}
      </p>

      {/* Evidence */}
      {type === "connection" && groupA && groupB ? (
        <div className="grid grid-cols-2 gap-2 mb-3">
          <div className="rounded-lg p-3" style={{ backgroundColor: "var(--color-surface-raised)" }}>
            <p className="font-medium mb-1.5" style={{ fontSize: 12, color: "var(--color-text-muted)" }}>
              {groupA.topic}
            </p>
            {renderEvidence(groupA.evidence, maxEvidence)}
          </div>
          <div className="rounded-lg p-3" style={{ backgroundColor: "var(--color-surface-raised)" }}>
            <p className="font-medium mb-1.5" style={{ fontSize: 12, color: "var(--color-text-muted)" }}>
              {groupB.topic}
            </p>
            {renderEvidence(groupB.evidence, maxEvidence)}
          </div>
        </div>
      ) : evidence && evidence.length > 0 ? (
        <div className="mb-3">{renderEvidence(evidence, maxEvidence)}</div>
      ) : null}

      {/* CTA */}
      <span className="font-medium" style={{ fontSize: 13, color: config.color }}>
        {config.cta}
      </span>
    </div>
  );
}
