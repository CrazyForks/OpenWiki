import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Payload emitted from Rust via `spotlight:content-ready`. */
interface SpotlightPayload {
  content_type: string;
  raw_text: string | null;
  image_path: string | null;
  source_app: string;
}

/** Content type icon mapping. */
const TYPE_ICON: Record<string, string> = {
  text: "📝",
  url: "🔗",
  image: "🖼️",
};

export default function SpotlightView() {
  const [payload, setPayload] = useState<SpotlightPayload | null>(null);
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Make html/body fully transparent so Tauri transparent window shows desktop
  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
  }, []);

  // Listen for content from Rust backend
  useEffect(() => {
    const unlisten = listen<SpotlightPayload>(
      "spotlight:content-ready",
      (event) => {
        setPayload(event.payload);
        setNote("");
        setSaving(false);
        setTimeout(() => inputRef.current?.focus(), 50);
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Auto-focus input when window gains focus.
  // (Blur/hide is handled by Rust on_window_event for reliability.)
  useEffect(() => {
    const unlisten = getCurrentWindow().onFocusChanged(
      ({ payload: focused }) => {
        if (focused) {
          inputRef.current?.focus();
        } else {
          // Reset state when hidden (Rust hides the window)
          setPayload(null);
          setNote("");
          setSaving(false);
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const hideWindow = useCallback(async () => {
    setPayload(null);
    setNote("");
    setSaving(false);
    try {
      await getCurrentWindow().hide();
    } catch (err) {
      console.error("Failed to hide window:", err);
    }
  }, []);

  const handleSave = useCallback(async () => {
    if (saving || !payload) return;

    setSaving(true);
    try {
      await invoke("save_spotlight_content", {
        contentType: payload.content_type,
        rawText: payload.raw_text,
        imagePath: payload.image_path,
        sourceApp: payload.source_app,
        userNote: note.trim(),
      });

      // Save success → hide immediately
      hideWindow();
    } catch (err) {
      console.error("Failed to save spotlight content:", err);
      setSaving(false);
    }
  }, [payload, note, saving, hideWindow]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.nativeEvent.isComposing) {
        e.preventDefault();
        handleSave();
      } else if (e.key === "Escape") {
        e.preventDefault();
        hideWindow();
      }
    },
    [handleSave, hideWindow]
  );

  const icon = payload ? (TYPE_ICON[payload.content_type] ?? "📋") : "📋";

  return (
    <div
      data-tauri-drag-region
      className="h-screen w-screen flex items-center select-none px-5 gap-3"
      style={{ background: "transparent" }}
    >
      {/* Icon */}
      {payload && (
        <span data-tauri-drag-region className="text-lg text-white/50 shrink-0">
          {icon}
        </span>
      )}

      {/* Input */}
      <input
        ref={inputRef}
        type="text"
        className="
          flex-1 bg-transparent text-lg
          text-white
          placeholder:text-white/40
          outline-none border-none
          min-w-0
        "
        placeholder={
          payload ? "添加备注，回车保存…" : "等待内容…"
        }
        value={note}
        onChange={(e) => setNote(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={saving || !payload}
        autoFocus
      />

      {/* Shortcut hints */}
      <div
        data-tauri-drag-region
        className="flex items-center gap-1 text-xs text-white/25 shrink-0"
      >
        <kbd className="px-1.5 py-0.5 rounded bg-white/10 font-mono">↵</kbd>
        <kbd className="px-1.5 py-0.5 rounded bg-white/10 font-mono ml-1">esc</kbd>
      </div>
    </div>
  );
}
