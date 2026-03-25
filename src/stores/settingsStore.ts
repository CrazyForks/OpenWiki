import { create } from "zustand";
import { getSettings, updateSetting, checkXReaderStatus, type XReaderStatus } from "../services/settingsService";

export type AIProvider = "anthropic" | "openai" | "openrouter";

export interface AIModelOption {
  id: string;
  label: string;
  free?: boolean;
  group?: string;
}

export const MODELS_BY_PROVIDER: Record<AIProvider, AIModelOption[]> = {
  anthropic: [
    { id: "claude-sonnet-4-20250514", label: "Claude Sonnet 4" },
    { id: "claude-opus-4-20250514", label: "Claude Opus 4" },
    { id: "claude-3-5-haiku-20241022", label: "Claude 3.5 Haiku" },
  ],
  openai: [
    { id: "gpt-4o", label: "GPT-4o" },
    { id: "gpt-4o-mini", label: "GPT-4o Mini" },
    { id: "gpt-4-turbo", label: "GPT-4 Turbo" },
  ],
  openrouter: [
    // ── 免费模型 ──
    { id: "google/gemini-2.5-flash:free", label: "Gemini 2.5 Flash", free: true, group: "免费模型" },
    { id: "google/gemini-2.5-flash-lite:free", label: "Gemini 2.5 Flash Lite", free: true, group: "免费模型" },
    { id: "google/gemini-3-flash-preview:free", label: "Gemini 3 Flash Preview", free: true, group: "免费模型" },
    { id: "deepseek/deepseek-chat-v3-0324:free", label: "DeepSeek V3", free: true, group: "免费模型" },
    { id: "deepseek/deepseek-chat-v3.1:free", label: "DeepSeek V3.1", free: true, group: "免费模型" },
    { id: "x-ai/grok-4.1-fast:free", label: "Grok 4.1 Fast", free: true, group: "免费模型" },
    { id: "minimax/minimax-m2.1:free", label: "MiniMax M2.1", free: true, group: "免费模型" },
    { id: "openrouter/free", label: "自动选择免费模型", free: true, group: "免费模型" },
    // ── Anthropic ──
    { id: "anthropic/claude-opus-4.5", label: "Claude Opus 4.5", group: "Anthropic" },
    { id: "anthropic/claude-sonnet-4", label: "Claude Sonnet 4", group: "Anthropic" },
    { id: "anthropic/claude-3.5-haiku", label: "Claude 3.5 Haiku", group: "Anthropic" },
    // ── OpenAI ──
    { id: "openai/gpt-5.2", label: "GPT-5.2", group: "OpenAI" },
    { id: "openai/gpt-5.1", label: "GPT-5.1", group: "OpenAI" },
    { id: "openai/gpt-4o", label: "GPT-4o", group: "OpenAI" },
    { id: "openai/gpt-4o-mini", label: "GPT-4o Mini", group: "OpenAI" },
    // ── Google ──
    { id: "google/gemini-3-pro-preview", label: "Gemini 3 Pro Preview", group: "Google" },
    { id: "google/gemini-3-flash-preview", label: "Gemini 3 Flash Preview", group: "Google" },
    { id: "google/gemini-2.5-pro-preview", label: "Gemini 2.5 Pro", group: "Google" },
    { id: "google/gemini-2.5-flash", label: "Gemini 2.5 Flash", group: "Google" },
    // ── DeepSeek ──
    { id: "deepseek/deepseek-v3.2", label: "DeepSeek V3.2", group: "DeepSeek" },
    { id: "deepseek/deepseek-r1", label: "DeepSeek R1", group: "DeepSeek" },
    // ── xAI ──
    { id: "x-ai/grok-4.1-fast", label: "Grok 4.1 Fast", group: "xAI" },
    // ── Moonshot ──
    { id: "moonshotai/kimi-k2.5", label: "Kimi K2.5", group: "Moonshot" },
    // ── Meta ──
    { id: "meta-llama/llama-4-maverick", label: "Llama 4 Maverick", group: "Meta" },
    // ── Qwen ──
    { id: "qwen/qwen3-coder-next", label: "Qwen3 Coder Next", group: "Qwen" },
    { id: "qwen/qwen3-235b-a22b", label: "Qwen3 235B", group: "Qwen" },
    // ── Mistral ──
    { id: "mistralai/mistral-large-2512", label: "Mistral Large 3", group: "Mistral" },
  ],
};

export const PROVIDER_LABELS: Record<AIProvider, string> = {
  anthropic: "Anthropic",
  openai: "OpenAI",
  openrouter: "OpenRouter",
};

const VALID_PROVIDERS: AIProvider[] = ["anthropic", "openai", "openrouter"];

export type CaptureMode = "auto" | "confirm";
export type BubbleStyle = "circle" | "bar";
export type BubblePosition = "bottom-right" | "bottom-center" | "bottom-left" | "top-right" | "top-center" | "top-left";
export type DefaultAction = "save" | "dismiss";
export type ThemeMode = "light" | "dark" | "system";

const VALID_BUBBLE_POSITIONS: BubblePosition[] = [
  "bottom-right", "bottom-center", "bottom-left",
  "top-right", "top-center", "top-left",
];

function applyTheme(theme: ThemeMode) {
  const isDark =
    theme === "dark" ||
    (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", isDark);
}

// Patterns for detecting sensitive data (passwords, private keys, API keys, tokens, secrets)
export const SENSITIVE_PATTERNS: RegExp[] = [
  // API Keys & tokens (generic)
  /(?:api[_-]?key|apikey|access[_-]?token|auth[_-]?token|bearer)\s*[:=]\s*['"]?[A-Za-z0-9_\-./+]{16,}/i,
  // AWS keys
  /AKIA[0-9A-Z]{16}/,
  // GitHub tokens
  /gh[ps]_[A-Za-z0-9_]{36,}/,
  // Slack tokens
  /xox[bpras]-[A-Za-z0-9-]{10,}/,
  // Private keys (PEM)
  /-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----/,
  // SSH private keys
  /-----BEGIN\s+OPENSSH\s+PRIVATE\s+KEY-----/,
  // Password patterns
  /(?:password|passwd|pwd)\s*[:=]\s*['"]?.{4,}/i,
  // Secret patterns
  /(?:secret|client[_-]?secret)\s*[:=]\s*['"]?[A-Za-z0-9_\-./+]{8,}/i,
  // JWT tokens
  /eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}/,
  // OpenAI keys
  /sk-[A-Za-z0-9]{20,}/,
  // Anthropic keys
  /sk-ant-[A-Za-z0-9_-]{20,}/,
];

export function containsSensitiveData(text: string): boolean {
  return SENSITIVE_PATTERNS.some((pattern) => pattern.test(text));
}

interface SettingsState {
  apiKey: string;
  provider: AIProvider;
  model: string;
  theme: ThemeMode;
  captureEnabled: boolean;
  captureMode: CaptureMode;
  bubbleStyle: BubbleStyle;
  bubblePosition: BubblePosition;
  defaultAction: DefaultAction;
  sensitiveFilterEnabled: boolean;
  urlReadingEnabled: boolean;
  countdownDuration: number;
  screenshotDir: string;
  totalItems: number;
  diskUsageMB: number;
  isLoaded: boolean;
  xreaderStatus: XReaderStatus | null;

  loadFromDB: () => Promise<void>;
  setApiKey: (key: string) => void;
  setProvider: (provider: AIProvider) => void;
  setModel: (model: string) => void;
  setTheme: (theme: ThemeMode) => void;
  setCaptureEnabled: (enabled: boolean) => void;
  setCaptureMode: (mode: CaptureMode) => void;
  setBubbleStyle: (style: BubbleStyle) => void;
  setBubblePosition: (position: BubblePosition) => void;
  setDefaultAction: (action: DefaultAction) => void;
  setSensitiveFilterEnabled: (enabled: boolean) => void;
  setUrlReadingEnabled: (enabled: boolean) => void;
  setCountdownDuration: (seconds: number) => void;
  setScreenshotDir: (dir: string) => void;
  setStorageInfo: (totalItems: number, diskUsageMB: number) => void;
  loadXReaderStatus: () => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  apiKey: "",
  provider: "anthropic",
  model: "claude-sonnet-4-20250514",
  theme: "system",
  captureEnabled: true,
  captureMode: "confirm" as CaptureMode,
  bubbleStyle: "circle" as BubbleStyle,
  bubblePosition: "bottom-right" as BubblePosition,
  defaultAction: "dismiss" as DefaultAction,
  sensitiveFilterEnabled: false,
  urlReadingEnabled: true,
  countdownDuration: 5,
  screenshotDir: "~/Library/Application Support/com.xiaoyun.app/screenshots",
  totalItems: 0,
  diskUsageMB: 0,
  isLoaded: false,
  xreaderStatus: null,

  loadFromDB: async () => {
    try {
      const settings = await getSettings();

      const provider = VALID_PROVIDERS.includes(settings.ai_provider as AIProvider)
        ? (settings.ai_provider as AIProvider)
        : "anthropic";

      const model = settings.ai_model || MODELS_BY_PROVIDER[provider][0].id;

      const theme = (["light", "dark", "system"].includes(settings.theme)
        ? settings.theme
        : "system") as ThemeMode;

      applyTheme(theme);

      set({
        apiKey: settings.ai_api_key || "",
        provider,
        model,
        theme,
        captureEnabled: settings.capture_enabled !== "false",
        captureMode: (settings.capture_mode === "auto" ? "auto" : "confirm") as CaptureMode,
        bubbleStyle: (settings.bubble_style === "bar" ? "bar" : "circle") as BubbleStyle,
        bubblePosition: (VALID_BUBBLE_POSITIONS.includes(settings.bubble_position as BubblePosition)
          ? settings.bubble_position
          : "bottom-right") as BubblePosition,
        defaultAction: (settings.default_action === "save" ? "save" : "dismiss") as DefaultAction,
        sensitiveFilterEnabled: settings.sensitive_filter_enabled === "true",
        urlReadingEnabled: settings.url_reading_enabled !== "false",
        countdownDuration: parseInt(settings.countdown_seconds || "5", 10),
        screenshotDir:
          settings.screenshot_dir ||
          "~/Library/Application Support/com.xiaoyun.app/screenshots",
        isLoaded: true,
      });
    } catch (e) {
      console.error("Failed to load settings from DB:", e);
      applyTheme("system");
      set({ isLoaded: true });
    }
  },

  setApiKey: (key) => {
    set({ apiKey: key });
    updateSetting("ai_api_key", key).catch((e) =>
      console.error("Failed to save api key:", e)
    );
  },

  setProvider: (provider) => {
    const firstModel = MODELS_BY_PROVIDER[provider][0].id;
    set({ provider, model: firstModel });
    updateSetting("ai_provider", provider).catch((e) =>
      console.error("Failed to save provider:", e)
    );
    updateSetting("ai_model", firstModel).catch((e) =>
      console.error("Failed to save model:", e)
    );
  },

  setModel: (model) => {
    set({ model });
    updateSetting("ai_model", model).catch((e) =>
      console.error("Failed to save model:", e)
    );
  },

  setTheme: (theme) => {
    set({ theme });
    applyTheme(theme);
    updateSetting("theme", theme).catch((e) =>
      console.error("Failed to save theme:", e)
    );
  },

  setCaptureEnabled: (enabled) => {
    set({ captureEnabled: enabled });
    updateSetting("capture_enabled", String(enabled)).catch((e) =>
      console.error("Failed to save capture_enabled:", e)
    );
  },

  setCaptureMode: (mode) => {
    set({ captureMode: mode });
    updateSetting("capture_mode", mode).catch((e) =>
      console.error("Failed to save capture_mode:", e)
    );
  },

  setBubbleStyle: (style) => {
    set({ bubbleStyle: style });
    updateSetting("bubble_style", style).catch((e) =>
      console.error("Failed to save bubble_style:", e)
    );
  },

  setBubblePosition: (position) => {
    set({ bubblePosition: position });
    updateSetting("bubble_position", position).catch((e) =>
      console.error("Failed to save bubble_position:", e)
    );
  },

  setDefaultAction: (action) => {
    set({ defaultAction: action });
    updateSetting("default_action", action).catch((e) =>
      console.error("Failed to save default_action:", e)
    );
  },

  setSensitiveFilterEnabled: (enabled) => {
    set({ sensitiveFilterEnabled: enabled });
    updateSetting("sensitive_filter_enabled", String(enabled)).catch((e) =>
      console.error("Failed to save sensitive_filter_enabled:", e)
    );
  },

  setUrlReadingEnabled: (enabled) => {
    set({ urlReadingEnabled: enabled });
    updateSetting("url_reading_enabled", String(enabled)).catch((e) =>
      console.error("Failed to save url_reading_enabled:", e)
    );
  },

  setCountdownDuration: (seconds) => {
    set({ countdownDuration: seconds });
    updateSetting("countdown_seconds", String(seconds)).catch((e) =>
      console.error("Failed to save countdown_seconds:", e)
    );
  },

  setScreenshotDir: (dir) => set({ screenshotDir: dir }),
  setStorageInfo: (totalItems, diskUsageMB) =>
    set({ totalItems, diskUsageMB }),

  loadXReaderStatus: async () => {
    try {
      const status = await checkXReaderStatus();
      set({ xreaderStatus: status });
    } catch (e) {
      console.error("Failed to load x-reader status:", e);
    }
  },
}));
