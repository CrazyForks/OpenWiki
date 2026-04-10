# OpenWiki

> Mac 桌面 AI 知识管理工具 — 自动捕获、智能整理、深度洞察

OpenWiki 在后台静默监听你的剪贴板，自动收集你浏览、复制的内容碎片，用 AI 将它们编织成结构化的个人知识库，并定期生成洞察报告，帮你看见自己的信息世界。

**隐私优先** — 所有数据存储在本地 SQLite，不上传任何云端。

## 📸 截图

> TODO: 添加应用截图

## 核心功能

### 📋 智能捕获
- 后台监听剪贴板，自动保存文本、图片、URL
- 识别来源应用（Safari / Chrome / 微信 / 备忘录…）
- **全局快捷键** `⌘⇧C` 呼出 Spotlight 捕获窗口，快速标注并保存
- 支持抓取微信公众号、X/Twitter 等 URL 的正文内容

### 📂 内容管理
- 按类型（文本 / 图片 / 链接）和时间范围过滤
- 全局搜索，跨内容和知识库同时检索
- 日历时间线视图，按天浏览历史
- 一键导出为 Markdown 文件

### 🧠 AI 知识库
- AI 自动将捕获内容编译为 Wiki 页面（概念、实体、主题）
- 知识图谱可视化，看见概念之间的关联
- **Ask 侧栏** — 向你的知识库提问，AI 基于你的内容回答
- 自动检测孤立页面、断裂链接等结构问题

### 📊 洞察报告
- 一键生成 AI 周报，汇总本周捕获内容
- **注意力分析** — 7 维度洞察你的信息习惯：
  - 一瞥总览 / 潜意识 / 遗忘墓地 / 盲区 / 热点 / 热力图 / 行动建议
- 对报告内容点赞或忽略，AI 学习你的偏好

### ⚙️ AI 提供商
- 支持 **Anthropic (Claude)** / **OpenAI** / **Google Gemini**
- API Key 或 OAuth 登录，两种接入方式
- 可为每个提供商选择不同模型

### 🖥 桌面体验
- 系统托盘常驻，关闭窗口不退出
- 全局快捷键 `⌘⇧Y` 唤起主窗口
- 深色 / 浅色 / 跟随系统主题
- MCP 协议集成，可连接 Claude Desktop

## 技术栈

| 层级 | 技术 |
|---|---|
| 框架 | Tauri 2 |
| 前端 | React 19 + TypeScript |
| 样式 | Tailwind CSS 4 |
| 状态 | Zustand |
| 动效 | Framer Motion |
| 后端 | Rust |
| 存储 | SQLite (本地) |

## 安装与开发

### 前置要求
- Node.js 18+
- Rust (最新 stable)
- macOS

### 开始

```bash
# 克隆仓库
git clone https://github.com/kdsz001/xiaoyun.git
cd xiaoyun

# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建应用
npm run tauri build
```

### AI 配置

复制环境变量模板并填入你的 API Key：

```bash
cp .env.example .env
```

也可以在应用内的 设置 → AI 提供商 中直接配置（支持 OAuth 登录）。

## Roadmap

- [ ] 多语言支持 (i18n)
- [ ] 云端同步
- [ ] 浏览器插件捕获
- [ ] 团队协作版本

## 参与贡献

欢迎贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解开发流程和规范。

## License

[MIT](LICENSE)
