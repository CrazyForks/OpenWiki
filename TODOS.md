# TODOS

## Design Debt (from /plan-design-review 2026-03-24)

### TODO-1: 创建 DESIGN.md 设计系统
- **What:** 跑 /design-consultation 生成正式设计系统（颜色、字体、间距、图标库）
- **Why:** 没有设计系统，每个组件自己发明风格，导致 AI slop 和不一致
- **Depends on:** 先完成 AI slop 清理（去光球、换图标）

### TODO-2: 拆分 ContentCard 组件
- **What:** ContentCard 470 行、12 个状态变量。展开/聊天 overlay 应抽成独立组件
- **Why:** 性能 + 可维护性。50 张卡 = 600 个状态变量
- **Depends on:** 无限滚动 + 虚拟列表实现时一起做

### TODO-3: 设计「回顾」功能交互流程
- **What:** 设计回顾 tab 的 UI——每天展示哪些内容、怎么展示、用户操作（保留/归档/置顶）
- **Why:** 产品核心卖点，目前不存在
- **Depends on:** 建议先跑 /office-hours 梳理产品逻辑

## Implementation Tasks (from design review decisions)

### IMPL-1: 导航重组 — 内容 | 回顾 | 设置
- 周报 tab → 回顾 tab
- 数据中心降级为设置子页或独立入口

### IMPL-2: 添加 toast 通知系统
- 所有操作的错误状态向用户展示
- 导出成功给反馈
- 搜索出错显示"出错了"而非"无结果"

### IMPL-3: 全面去 AI slop
- 去掉 4 个浮动光球 (orb-1 ~ orb-4)
- emoji 图标 → Lucide 图标库
- 统一 rounded-2xl → 分层圆角（卡片 xl、按钮 lg、模态 2xl）
- 紫色渐变 → 单色强调色

### IMPL-4: 内容列表无限滚动 + 虚拟列表
- getAllContent(50, 0) → 无限滚动分页
- 引入 react-window 或类似虚拟列表

### IMPL-5: 悬浮球默认行为配置项
- 设置中添加：默认保存 vs 默认丢弃

## Engineering Debt (from /plan-eng-review 2026-03-24)

### TODO-4: 前端测试基础设施 (Vitest)
- **What:** 配置 Vitest + React Testing Library，为前端组件写测试
- **Why:** 前端完全没有测试，随着功能增多 UI 回归风险增大
- **Pros:** 捕获 UI 回归、快速验证组件行为
- **Cons:** ~15 分钟 CC 时间
- **Depends on:** 无

### NOTE: TODO-3 已完成
- /office-hours 产出了完整的消化功能设计文档 (APPROVED)
- 设计文档: ~/.gstack/projects/kdsz001-xiaoyun/pipiwang-feat-data-hub-design-20260324-172400.md
- IMPL-1 更新：导航改为 内容 | 消化 | 设置（"回顾"改名"消化"）
