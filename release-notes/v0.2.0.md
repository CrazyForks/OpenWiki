## ✨ What's New

- Added DeepSeek as a first-class AI provider with V4 series (V4 Pro / V4 Flash) — cheap and fast
- Added LM Studio as a built-in local model provider, sitting alongside Ollama
- Added latest OpenAI models: GPT-5.5, GPT-5.5 Pro, GPT-5.4 Nano
- Fixed Ollama hanging or returning empty content during deep analysis
- Improved local-model analysis timeout to 15 minutes to avoid false timeouts
- Insight reports now show the analyzed date range at the top
- Fixed the "Add to Knowledge" button still appearing on already-compiled items

## Installation (macOS)

1. Download the `.dmg` file that matches your Mac:
   - **Apple Silicon (M1/M2/M3/M4)**: `OpenWiki_0.2.0_aarch64.dmg`
   - **Intel Mac**: `OpenWiki_0.2.0_x64.dmg`
2. Double-click the `.dmg` and drag **OpenWiki** into the **Applications** folder.
3. **First launch** — because OpenWiki is not notarized with Apple:
   - **Right-click** OpenWiki in Applications → **Open** → confirm **Open** in the dialog, OR
   - Open **System Settings → Privacy & Security**, scroll to the "OpenWiki was blocked" notice, and click **Open Anyway**.
4. Once approved, double-click to launch normally. You'll only need to do this once.
5. Go to **Settings → AI** to configure your AI provider (Claude / OpenAI / Gemini).

---

## ✨ 本次更新

- 新增 DeepSeek 作为独立 AI 服务商，支持最新的 V4 系列（V4 Pro / V4 Flash），便宜速度又快
- 新增 LM Studio 本地模型支持，可在设置里直接选择，与 Ollama 并列
- 新增 GPT-5.5、GPT-5.5 Pro、GPT-5.4 Nano 等 OpenAI 最新模型
- 修复了 Ollama 跑深度分析时卡住、返回空结果的问题
- 优化了本地模型的分析等待时长，最长 15 分钟，不再"假超时"
- 洞察报告顶部新增数据时间范围显示（年-月-日 至 年-月-日）
- 修复了内容已入知识库后仍显示「入知识库」按钮的误导问题

## 安装说明（macOS）

1. 下载与你的 Mac 对应的 `.dmg` 文件：
   - **Apple Silicon（M1/M2/M3/M4）**：`OpenWiki_0.2.0_aarch64.dmg`
   - **Intel 芯片 Mac**：`OpenWiki_0.2.0_x64.dmg`
2. 双击 `.dmg`，将 **OpenWiki** 拖入 **Applications**（应用程序）文件夹。
3. **首次打开**（应用未经 Apple 公证，需手动放行一次）：
   - **右键点击**「应用程序」里的 OpenWiki → 选 **打开** → 在弹窗里再点 **打开**，或
   - 打开 **系统设置 → 隐私与安全性**，滚动到底部看到「OpenWiki 已被阻止」，点 **仍要打开**。
4. 放行之后，以后直接双击即可打开，不会再拦截。
5. 打开应用后进入 **设置 → AI**，填入 AI 服务商的 API Key（Claude / OpenAI / Gemini）。
