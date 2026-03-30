import { ArrowLeft, Target, Zap, Sparkles } from "lucide-react";
import type {
  RecurringThread,
  UnexpectedConnection,
  NewObsession,
} from "../../services/radarService";

const TYPE_CONFIG = {
  thread: { color: "#F97316", bg: "#FFF7ED", icon: Target, label: "反复线索" },
  connection: { color: "#2563EB", bg: "#EFF6FF", icon: Zap, label: "意外联系" },
  obsession: { color: "#16A34A", bg: "#F0FDF4", icon: Sparkles, label: "新痴迷" },
} as const;

interface InsightDetailProps {
  type: "thread" | "connection" | "obsession";
  data: RecurringThread | UnexpectedConnection | NewObsession;
  onBack: () => void;
}

export function InsightDetail({ type, data, onBack }: InsightDetailProps) {
  const config = TYPE_CONFIG[type];
  const Icon = config.icon;

  const isConnection = type === "connection";
  const connData = isConnection ? (data as UnexpectedConnection) : null;
  const evidenceData = !isConnection
    ? (data as RecurringThread | NewObsession)
    : null;

  const evidenceCount = isConnection
    ? (connData!.group_a.evidence.length + connData!.group_b.evidence.length)
    : (evidenceData!.evidence.length);

  return (
    <div className="space-y-6">
      {/* Back button */}
      <button
        onClick={onBack}
        className="flex items-center gap-1 transition-colors"
        style={{ fontSize: 13, color: "var(--color-text-muted)" }}
      >
        <ArrowLeft size={14} strokeWidth={2} />
        返回雷达
      </button>

      {/* Badge */}
      <span
        className="inline-flex items-center gap-1 px-2.5 py-1 rounded-full"
        style={{ backgroundColor: config.bg, color: config.color, fontSize: 12 }}
      >
        <Icon size={13} strokeWidth={2} />
        {config.label}
      </span>

      {/* Title */}
      <h1
        className="font-bold"
        style={{ fontSize: 24, fontFamily: "'Cabinet Grotesk', sans-serif", color: "var(--color-text-primary)" }}
      >
        {data.title}
      </h1>

      {/* Subtitle */}
      <p style={{ fontSize: 13, color: "var(--color-text-muted)" }}>
        过去 14 天 · {evidenceCount} 条相关内容
      </p>

      {/* AI insight block */}
      <div
        className="rounded-xl p-4"
        style={{ backgroundColor: "var(--color-accent-soft)", border: "1px solid var(--color-border)" }}
      >
        <p style={{ fontSize: 14, color: "var(--color-text-secondary)" }}>
          {data.why_now}
        </p>
      </div>

      {/* Content */}
      {isConnection && connData ? (
        /* Dual columns for connection */
        <div className="grid grid-cols-2 gap-4">
          <div className="border-l-2 pl-4" style={{ borderColor: "#2563EB" }}>
            <h3 className="font-semibold mb-3" style={{ fontSize: 14, color: "var(--color-text-primary)" }}>
              {connData.group_a.topic}
            </h3>
            <div className="space-y-3">
              {connData.group_a.evidence.map((item, i) => (
                <div key={i}>
                  <span className="font-mono text-stone-400 block" style={{ fontSize: 11 }}>
                    {item.date}
                  </span>
                  <span style={{ fontSize: 13, color: "var(--color-text-secondary)" }}>
                    {item.title}
                  </span>
                </div>
              ))}
            </div>
          </div>
          <div className="border-l-2 pl-4" style={{ borderColor: "#2563EB" }}>
            <h3 className="font-semibold mb-3" style={{ fontSize: 14 }}>
              {connData.group_b.topic}
            </h3>
            <div className="space-y-3">
              {connData.group_b.evidence.map((item, i) => (
                <div key={i}>
                  <span className="font-mono text-stone-400 block" style={{ fontSize: 11 }}>
                    {item.date}
                  </span>
                  <span style={{ fontSize: 13, color: "var(--color-text-secondary)" }}>
                    {item.title}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>
      ) : evidenceData ? (
        /* Timeline for thread/obsession */
        <div className="border-l-2 pl-4 space-y-4" style={{ borderColor: "var(--color-border)" }}>
          {evidenceData.evidence.map((item, i) => (
            <div key={i} className="relative">
              <div
                className="absolute -left-[21px] top-1.5 w-2.5 h-2.5 rounded-full border-2"
                style={{ backgroundColor: "var(--color-surface)", borderColor: config.color }}
              />
              <span className="font-mono text-stone-400 block" style={{ fontSize: 11 }}>
                {item.date}
              </span>
              <span className="" style={{ fontSize: 13 }}>
                {item.title}
              </span>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
