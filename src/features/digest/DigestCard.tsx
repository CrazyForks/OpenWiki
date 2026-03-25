import type { CapturedContent } from "../../types/content";

interface DigestCardProps {
  content: CapturedContent;
}

function timeAgo(dateStr: string): string {
  const now = new Date();
  const then = new Date(dateStr);
  const diffMs = now.getTime() - then.getTime();
  const days = Math.floor(diffMs / (1000 * 60 * 60 * 24));
  if (days === 0) return "今天";
  if (days === 1) return "昨天";
  if (days < 30) return `${days} 天前`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months} 个月前`;
  return `${Math.floor(months / 12)} 年前`;
}

function typeIcon(type: string): string {
  switch (type) {
    case "image": return "📷";
    case "url": return "🔗";
    default: return "📝";
  }
}

export function DigestCard({ content }: DigestCardProps) {
  const fullText = content.raw_text || content.source_url || (content.image_path ? "[图片]" : "无内容");

  return (
    <div className="glass rounded-xl overflow-hidden flex flex-col h-full">
      {/* Meta — fixed top */}
      <div className="flex items-center justify-between px-4 pt-3 pb-2 flex-shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-xs">{typeIcon(content.content_type)}</span>
          <span className="text-[11px] text-gray-500 dark:text-slate-400 bg-gray-100/50 dark:bg-white/[0.06] px-2 py-0.5 rounded-full">
            {content.source_app}
          </span>
        </div>
        <span className="text-[11px] text-amber-500 dark:text-amber-400 italic">
          {timeAgo(content.captured_at)}保存
        </span>
      </div>

      {/* Content — scrollable */}
      <div className="flex-1 overflow-y-auto px-4 pb-3">
        <p className="text-sm text-gray-700 dark:text-gray-200 leading-relaxed whitespace-pre-wrap break-words">
          {fullText}
        </p>
        {content.content_type === "image" && content.image_path && (
          <div className="mt-3 rounded-lg overflow-hidden">
            <img
              src={`asset://localhost/${content.image_path}`}
              alt=""
              className="w-full max-h-48 object-cover rounded-lg"
            />
          </div>
        )}
      </div>
    </div>
  );
}
