import { useState } from "react";
import { motion } from "framer-motion";
import type { CapturedContent } from "../../types/content";

interface TextContentListProps {
  items: CapturedContent[];
}

export function TextContentList({ items }: TextContentListProps) {
  if (items.length === 0) {
    return (
      <div className="text-center py-8 text-xs text-gray-400 dark:text-slate-500">
        本周没有文本内容
      </div>
    );
  }

  return (
    <div className="space-y-1.5">
      {/* Section label */}
      <div className="flex items-center gap-1.5 px-0.5">
        <svg className="w-3 h-3 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25H12" />
        </svg>
        <span className="text-[11px] font-medium text-gray-500 dark:text-slate-400">
          {items.length} 条文本
        </span>
      </div>

      {/* Text items */}
      <div className="space-y-1.5">
        {items.map((item, idx) => (
          <TextRow key={item.id} item={item} index={idx} />
        ))}
      </div>
    </div>
  );
}

function TextRow({ item, index }: { item: CapturedContent; index: number }) {
  const [expanded, setExpanded] = useState(false);
  const text = item.raw_text || "";
  const preview = text.length > 120 ? text.slice(0, 120) + "..." : text;

  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.15, delay: index * 0.02 }}
      className="glass rounded-2xl px-2.5 py-2
                 hover:shadow-sm transition-all cursor-pointer"
      onClick={() => setExpanded(!expanded)}
    >
      <p className={`text-xs leading-relaxed text-gray-600 dark:text-gray-300 ${expanded ? "" : "line-clamp-2"}`}>
        {expanded ? text : preview}
      </p>
      <div className="flex items-center gap-2 mt-1">
        <span className="text-[10px] text-gray-400 dark:text-slate-500">
          {item.source_app}
        </span>
        <span className="text-[10px] text-gray-300 dark:text-slate-600">
          {formatTime(item.captured_at)}
        </span>
        {text.length > 120 && (
          <span className="text-[10px] text-blue-400 dark:text-blue-500 ml-auto">
            {expanded ? "收起" : "展开"}
          </span>
        )}
      </div>
    </motion.div>
  );
}

function formatTime(dateStr: string): string {
  const d = new Date(dateStr);
  return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}
