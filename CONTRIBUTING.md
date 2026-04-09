# Contributing to OpenWiki

Thank you for your interest in contributing to OpenWiki! This document will help you get started.

## Development Setup

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) (latest stable)
- [Tauri 2 CLI](https://v2.tauri.app/start/prerequisites/)
- macOS (currently the only supported platform)

### Getting Started

```bash
# Clone the repository
git clone https://github.com/kdsz001/xiaoyun.git
cd xiaoyun

# Install frontend dependencies
npm install

# Copy environment variables
cp .env.example .env
# Edit .env with your own API credentials

# Run in development mode
npm run dev
```

### Build

```bash
npm run build
```

## Project Structure

```
├── src/                  # React frontend (TypeScript)
│   ├── features/         # Feature modules (wiki, digest, settings, etc.)
│   ├── stores/           # Zustand state management
│   ├── services/         # API service layer
│   └── types/            # TypeScript type definitions
├── src-tauri/            # Rust backend
│   └── src/
│       ├── ai/           # AI integration (OAuth, API clients, prompts)
│       ├── capture/      # Clipboard & screenshot capture
│       ├── commands/     # Tauri command handlers
│       ├── storage/      # SQLite database & models
│       └── export/       # Export functionality
├── DESIGN.md             # Design system reference
└── docs/                 # Design prototypes & specs
```

## How to Contribute

### Reporting Bugs

- Open a [GitHub Issue](https://github.com/kdsz001/xiaoyun/issues)
- Include steps to reproduce, expected vs actual behavior
- Include your macOS version and app version

### Suggesting Features

- Open a GitHub Issue with the `enhancement` label
- Describe the use case and why it would be valuable

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/your-feature`
3. Make your changes
4. Test locally with `npm run dev`
5. Commit with conventional format: `feat: add new feature` / `fix: resolve bug`
6. Push and open a Pull Request

### Coding Style

- **Frontend**: Follow existing React/TypeScript patterns, use Tailwind CSS for styling
- **Backend**: Follow Rust conventions, run `cargo check` before committing
- **Design**: Read `DESIGN.md` before making any visual changes

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
