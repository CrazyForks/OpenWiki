# Project: 小云 (Xiaoyun)

## Permissions

- You may run any shell commands needed for development (npm, cargo, npx, etc.)
- You may read, write, and edit any files in this project
- You may run `npm run dev`, `npm run build`, `cargo build`, `cargo check`, `cargo tauri dev` and similar development commands freely
- You may install npm packages and cargo crates as needed
- You may run tests freely
- You may create, delete, and modify files within this project directory

## Decision Protocol

- For routine development tasks (coding, debugging, refactoring, testing): proceed autonomously, no need to ask
- For important project direction decisions (architecture changes, new major dependencies, data model redesign, feature scope changes): ask the user first, present 2-3 options with a recommended choice clearly marked
- If the user does not respond within 3 minutes: proceed with the recommended option and note what was chosen
- The user is a beginner — always explain decisions in plain Chinese, avoid jargon

## GitHub

- Repository: https://github.com/kdsz001/xiaoyun (private)
- When user says "存一下" or "保存进度": commit + push to GitHub
- Write clear commit messages in conventional format (feat/fix/refactor)

## Project Info

- Tauri 2 desktop app (Rust + React/TypeScript)
- Frontend: React 19, Tailwind CSS 4, Zustand, Framer Motion
- Backend: Rust with SQLite (via rusqlite)
- Package manager: npm
- Build: Vite 6

## Design System
Always read DESIGN.md before making any visual or UI decisions.
All font choices, colors, spacing, and aesthetic direction are defined there.
Do not deviate without explicit user approval.
In QA mode, flag any code that doesn't match DESIGN.md.
