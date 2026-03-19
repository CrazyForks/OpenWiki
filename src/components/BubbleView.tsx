import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const DEFAULT_COUNTDOWN = 5;
const CIRCLE_SIZE = 48;
const CAPSULE_W = 320;
const CAPSULE_H = 48;

interface PendingCapture {
  content_type: string;
  preview: string;
  source_app: string;
  raw_text: string | null;
  image_path: string | null;
}

export default function BubbleView() {
  useEffect(() => {
    document.body.style.background = "transparent";
    document.documentElement.style.background = "transparent";
    const root = document.getElementById("root");
    if (root) root.style.background = "transparent";
  }, []);

  const [pending, setPending] = useState<PendingCapture | null>(null);
  const [countdownMax, setCountdownMax] = useState(DEFAULT_COUNTDOWN);
  const [countdown, setCountdown] = useState(DEFAULT_COUNTDOWN);
  const [saving, setSaving] = useState(false);
  const [bubbleStyle, setBubbleStyle] = useState<"circle" | "bar">("circle");
  const [expanded, setExpanded] = useState(false);
  const [memo, setMemo] = useState("");
  const [bubblePosition, setBubblePosition] = useState("bottom-right");
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const pendingRef = useRef<PendingCapture | null>(null);
  const appWindow = useRef(getCurrentWindow());
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => { pendingRef.current = pending; }, [pending]);

  const clearTimer = useCallback(() => {
    if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
  }, []);

  const closeWindow = useCallback(async () => {
    clearTimer();
    try { await appWindow.current.close(); } catch (e) { console.error("close failed:", e); }
  }, [clearTimer]);

  const dismiss = useCallback(async () => {
    const capture = pendingRef.current;
    clearTimer();
    try { await invoke("dismiss_capture", { imagePath: capture?.image_path ?? null }); } catch {}
    await closeWindow();
  }, [clearTimer, closeWindow]);

  const confirm = useCallback(async () => {
    const capture = pendingRef.current;
    if (!capture || saving) return;
    clearTimer();
    setSaving(true);
    try {
      await invoke("confirm_capture", {
        contentType: capture.content_type,
        preview: capture.preview,
        sourceApp: capture.source_app,
        rawText: capture.raw_text,
        imagePath: capture.image_path,
        userNote: memo.trim() || null,
      });
    } catch (e) { console.error("confirm failed:", e); }
    await closeWindow();
  }, [saving, clearTimer, closeWindow, memo]);

  // Expand circle → capsule with native window resize for IME support
  const expandToCapsule = useCallback(async () => {
    if (expanded || bubbleStyle !== "circle") return;
    clearTimer(); // pause countdown while typing
    setExpanded(true);
    // Resize native window taller to accommodate IME candidate window
    try {
      const win = appWindow.current;
      const size = await win.innerSize();
      // Increase height to 200px to give room for IME popup
      await win.setSize(new (await import("@tauri-apps/api/dpi")).LogicalSize(size.width / (await win.scaleFactor()), 200));
    } catch (e) {
      console.error("Failed to resize bubble window:", e);
    }
    setTimeout(() => inputRef.current?.focus(), 350); // focus after animation
  }, [expanded, bubbleStyle, clearTimer]);

  // On mount: fetch pending data + bubble style
  useEffect(() => {
    const init = async () => {
      try {
        const settings = await invoke<Record<string, string>>("get_settings");
        if (settings?.bubble_style === "bar") setBubbleStyle("bar");
        if (settings?.bubble_position) setBubblePosition(settings.bubble_position);
        if (settings?.countdown_seconds) {
          const secs = parseInt(settings.countdown_seconds, 10);
          if (secs >= 1 && secs <= 30) { setCountdownMax(secs); setCountdown(secs); }
        }
      } catch {}
      try {
        const data = await invoke<PendingCapture | null>("get_pending_capture");
        if (data) { setPending(data); }
      } catch (e) { console.error("get_pending_capture failed:", e); }
    };
    const timer = setTimeout(init, 100);
    return () => clearTimeout(timer);
  }, []);

  // Listen for events
  useEffect(() => {
    const unlisten = listen<PendingCapture>("capture:pending", (event) => {
      clearTimer(); setPending(event.payload); setCountdown(countdownMax);
      setExpanded(false); setMemo("");
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [clearTimer, countdownMax]);

  // Countdown (only when NOT expanded)
  useEffect(() => {
    if (!pending || expanded) return;
    timerRef.current = setInterval(() => {
      setCountdown((prev) => {
        if (prev <= 1) { setTimeout(() => dismiss(), 0); return 0; }
        return prev - 1;
      });
    }, 1000);
    return () => clearTimer();
  }, [pending, expanded, dismiss, clearTimer]);

  const progress = pending ? countdown / countdownMax : 0;
  const circumference = 2 * Math.PI * 16;

  if (!pending) {
    return <div style={{ background: "transparent" }} />;
  }

  // Determine circle alignment within the capsule-sized window
  // right → circle at right edge, expands left
  // left → circle at left edge, expands right
  // center → circle at center, expands both sides
  const isRight = bubblePosition.includes("right");
  const isLeft = bubblePosition.includes("left");

  // ─── Circle Mode (animated circle → capsule) ───
  if (bubbleStyle === "circle") {
    const isImage = pending.content_type === "image";

    // The container width animates from 48px to 320px
    const containerStyle: React.CSSProperties = {
      width: expanded ? CAPSULE_W : CIRCLE_SIZE,
      height: CAPSULE_H,
      borderRadius: CAPSULE_H / 2,
      background: "rgba(15, 15, 30, 0.88)",
      backdropFilter: "blur(20px)",
      WebkitBackdropFilter: "blur(20px)",
      boxShadow: [
        "0 4px 24px rgba(0, 0, 0, 0.45)",
        "0 0 12px rgba(99, 102, 241, 0.15)",
        "inset 0 1px 0 rgba(255, 255, 255, 0.1)",
        "inset 0 0 0 1px rgba(255, 255, 255, 0.08)",
      ].join(", "),
      transition: "width 0.3s cubic-bezier(0.4, 0, 0.2, 1)",
      overflow: "hidden",
      // Position within the 320x48 window:
      // right-aligned position → float the container to the right
      marginLeft: isRight ? "auto" : isLeft ? "0" : "auto",
      marginRight: isRight ? "0" : isLeft ? "auto" : "auto",
    };

    return (
      <div
        className="select-none"
        style={{
          width: CAPSULE_W,
          height: CAPSULE_H,
          background: "transparent",
          display: "flex",
          justifyContent: isRight ? "flex-end" : isLeft ? "flex-start" : "center",
        }}
      >
        <div style={containerStyle}>
          <div className="flex items-center h-full" style={{ minWidth: CAPSULE_W }}>
            {/* Left side: icon (always visible) */}
            <div
              className="flex-shrink-0 flex items-center justify-center cursor-pointer"
              style={{ width: CIRCLE_SIZE, height: CIRCLE_SIZE }}
              onClick={expanded ? undefined : expandToCapsule}
            >
              <div className="relative" style={{ width: 38, height: 38 }}>
                {/* Countdown ring */}
                {!expanded && (
                  <svg className="absolute inset-0 -rotate-90" width="38" height="38" viewBox="0 0 38 38">
                    <circle cx="19" cy="19" r="15" fill="none" stroke="rgba(255,255,255,0.06)" strokeWidth="2" />
                    <circle
                      cx="19" cy="19" r="15"
                      fill="none" stroke="url(#cg)" strokeWidth="2" strokeLinecap="round"
                      strokeDasharray={2 * Math.PI * 15}
                      strokeDashoffset={2 * Math.PI * 15 * (1 - progress)}
                      className="transition-all duration-1000 ease-linear"
                    />
                    <defs>
                      <linearGradient id="cg" x1="0" y1="0" x2="1" y2="1">
                        <stop offset="0%" stopColor="#818cf8" />
                        <stop offset="100%" stopColor="#c084fc" />
                      </linearGradient>
                    </defs>
                  </svg>
                )}
                {/* Icon center */}
                <div className="absolute inset-0 flex flex-col items-center justify-center">
                  <span className="text-sm leading-none">{isImage ? "📷" : "📋"}</span>
                  {!expanded && (
                    <span className="text-[9px] font-bold text-indigo-400 mt-0.5">{countdown}s</span>
                  )}
                </div>
              </div>
            </div>

            {/* Expanded content: input + actions */}
            <div
              className="flex items-center gap-2 flex-1 pr-3 min-w-0"
              style={{
                opacity: expanded ? 1 : 0,
                transition: "opacity 0.2s ease-out 0.15s",
                pointerEvents: expanded ? "auto" : "none",
              }}
            >
              <input
                ref={inputRef}
                type="text"
                value={memo}
                onChange={(e) => setMemo(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") confirm();
                  if (e.key === "Escape") dismiss();
                }}
                placeholder="输入备忘..."
                className="flex-1 bg-transparent text-[13px] text-white/90 placeholder-white/25
                           outline-none border-none min-w-0"
                style={{ caretColor: "#818cf8" }}
              />

              {/* Save */}
              <button
                onClick={confirm}
                disabled={saving}
                className="h-7 px-3 rounded-full text-[12px] font-medium
                           bg-indigo-500/20 hover:bg-indigo-500/35
                           text-indigo-300 hover:text-indigo-200
                           border border-indigo-400/15 hover:border-indigo-400/30
                           transition-all duration-150 cursor-pointer flex-shrink-0
                           flex items-center gap-1"
              >
                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M4.5 12.75l6 6 9-13.5" />
                </svg>
                {saving ? "..." : "保存"}
              </button>

              {/* Dismiss */}
              <button
                onClick={dismiss}
                className="w-6 h-6 rounded-full flex items-center justify-center
                           text-white/20 hover:text-red-400 hover:bg-red-500/15
                           transition-all duration-150 cursor-pointer flex-shrink-0"
              >
                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>

          {/* Dismiss X on the circle (non-expanded only) */}
          {!expanded && (
            <button
              onClick={(e) => { e.stopPropagation(); dismiss(); }}
              className="absolute rounded-full bg-red-500/80 hover:bg-red-500
                         flex items-center justify-center
                         opacity-0 hover:opacity-100 transition-opacity duration-200
                         shadow-lg cursor-pointer"
              style={{
                width: 16, height: 16,
                top: 0,
                right: isRight ? 0 : undefined,
                left: isLeft ? CIRCLE_SIZE - 16 : undefined,
              }}
            >
              <svg className="w-2 h-2 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
      </div>
    );
  }

  // ─── Bar Mode (full 340x72 bar) ───
  const previewText = pending.content_type === "image"
    ? "截图 / 图片"
    : (pending.preview || pending.raw_text || "").length > 20
      ? (pending.preview || pending.raw_text || "").slice(0, 20) + "..."
      : (pending.preview || pending.raw_text || "");

  const iconBg = pending.content_type === "image"
    ? "from-pink-500/20 to-rose-500/20"
    : "from-indigo-500/20 to-violet-500/20";
  const iconEmoji = pending.content_type === "image" ? "📷" : "📋";

  return (
    <div className="w-[340px] h-[72px] select-none" style={{ background: "transparent" }}>
      <div
        className="relative w-full h-full rounded-2xl overflow-hidden cursor-pointer group"
        onClick={confirm}
        style={{
          background: "rgba(15, 15, 30, 0.75)",
          backdropFilter: "blur(24px)",
          WebkitBackdropFilter: "blur(24px)",
          boxShadow: [
            "0 8px 32px rgba(0, 0, 0, 0.35)",
            "0 2px 8px rgba(99, 102, 241, 0.15)",
            "inset 0 1px 0 rgba(255, 255, 255, 0.08)",
            "inset 0 0 0 1px rgba(255, 255, 255, 0.06)",
          ].join(", "),
        }}
      >
        {/* Top shimmer */}
        <div className="absolute inset-x-0 top-0 h-[1px]" style={{
          background: "linear-gradient(90deg, transparent, rgba(255,255,255,0.15) 30%, rgba(139,92,246,0.3) 50%, rgba(255,255,255,0.15) 70%, transparent)",
        }} />

        {/* Bottom progress */}
        <div className="absolute inset-x-0 bottom-0 h-[2px] bg-white/[0.03]">
          <div className="h-full transition-all duration-1000 ease-linear" style={{
            width: `${progress * 100}%`,
            background: "linear-gradient(90deg, #818cf8, #a78bfa, #c084fc)",
          }} />
        </div>

        <div className="relative flex items-center gap-3 h-full px-4">
          {/* Icon + ring */}
          <div className="relative w-10 h-10 flex-shrink-0">
            <svg className="absolute inset-0 w-10 h-10 -rotate-90" viewBox="0 0 40 40">
              <circle cx="20" cy="20" r="16" fill="none" stroke="rgba(255,255,255,0.06)" strokeWidth="2" />
              <circle cx="20" cy="20" r="16" fill="none" stroke="url(#bar-grad)" strokeWidth="2" strokeLinecap="round"
                strokeDasharray={circumference} strokeDashoffset={circumference * (1 - progress)}
                className="transition-all duration-1000 ease-linear" />
              <defs>
                <linearGradient id="bar-grad" x1="0" y1="0" x2="1" y2="1">
                  <stop offset="0%" stopColor="#818cf8" />
                  <stop offset="50%" stopColor="#a78bfa" />
                  <stop offset="100%" stopColor="#c084fc" />
                </linearGradient>
              </defs>
            </svg>
            <div className={`absolute inset-[5px] rounded-full bg-gradient-to-br ${iconBg} flex items-center justify-center`}>
              <span className="text-sm">{iconEmoji}</span>
            </div>
          </div>

          {/* Text */}
          <div className="flex flex-col min-w-0 flex-1 gap-0.5">
            <div className="flex items-center gap-1.5">
              <span className="text-[10px] font-medium text-white/30 uppercase tracking-wider">{pending.source_app}</span>
              <span className="w-[3px] h-[3px] rounded-full bg-white/15" />
              <span className="text-[10px] text-indigo-400/70">{countdown}s</span>
            </div>
            <span className="text-[13px] font-medium text-white/85 leading-snug truncate">
              {saving ? "保存中..." : previewText}
            </span>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-1.5 flex-shrink-0">
            <button onClick={(e) => { e.stopPropagation(); confirm(); }} disabled={saving}
              className="w-8 h-8 rounded-xl flex items-center justify-center bg-indigo-500/15 hover:bg-indigo-500/30 border border-indigo-400/10 hover:border-indigo-400/25 transition-all duration-200 cursor-pointer">
              <svg className="w-3.5 h-3.5 text-indigo-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M4.5 12.75l6 6 9-13.5" />
              </svg>
            </button>
            <button onClick={(e) => { e.stopPropagation(); dismiss(); }}
              className="w-6 h-6 rounded-lg flex items-center justify-center text-white/20 hover:text-red-400 hover:bg-red-500/15 transition-all duration-200 cursor-pointer">
              <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
