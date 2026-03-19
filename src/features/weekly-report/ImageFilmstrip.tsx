import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { CapturedContent } from "../../types/content";

interface ImageFilmstripProps {
  items: CapturedContent[];
}

export function ImageFilmstrip({ items }: ImageFilmstripProps) {
  const [selectedIdx, setSelectedIdx] = useState<number | null>(null);

  if (items.length === 0) {
    return (
      <div className="text-center py-8 text-xs text-gray-400 dark:text-slate-500">
        本周没有图片内容
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {/* Section label */}
      <div className="flex items-center gap-1.5 px-0.5">
        <svg className="w-3 h-3 text-amber-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909M3.75 21h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v13.5A1.5 1.5 0 003.75 21z" />
        </svg>
        <span className="text-[11px] font-medium text-gray-500 dark:text-slate-400">
          {items.length} 张图片
        </span>
      </div>

      {/* Filmstrip — horizontal scroll grid */}
      <div className="overflow-x-auto scrollbar-hide -mx-3 px-3">
        <div className="flex gap-1.5" style={{ minWidth: "min-content" }}>
          {items.map((item, idx) => {
            const src = item.thumbnail_path
              ? convertFileSrc(item.thumbnail_path)
              : item.image_path
                ? convertFileSrc(item.image_path)
                : null;

            if (!src) return null;

            return (
              <motion.button
                key={item.id}
                initial={{ opacity: 0, scale: 0.9 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ duration: 0.15, delay: idx * 0.02 }}
                onClick={() => setSelectedIdx(selectedIdx === idx ? null : idx)}
                className={`flex-shrink-0 rounded-lg overflow-hidden border-2 transition-all duration-150 cursor-pointer
                  ${selectedIdx === idx
                    ? "border-blue-400 dark:border-blue-500 shadow-md"
                    : "border-transparent hover:border-gray-200 dark:hover:border-slate-600"
                  }`}
              >
                <img
                  src={src}
                  alt=""
                  className="w-16 h-16 object-cover"
                  loading="lazy"
                />
              </motion.button>
            );
          })}
        </div>
      </div>

      {/* Expanded preview — when an image is selected */}
      <AnimatePresence>
        {selectedIdx !== null && items[selectedIdx] && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden"
          >
            <div className="glass rounded-2xl overflow-hidden">
              <img
                src={
                  items[selectedIdx].image_path
                    ? convertFileSrc(items[selectedIdx].image_path!)
                    : items[selectedIdx].thumbnail_path
                      ? convertFileSrc(items[selectedIdx].thumbnail_path!)
                      : ""
                }
                alt=""
                className="w-full max-h-48 object-contain bg-gray-50 dark:bg-slate-900"
              />
              <div className="px-2.5 py-1.5 flex items-center justify-between">
                <span className="text-[10px] text-gray-400 dark:text-slate-500">
                  {items[selectedIdx].source_app}
                </span>
                <span className="text-[10px] text-gray-400 dark:text-slate-500">
                  {formatTime(items[selectedIdx].captured_at)}
                </span>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function formatTime(dateStr: string): string {
  const d = new Date(dateStr);
  return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}
