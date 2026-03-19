import { useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { CapturedContent } from "../../types/content";

interface ContentPreviewPanelProps {
  items: CapturedContent[];
  onClose: () => void;
}

/**
 * Slide-up floating panel that displays content items
 * related to a report section. Shows inline instead of
 * navigating away to the content tab.
 */
export function ContentPreviewPanel({ items, onClose }: ContentPreviewPanelProps) {
  // Close on Escape
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [onClose]);

  if (items.length === 0) return null;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 z-50 flex items-end justify-center"
        onClick={onClose}
      >
        {/* Backdrop */}
        <div className="absolute inset-0 bg-black/30 backdrop-blur-[2px]" />

        {/* Panel */}
        <motion.div
          initial={{ y: "100%" }}
          animate={{ y: 0 }}
          exit={{ y: "100%" }}
          transition={{ type: "spring", damping: 28, stiffness: 300 }}
          className="relative w-full max-h-[70vh] glass rounded-t-2xl shadow-2xl overflow-hidden flex flex-col"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Handle bar + header */}
          <div className="flex-shrink-0 px-4 pt-3 pb-2">
            {/* Drag handle */}
            <div className="w-8 h-1 rounded-full bg-gray-300 dark:bg-slate-600 mx-auto mb-3" />

            <div className="flex items-center justify-between">
              <h3 className="text-[13px] font-bold text-gray-900 dark:text-gray-100">
                相关内容
                <span className="ml-1.5 text-[11px] font-normal text-gray-400 dark:text-slate-500">
                  {items.length} 条
                </span>
              </h3>
              <button
                onClick={onClose}
                className="w-6 h-6 rounded-full bg-white/50 dark:bg-white/[0.06] flex items-center justify-center
                           text-gray-400 dark:text-slate-500 hover:bg-gray-200 dark:hover:bg-slate-600
                           transition-colors cursor-pointer"
              >
                <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>

          {/* Scrollable content list */}
          <div className="flex-1 overflow-y-auto px-4 pb-6 space-y-2">
            {items.map((item) => (
              <ContentPreviewItem key={item.id} content={item} />
            ))}
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
}

/* ── Individual content preview item ── */

function ContentPreviewItem({ content }: { content: CapturedContent }) {
  const typeConfig: Record<string, { icon: string; label: string }> = {
    image: { icon: "🖼️", label: "图片" },
    url: { icon: "🔗", label: "链接" },
    text: { icon: "📝", label: "文本" },
    mixed: { icon: "📎", label: "混合" },
  };
  const { icon, label } = typeConfig[content.content_type] || typeConfig.text;

  const isUrl = content.content_type === "url";
  const hasSourceUrl = isUrl && !!content.source_url;
  const hasFetchedText = hasSourceUrl && content.raw_text !== content.source_url;

  const imageSrc =
    content.content_type === "image"
      ? content.thumbnail_path
        ? convertFileSrc(content.thumbnail_path)
        : content.image_path
          ? convertFileSrc(content.image_path)
          : null
      : null;

  const timeStr = formatTime(content.captured_at);

  return (
    <div className="rounded-xl bg-white/40 dark:bg-slate-800/60 p-3">
      <div className="flex items-start gap-2.5">
        {/* Type badge */}
        <div className="w-7 h-7 rounded-lg glass flex items-center justify-center flex-shrink-0 shadow-sm">
          <span className="text-sm">{icon}</span>
        </div>

        <div className="flex-1 min-w-0">
          {/* Image */}
          {imageSrc && (
            <img
              src={imageSrc}
              alt="Preview"
              className="max-w-full max-h-40 rounded-lg object-cover mb-2 "
              loading="lazy"
            />
          )}

          {/* URL with fetched text */}
          {isUrl && hasFetchedText && (
            <>
              <p className="text-[13px] text-gray-700 dark:text-gray-200 leading-relaxed line-clamp-4">
                {content.raw_text}
              </p>
              <a
                href={content.source_url}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1 mt-1.5 text-[11px] text-blue-500 hover:text-blue-600 transition-colors"
              >
                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
                </svg>
                {extractDomain(content.source_url!)}
              </a>
            </>
          )}

          {/* URL without fetched text — just show the URL */}
          {isUrl && !hasFetchedText && content.source_url && (
            <a
              href={content.source_url}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[13px] text-blue-500 hover:text-blue-600 break-all transition-colors"
            >
              {content.source_url}
            </a>
          )}

          {/* Plain text */}
          {!isUrl && content.raw_text && (
            <p className="text-[13px] text-gray-700 dark:text-gray-200 leading-relaxed line-clamp-6">
              {content.raw_text}
            </p>
          )}

          {/* Meta footer */}
          <div className="flex items-center gap-2 mt-2 text-[10px] text-gray-400 dark:text-slate-500">
            <span>{timeStr}</span>
            <span>·</span>
            <span>{content.source_app}</span>
            <span>·</span>
            <span>{label}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ── Helpers ── */

function formatTime(dateStr: string): string {
  const d = new Date(dateStr);
  return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

function extractDomain(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
