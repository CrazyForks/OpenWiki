import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  useSettingsStore,
  MODELS_BY_PROVIDER,
  PROVIDER_LABELS,
  type AIProvider,
  type ThemeMode,
  type BubbleStyle,
  type BubblePosition,
  type DefaultAction,
} from "../../stores/settingsStore";

const BUBBLE_POSITION_OPTIONS: { value: BubblePosition; label: string; icon: string }[] = [
  { value: "bottom-right", label: "右下", icon: "↘" },
  { value: "bottom-center", label: "下方居中", icon: "↓" },
  { value: "bottom-left", label: "左下", icon: "↙" },
  { value: "top-right", label: "右上", icon: "↗" },
  { value: "top-center", label: "上方居中", icon: "↑" },
  { value: "top-left", label: "左上", icon: "↖" },
];

const THEME_OPTIONS: { value: ThemeMode; label: string; icon: string }[] = [
  { value: "light", label: "浅色", icon: "☀️" },
  { value: "dark", label: "深色", icon: "🌙" },
  { value: "system", label: "跟随系统", icon: "💻" },
];

export function SettingsView() {
  const {
    apiKey,
    provider,
    model,
    theme,
    captureEnabled,
    captureMode,
    bubbleStyle,
    bubblePosition,
    countdownDuration,
    sensitiveFilterEnabled,
    urlReadingEnabled,
    screenshotDir,
    totalItems,
    diskUsageMB,
    setApiKey,
    setProvider,
    setModel,
    setTheme,
    setCaptureEnabled,
    setCaptureMode,
    setBubbleStyle,
    setBubblePosition,
    setCountdownDuration,
    setSensitiveFilterEnabled,
    defaultAction,
    setDefaultAction,
    setUrlReadingEnabled,
    loadXReaderStatus,
  } = useSettingsStore();

  const [showApiKey, setShowApiKey] = useState(false);

  // MCP connection state per target
  type McpTargetId = "claude" | "openclaw";
  interface McpTargetState {
    connected: boolean;
    loading: boolean;
    message: string | null;
    error: string | null;
  }
  const [mcpStates, setMcpStates] = useState<Record<McpTargetId, McpTargetState>>({
    claude: { connected: false, loading: false, message: null, error: null },
    openclaw: { connected: false, loading: false, message: null, error: null },
  });
  const [summaryCopied, setSummaryCopied] = useState(false);
  const [mcpGlobalError, setMcpGlobalError] = useState<string | null>(null);

  const updateMcpTarget = (id: McpTargetId, update: Partial<McpTargetState>) => {
    setMcpStates((prev) => ({ ...prev, [id]: { ...prev[id], ...update } }));
  };

  const loadMcpStatus = useCallback(async () => {
    for (const target of ["claude", "openclaw"] as McpTargetId[]) {
      try {
        const status = await invoke<{ connected: boolean }>("get_mcp_status", { target });
        updateMcpTarget(target, { connected: status.connected });
      } catch {
        // silently fail — target may not be installed
      }
    }
  }, []);

  useEffect(() => {
    loadMcpStatus();
  }, [loadMcpStatus]);

  const handleConnectMcp = async (target: McpTargetId) => {
    updateMcpTarget(target, { loading: true, error: null, message: null });
    try {
      const msg = await invoke<string>("connect_mcp", { target });
      updateMcpTarget(target, { loading: false, message: msg, connected: true });
    } catch (e) {
      updateMcpTarget(target, { loading: false, error: typeof e === "string" ? e : String(e) });
    }
  };

  const handleDisconnectMcp = async (target: McpTargetId) => {
    updateMcpTarget(target, { loading: true, error: null, message: null });
    try {
      await invoke("disconnect_mcp", { target });
      updateMcpTarget(target, { loading: false, connected: false, message: "已断开连接。" });
    } catch (e) {
      updateMcpTarget(target, { loading: false, error: typeof e === "string" ? e : String(e) });
    }
  };

  const handleCopySummary = async () => {
    setMcpGlobalError(null);
    try {
      const summary = await invoke<string>("copy_content_summary");
      await writeText(summary);
      setSummaryCopied(true);
      setTimeout(() => setSummaryCopied(false), 2000);
    } catch (e) {
      setMcpGlobalError(typeof e === "string" ? e : String(e));
    }
  };

  const availableModels = MODELS_BY_PROVIDER[provider];

  useEffect(() => {
    loadXReaderStatus();
  }, [loadXReaderStatus]);

  return (
    <div className="max-w-2xl mx-auto p-6 space-y-6">
      {/* Appearance */}
      <section>
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-3 flex items-center gap-2">
          <span className="text-xl">🎨</span>
          外观
        </h2>
        <div className="glass rounded-2xl">
          <div className="p-4">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              主题模式
            </label>
            <div className="flex gap-2">
              {THEME_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  onClick={() => setTheme(opt.value)}
                  className={`
                    flex-1 flex items-center justify-center gap-1.5 px-3 py-2.5 text-sm font-medium rounded-lg border
                    transition-all duration-150
                    ${
                      theme === opt.value
                        ? "bg-gradient-to-r from-indigo-500/10 to-purple-500/10 dark:from-indigo-500/15 dark:to-purple-500/15 border-indigo-300/60 dark:border-indigo-500/30 text-indigo-700 dark:text-indigo-400 shadow-sm"
                        : "bg-white/50 dark:bg-white/[0.04] border-white/60 dark:border-white/[0.08] text-gray-600 dark:text-slate-300 hover:bg-white/80 dark:hover:bg-white/[0.08]"
                    }
                  `}
                >
                  <span>{opt.icon}</span>
                  <span>{opt.label}</span>
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* AI Configuration */}
      <section>
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-3 flex items-center gap-2">
          <span className="text-xl">🤖</span>
          AI 配置
        </h2>
        <div className="glass rounded-2xl divide-y divide-gray-100/50 dark:divide-white/[0.06]">
          {/* API Key */}
          <div className="p-4">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              API Key
            </label>
            <div className="relative">
              <input
                type={showApiKey ? "text" : "password"}
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="输入你的 API Key..."
                className="w-full px-3 py-2 pr-20 text-sm border border-white/60 dark:border-white/[0.1] rounded-xl
                           bg-white/50 dark:bg-white/[0.04] text-gray-800 dark:text-gray-200
                           placeholder-gray-400 dark:placeholder-slate-500
                           focus:bg-white/80 dark:focus:bg-white/[0.08] focus:border-indigo-400/60 dark:focus:border-indigo-500/40
                           focus:ring-1 focus:ring-indigo-400/30 dark:focus:ring-indigo-500/30 outline-none transition-all"
              />
              <button
                type="button"
                onClick={() => setShowApiKey(!showApiKey)}
                className="absolute right-2 top-1/2 -translate-y-1/2 px-2 py-1
                           text-xs text-gray-500 dark:text-slate-400 hover:text-gray-700 dark:hover:text-slate-200
                           bg-white/60 dark:bg-white/[0.08] hover:bg-white/80 dark:hover:bg-white/[0.12] rounded-lg transition-colors"
              >
                {showApiKey ? "隐藏" : "显示"}
              </button>
            </div>
            <p className="mt-1.5 text-xs text-gray-400 dark:text-slate-500">
              Key 将安全存储在本地，不会上传到任何服务器
            </p>
          </div>

          {/* AI Provider */}
          <div className="p-4">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              AI 服务商
            </label>
            <div className="flex gap-2">
              {(["anthropic", "openai", "openrouter"] as AIProvider[]).map((p) => (
                <button
                  key={p}
                  onClick={() => setProvider(p)}
                  className={`
                    flex-1 px-3 py-2 text-sm font-medium rounded-lg border
                    transition-all duration-150
                    ${
                      provider === p
                        ? "bg-gradient-to-r from-indigo-500/10 to-purple-500/10 dark:from-indigo-500/15 dark:to-purple-500/15 border-indigo-300/60 dark:border-indigo-500/30 text-indigo-700 dark:text-indigo-400 shadow-sm"
                        : "bg-white/50 dark:bg-white/[0.04] border-white/60 dark:border-white/[0.08] text-gray-600 dark:text-slate-300 hover:bg-white/80 dark:hover:bg-white/[0.08]"
                    }
                  `}
                >
                  {PROVIDER_LABELS[p]}
                </button>
              ))}
            </div>
            {provider === "openrouter" && (
              <p className="mt-1.5 text-xs text-gray-400 dark:text-slate-500">
                OpenRouter 支持多种模型，使用统一 API Key，
                <a
                  href="https://openrouter.ai/keys"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-indigo-500 dark:text-indigo-400 hover:underline"
                >
                  前往获取 Key
                </a>
              </p>
            )}
          </div>

          {/* Model Selection */}
          <div className="p-4">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              模型
            </label>
            <select
              value={model}
              onChange={(e) => setModel(e.target.value)}
              className="w-full px-3 py-2 text-sm border border-white/60 dark:border-white/[0.1] rounded-xl
                         bg-white/50 dark:bg-white/[0.04] text-gray-800 dark:text-gray-200
                         focus:bg-white/80 dark:focus:bg-white/[0.08] focus:border-indigo-400/60 dark:focus:border-indigo-500/40
                         focus:ring-1 focus:ring-indigo-400/30 dark:focus:ring-indigo-500/30 outline-none transition-all cursor-pointer"
            >
              {(() => {
                const hasGroups = availableModels.some((m) => m.group);
                if (!hasGroups) {
                  return availableModels.map((m) => (
                    <option key={m.id} value={m.id}>{m.label}</option>
                  ));
                }
                const groups: string[] = [];
                for (const m of availableModels) {
                  const g = m.group || "其他";
                  if (!groups.includes(g)) groups.push(g);
                }
                return groups.map((g) => (
                  <optgroup key={g} label={g}>
                    {availableModels
                      .filter((m) => (m.group || "其他") === g)
                      .map((m) => (
                        <option key={m.id} value={m.id}>
                          {m.free ? `🆓 ${m.label}` : m.label}
                        </option>
                      ))}
                  </optgroup>
                ));
              })()}
            </select>
            {availableModels.find((m) => m.id === model)?.free && (
              <p className="mt-1.5 text-xs text-green-600 dark:text-green-400">
                ✅ 当前模型免费，无需消耗额度
              </p>
            )}
          </div>
        </div>
      </section>

      {/* Capture Settings */}
      <section>
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-3 flex items-center gap-2">
          <span className="text-xl">📸</span>
          捕获设置
        </h2>
        <div className="glass rounded-2xl divide-y divide-gray-100/50 dark:divide-white/[0.06]">
          {/* Capture Toggle */}
          <div className="p-4 flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-gray-700 dark:text-gray-300">
                内容捕获
              </div>
              <div className="text-xs text-gray-400 dark:text-slate-500 mt-0.5">
                开启后将自动检测剪贴板和截图变化
              </div>
            </div>
            <button
              onClick={() => setCaptureEnabled(!captureEnabled)}
              className={`
                relative w-11 h-6 rounded-full transition-colors duration-200
                ${captureEnabled ? "bg-gradient-to-r from-indigo-500 to-purple-500" : "bg-gray-300 dark:bg-slate-600"}
              `}
            >
              <span
                className="absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform duration-200"
                style={{
                  transform: captureEnabled
                    ? "translateX(22px)"
                    : "translateX(2px)",
                }}
              />
            </button>
          </div>

          {/* Capture Mode */}
          <div className="p-4">
            <div className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              捕获模式
            </div>
            <div className="text-xs text-gray-400 dark:text-slate-500 mb-2.5">
              确认模式：复制后弹出悬浮球，点击才保存；自动模式：所有内容自动保存
            </div>
            <div className="flex gap-2">
              {([
                { value: "confirm" as const, label: "确认保存", icon: "🫧", desc: "悬浮球确认" },
                { value: "auto" as const, label: "自动保存", icon: "⚡", desc: "全部自动" },
              ]).map((opt) => (
                <button
                  key={opt.value}
                  onClick={() => setCaptureMode(opt.value)}
                  className={`
                    flex-1 flex items-center justify-center gap-1.5 px-3 py-2.5 text-sm font-medium rounded-lg border
                    transition-all duration-150
                    ${
                      captureMode === opt.value
                        ? "bg-gradient-to-r from-indigo-500/10 to-purple-500/10 dark:from-indigo-500/15 dark:to-purple-500/15 border-indigo-300/60 dark:border-indigo-500/30 text-indigo-700 dark:text-indigo-400 shadow-sm"
                        : "bg-white/50 dark:bg-white/[0.04] border-white/60 dark:border-white/[0.08] text-gray-600 dark:text-slate-300 hover:bg-white/80 dark:hover:bg-white/[0.08]"
                    }
                  `}
                >
                  <span>{opt.icon}</span>
                  <span>{opt.label}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Bubble Style (only when confirm mode) */}
          {captureMode === "confirm" && (
            <div className="p-4">
              <div className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                悬浮球样式
              </div>
              <div className="text-xs text-gray-400 dark:text-slate-500 mb-2.5">
                圆形更小巧，长条显示更多信息
              </div>
              <div className="flex gap-2">
                {([
                  { value: "circle" as BubbleStyle, label: "圆形", icon: "🫧" },
                  { value: "bar" as BubbleStyle, label: "长条", icon: "▬" },
                ] as const).map((opt) => (
                  <button
                    key={opt.value}
                    onClick={() => setBubbleStyle(opt.value)}
                    className={`
                      flex-1 flex items-center justify-center gap-1.5 px-3 py-2.5 text-sm font-medium rounded-lg border
                      transition-all duration-150
                      ${
                        bubbleStyle === opt.value
                          ? "bg-gradient-to-r from-indigo-500/10 to-purple-500/10 dark:from-indigo-500/15 dark:to-purple-500/15 border-indigo-300/60 dark:border-indigo-500/30 text-indigo-700 dark:text-indigo-400 shadow-sm"
                          : "bg-white/50 dark:bg-white/[0.04] border-white/60 dark:border-white/[0.08] text-gray-600 dark:text-slate-300 hover:bg-white/80 dark:hover:bg-white/[0.08]"
                      }
                    `}
                  >
                    <span>{opt.icon}</span>
                    <span>{opt.label}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Bubble Position (only when confirm mode) */}
          {captureMode === "confirm" && (
            <div className="p-4">
              <div className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                悬浮球位置
              </div>
              <div className="text-xs text-gray-400 dark:text-slate-500 mb-2.5">
                选择悬浮球弹出时的屏幕位置
              </div>
              <div className="grid grid-cols-3 gap-2">
                {BUBBLE_POSITION_OPTIONS.map((opt) => (
                  <button
                    key={opt.value}
                    onClick={() => setBubblePosition(opt.value)}
                    className={`
                      flex items-center justify-center gap-1 px-2 py-2 text-sm font-medium rounded-lg border
                      transition-all duration-150
                      ${
                        bubblePosition === opt.value
                          ? "bg-gradient-to-r from-indigo-500/10 to-purple-500/10 dark:from-indigo-500/15 dark:to-purple-500/15 border-indigo-300/60 dark:border-indigo-500/30 text-indigo-700 dark:text-indigo-400 shadow-sm"
                          : "bg-white/50 dark:bg-white/[0.04] border-white/60 dark:border-white/[0.08] text-gray-600 dark:text-slate-300 hover:bg-white/80 dark:hover:bg-white/[0.08]"
                      }
                    `}
                  >
                    <span>{opt.icon}</span>
                    <span>{opt.label}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Countdown Duration (only when confirm mode) */}
          {captureMode === "confirm" && (
            <div className="p-4">
              <div className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                倒计时秒数
              </div>
              <div className="text-xs text-gray-400 dark:text-slate-500 mb-2.5">
                悬浮球弹出后自动消失的等待时间
              </div>
              <div className="flex items-center gap-3">
                <input
                  type="range"
                  min={1}
                  max={15}
                  step={1}
                  value={countdownDuration}
                  onChange={(e) => setCountdownDuration(Number(e.target.value))}
                  className="flex-1 h-1.5 rounded-full appearance-none bg-gray-200 dark:bg-slate-600
                             accent-indigo-500 cursor-pointer
                             [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4
                             [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-gradient-to-r
                             [&::-webkit-slider-thumb]:from-indigo-500 [&::-webkit-slider-thumb]:to-purple-500
                             [&::-webkit-slider-thumb]:shadow-md [&::-webkit-slider-thumb]:cursor-pointer"
                />
                <span className="text-sm font-semibold text-indigo-600 dark:text-indigo-400 min-w-[3rem] text-center
                               bg-gradient-to-r from-indigo-500/10 to-purple-500/10 dark:from-indigo-500/15 dark:to-purple-500/15
                               px-2 py-1 rounded-lg border border-indigo-300/40 dark:border-indigo-500/20">
                  {countdownDuration}s
                </span>
              </div>
            </div>
          )}

          {/* Default Action (only when confirm mode) */}
          {captureMode === "confirm" && (
            <div className="p-4">
              <div className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                默认行为
              </div>
              <div className="text-xs text-gray-400 dark:text-slate-500 mb-2.5">
                倒计时结束后自动执行的操作。可随时用键盘覆盖：Enter 执行默认，Esc 执行相反
              </div>
              <div className="flex gap-2">
                {([
                  { value: "dismiss" as DefaultAction, label: "默认丢弃", icon: "🗑️", desc: "不操作就自动丢弃" },
                  { value: "save" as DefaultAction, label: "默认保存", icon: "💾", desc: "不操作就自动保存" },
                ]).map((opt) => (
                  <button
                    key={opt.value}
                    onClick={() => setDefaultAction(opt.value)}
                    className={`
                      flex-1 flex items-center justify-center gap-1.5 px-3 py-2.5 text-sm font-medium rounded-lg border
                      transition-all duration-150
                      ${
                        defaultAction === opt.value
                          ? "bg-gradient-to-r from-indigo-500/10 to-purple-500/10 dark:from-indigo-500/15 dark:to-purple-500/15 border-indigo-300/60 dark:border-indigo-500/30 text-indigo-700 dark:text-indigo-400 shadow-sm"
                          : "bg-white/50 dark:bg-white/[0.04] border-white/60 dark:border-white/[0.08] text-gray-600 dark:text-slate-300 hover:bg-white/80 dark:hover:bg-white/[0.08]"
                      }
                    `}
                  >
                    <span>{opt.icon}</span>
                    <span>{opt.label}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Sensitive Data Filter Toggle */}
          <div className="p-4 flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-gray-700 dark:text-gray-300">
                敏感数据过滤
              </div>
              <div className="text-xs text-gray-400 dark:text-slate-500 mt-0.5">
                自动过滤密码、私钥、API Key、Token 等敏感内容
              </div>
            </div>
            <button
              onClick={() => setSensitiveFilterEnabled(!sensitiveFilterEnabled)}
              className={`
                relative w-11 h-6 rounded-full transition-colors duration-200
                ${sensitiveFilterEnabled ? "bg-amber-500" : "bg-gray-300 dark:bg-slate-600"}
              `}
            >
              <span
                className="absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform duration-200"
                style={{
                  transform: sensitiveFilterEnabled
                    ? "translateX(22px)"
                    : "translateX(2px)",
                }}
              />
            </button>
          </div>

          {/* URL Reading Toggle */}
          <div className="p-4 flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-gray-700 dark:text-gray-300">
                链接内容读取
              </div>
              <div className="text-xs text-gray-400 dark:text-slate-500 mt-0.5">
                复制链接时自动获取网页正文，提升周报分析质量
              </div>
            </div>
            <button
              onClick={() => setUrlReadingEnabled(!urlReadingEnabled)}
              className={`
                relative w-11 h-6 rounded-full transition-colors duration-200
                ${urlReadingEnabled ? "bg-green-500" : "bg-gray-300 dark:bg-slate-600"}
              `}
            >
              <span
                className="absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform duration-200"
                style={{
                  transform: urlReadingEnabled
                    ? "translateX(22px)"
                    : "translateX(2px)",
                }}
              />
            </button>
          </div>

          {/* x-reader 状态提示 - 自动安装 */}
          {urlReadingEnabled && (
            <div className="mx-4 mb-4 p-3 bg-indigo-500/8 dark:bg-indigo-500/10 rounded-xl border border-indigo-200/50 dark:border-indigo-500/20">
              <div className="flex items-start gap-2">
                <span className="text-base">⚡️</span>
                <div className="flex-1">
                  <div className="text-sm font-medium text-blue-800 dark:text-blue-200">
                    增强内容读取
                  </div>
                  <div className="text-xs text-blue-600 dark:text-blue-400 mt-1">
                    自动支持微信公众号、X/Twitter、YouTube、B站、小红书等内容读取
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Screenshot Directory */}
          <div className="p-4">
            <div className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              截图存储目录
            </div>
            <div
              className="px-3 py-2 text-xs text-gray-500 dark:text-slate-400 bg-white/40 dark:bg-white/[0.04] rounded-xl
                         border border-white/50 dark:border-white/[0.06] font-mono break-all"
            >
              {screenshotDir}
            </div>
          </div>
        </div>
      </section>

      {/* AI Assistant Connection (MCP) */}
      <section>
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-3 flex items-center gap-2">
          <span className="text-xl">🔗</span>
          AI 助理连接
        </h2>
        <div className="glass rounded-2xl divide-y divide-gray-100/50 dark:divide-white/[0.06]">
          {/* MCP Connections */}
          {([
            { id: "claude" as McpTargetId, name: "Claude Desktop", hint: "在 Claude 中问" },
          ]).map((t) => {
            const s = mcpStates[t.id];
            return (
              <div key={t.id} className="p-4">
                <div className="flex items-center justify-between mb-2">
                  <div>
                    <div className="text-sm font-medium text-gray-700 dark:text-gray-300">
                      {t.name}
                    </div>
                    <div className="text-xs text-gray-400 dark:text-slate-500 mt-0.5">
                      {s.connected
                        ? `已连接 — ${t.name} 可以读取你保存的内容`
                        : `未连接 — 一键让 ${t.name} 读取你的数据`}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className={`w-2 h-2 rounded-full ${s.connected ? "bg-green-500" : "bg-gray-300 dark:bg-slate-600"}`} />
                    <span className="text-xs text-gray-500 dark:text-slate-400">
                      {s.connected ? "已连接" : "未连接"}
                    </span>
                  </div>
                </div>

                {s.connected ? (
                  <button
                    onClick={() => handleDisconnectMcp(t.id)}
                    disabled={s.loading}
                    className="w-full py-2 text-sm font-medium rounded-lg border
                               text-red-500 dark:text-red-400
                               border-red-200/50 dark:border-red-500/20
                               bg-red-50/50 dark:bg-red-500/[0.06]
                               hover:bg-red-100/50 dark:hover:bg-red-500/[0.12]
                               disabled:opacity-50 transition-colors"
                  >
                    {s.loading ? "处理中..." : "断开连接"}
                  </button>
                ) : (
                  <button
                    onClick={() => handleConnectMcp(t.id)}
                    disabled={s.loading}
                    className="w-full py-2 text-sm font-medium rounded-lg border
                               text-indigo-600 dark:text-indigo-400
                               border-indigo-200/50 dark:border-indigo-500/20
                               bg-indigo-50/50 dark:bg-indigo-500/[0.06]
                               hover:bg-indigo-100/50 dark:hover:bg-indigo-500/[0.12]
                               disabled:opacity-50 transition-colors"
                  >
                    {s.loading ? "连接中..." : `连接 ${t.name}`}
                  </button>
                )}

                {s.message && (
                  <p className="mt-2 text-xs text-green-600 dark:text-green-400">{s.message}</p>
                )}
                {s.error && (
                  <p className="mt-2 text-xs text-red-500 dark:text-red-400">{s.error}</p>
                )}

                {s.connected && (
                  <div className="mt-3 p-2.5 bg-indigo-500/5 dark:bg-indigo-500/10 rounded-lg">
                    <p className="text-xs text-gray-500 dark:text-slate-400 mb-1">{t.hint}：</p>
                    <p className="text-xs text-indigo-600 dark:text-indigo-400 italic">"查看我最近保存的内容"</p>
                    <p className="text-xs text-indigo-600 dark:text-indigo-400 italic">"帮我整理这周收藏的文章"</p>
                  </div>
                )}
              </div>
            );
          })}

          {/* Copy Summary for ChatGPT/Gemini */}
          <div className="p-4">
            <div className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              ChatGPT / Gemini
            </div>
            <div className="text-xs text-gray-400 dark:text-slate-500 mb-2.5">
              复制最近 7 天内容摘要，粘贴到任意 AI 对话中
            </div>
            <button
              onClick={handleCopySummary}
              className="w-full py-2 text-sm font-medium rounded-lg border
                         text-gray-600 dark:text-slate-300
                         border-white/60 dark:border-white/[0.08]
                         bg-white/50 dark:bg-white/[0.04]
                         hover:bg-white/80 dark:hover:bg-white/[0.08]
                         transition-colors"
            >
              {summaryCopied ? "✓ 已复制到剪贴板" : "复制最近内容摘要"}
            </button>
            {mcpGlobalError && (
              <p className="mt-2 text-xs text-red-500 dark:text-red-400">{mcpGlobalError}</p>
            )}
          </div>
        </div>
      </section>

      {/* Storage Info */}
      <section>
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-3 flex items-center gap-2">
          <span className="text-xl">💾</span>
          存储信息
        </h2>
        <div className="glass rounded-2xl">
          <div className="p-4 grid grid-cols-2 gap-4">
            <div className="text-center p-3 bg-gradient-to-br from-indigo-500/10 to-purple-500/10 dark:from-indigo-500/10 dark:to-purple-500/10 rounded-xl border border-white/40 dark:border-white/[0.06]">
              <div className="text-2xl font-bold bg-gradient-to-r from-indigo-600 to-purple-600 dark:from-indigo-400 dark:to-purple-400 bg-clip-text text-transparent">
                {totalItems}
              </div>
              <div className="text-xs text-gray-500 dark:text-slate-400 mt-1">已保存内容</div>
            </div>
            <div className="text-center p-3 bg-gradient-to-br from-indigo-500/10 to-purple-500/10 dark:from-indigo-500/10 dark:to-purple-500/10 rounded-xl border border-white/40 dark:border-white/[0.06]">
              <div className="text-2xl font-bold bg-gradient-to-r from-indigo-600 to-purple-600 dark:from-indigo-400 dark:to-purple-400 bg-clip-text text-transparent">
                {diskUsageMB.toFixed(1)} MB
              </div>
              <div className="text-xs text-gray-500 dark:text-slate-400 mt-1">磁盘占用</div>
            </div>
          </div>
        </div>
      </section>

      {/* Version Info */}
      <div className="text-center pb-4">
        <p className="text-xs text-gray-400 dark:text-slate-600">小云 v0.1.0</p>
      </div>
    </div>
  );
}
