# Release Notes

每个版本对应一个独立的 Markdown 文件，文件名即 git tag：`v0.1.5.md` → tag `v0.1.5`。

`.github/workflows/release.yml` 在构建 release 时会自动读取 `release-notes/${tag}.md` 的完整内容作为 GitHub Release 的 body。因此这里的每个文件都是完整的 release body —— 包含 changelog 和安装说明。

## For Claude (发版流程)

用户（Ray）是非技术人员，不会自己维护这个文件夹。发版流程由 Claude 自动完成。

当用户说"发版"、"发 release"、"打 tag"、"发个新版本"时，执行以下步骤：

1. **决定版本号** — 检查 `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml` 的当前版本，和最新的 git tag 对比。通常 bump patch 号。
2. **总结改动** — 运行 `git log <previous-tag>..HEAD --oneline` 查看所有 commit。**不要直接贴 commit message**（那是给开发者看的）。把它们精简成用户能看懂的短条目：
   - 格式：`- 优化了 X`、`- 修复了 X`、`- 新增 X`
   - 不暴露文件名、变量名、技术栈名词（除非是产品名）
   - 每条一句话，说用户能感知到什么变化，不说"为什么"
   - 双语（中文 + 英文）
3. **创建 release notes 文件** — 从 `release-notes/TEMPLATE.md` 复制一份到 `release-notes/v<new-version>.md`：
   - 把 changelog 填进 `本次更新 / What's New` 部分
   - 把所有 `{{VERSION}}` 占位符替换成实际版本号（不带 `v` 前缀，如 `0.1.6`）
4. **bump 版本号** — 同时改三个文件的 version 字段：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`
5. **同步 Cargo.lock** — 在 `src-tauri/` 下跑 `cargo check`，让 Cargo.lock 自动更新
6. **commit** — `chore: release vX.Y.Z`（commit message 可以包含详细技术细节给开发者看，用户面的 changelog 走 release-notes 文件）
7. **打 tag** — `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
8. **push code + tag** — `git push && git push origin vX.Y.Z`，这会触发 GitHub Actions 自动构建 DMG 并发布 Release
9. **告诉用户** — 简短同步进度：tag 已推、Actions 正在跑、大约 15 分钟会出 DMG、给 Actions 页面链接

## 写 changelog 的好例子

好：
- 优化了 YouTube 视频字幕识别，开箱即用
- 修复了 AI 摘要语言不跟随系统的问题
- 优化了洞察页面的体验

坏（太技术）：
- 把 yt-dlp 打包进 app bundle
- 修复 locale.rs 里 LANG env var 判断优先级
- 删除 on_content_deleted 里的 save_lint_result 调用

## 文件结构

```
release-notes/
├── README.md        ← 这个文件
├── TEMPLATE.md      ← 发新版时从这里复制
├── v0.1.5.md        ← 每个版本一个文件
├── v0.1.6.md
└── ...
```
