# OpenWiki i18n Implementation Plan (v2 — post Codex review)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Chinese/English bilingual support with language switching to OpenWiki, auto-detecting OS language on first launch.

**Architecture:** react-i18next with static imports, SQLite-backed persistence (same as other settings), Tauri event broadcast for cross-window sync, 6 namespace JSON files per language. Language switcher in Settings > Appearance. Brand renamed from "小云" to "OpenWiki".

**Tech Stack:** i18next, react-i18next, i18next-browser-languagedetector

**Design spec:** `docs/superpowers/specs/2026-04-09-i18n-design.md`

**Codex review fixes (v2):**
1. ~~Language detection config doesn't match spec~~ → Fixed: `supportedLngs` + `nonExplicitSupportedLngs` + `fallbackLng: 'en'`
2. ~~localStorage as only language source~~ → Fixed: language stored in SQLite via settingsStore, same as theme
3. ~~Multi-window sync incomplete~~ → Fixed: Tauri event `language-changed` broadcast + listener in all windows
4. ~~Migration order causes mixed-language app~~ → Fixed: each module ships zh-CN + en together, switcher lands last
5. ~~settingsStore stores i18n keys in data~~ → Fixed: store keeps stable IDs, UI derives keys via `t(\`provider.${id}\`)`
6. ~~Key naming inconsistent~~ → Fixed: all keys use `dot.separated.lowercase`, no camelCase
7. ~~Date/number formatting underdesigned~~ → Fixed: use `Intl.DateTimeFormat` for dates, keep simple count interpolation (only 2 languages, no complex plurals)
8. ~~Backend error strings leak Chinese~~ → Fixed: frontend wraps backend errors with generic translated message
9. ~~Testing plan too weak~~ → Fixed: add key parity check script

---

## File Structure Overview

```
New files:
  src/lib/i18n.ts                          — i18n initialization + cross-window sync
  src/lib/dateFormat.ts                    — locale-aware date formatting helper
  src/locales/zh-CN/common.json            — shared text (nav, buttons, time)
  src/locales/zh-CN/settings.json          — settings page
  src/locales/zh-CN/content.json           — content list, cards, bubble
  src/locales/zh-CN/wiki.json              — knowledge base
  src/locales/zh-CN/digest.json            — insights/radar + digest
  src/locales/zh-CN/datahub.json           — data hub + export + weekly report
  src/locales/en/common.json
  src/locales/en/settings.json
  src/locales/en/content.json
  src/locales/en/wiki.json
  src/locales/en/digest.json
  src/locales/en/datahub.json
  scripts/check-i18n-keys.ts               — key parity check between zh-CN and en

Modified files:
  package.json                             — add i18n dependencies
  src/main.tsx                             — import i18n init
  src/App.tsx                              — brand rename + useTranslation
  src/stores/settingsStore.ts              — add language field + persistence (NO i18n keys in data)
  src/features/settings/SettingsView.tsx   — language switcher + useTranslation
  src/features/content-list/ContentList.tsx
  src/features/content-list/ContentCard.tsx
  src/features/content-list/ImagePreview.tsx
  src/features/wiki/WikiView.tsx
  src/features/wiki/WikiBrowseView.tsx
  src/features/wiki/WikiPageDetail.tsx
  src/features/wiki/WikiPageCard.tsx
  src/features/wiki/WikiAskSidebar.tsx
  src/features/wiki/WikiGraphView.tsx
  src/features/wiki/WikiLintSection.tsx
  src/features/digest/RadarView.tsx
  src/features/digest/DigestView.tsx
  src/features/digest/DigestCard.tsx
  src/features/digest/InsightDetail.tsx
  src/features/data-hub/DayDetail.tsx
  src/features/data-hub/DateSidebar.tsx
  src/features/data-hub/ExportPanel.tsx
  src/features/weekly-report/ReportView.tsx
  src/features/weekly-report/ReportCard.tsx
  src/features/weekly-report/ContentPreviewPanel.tsx
  src/features/weekly-report/FeedbackButtons.tsx
  src/features/weekly-report/TextContentList.tsx
  src/features/weekly-report/CompactLinkList.tsx
  src/features/weekly-report/ActivityStatsCard.tsx
  src/features/weekly-report/ImageFilmstrip.tsx
  src/components/FloatingBubble.tsx
  src/components/BubbleView.tsx
  src/features/spotlight/SpotlightView.tsx
```

---

## Key Design Decisions (Codex-validated)

### Language detection and fallback

```typescript
i18n.init({
  supportedLngs: ['zh-CN', 'en'],
  nonExplicitSupportedLngs: true, // zh-TW, zh-HK → zh-CN; en-US, en-GB → en
  fallbackLng: 'en',              // unknown locales → English (open source default)
  // ...
});
```

This correctly implements: zh* → zh-CN, everything else → en.

### Language persistence — SQLite via settingsStore

Language is stored in the same SQLite database as theme, capture mode, etc. This keeps ONE source of truth.

```typescript
// settingsStore.ts — new field and setter
language: 'auto' as 'zh-CN' | 'en' | 'auto',
setLanguage: (lng: 'zh-CN' | 'en' | 'auto') => {
  set({ language: lng });
  updateSetting('language', lng);
  // Change i18n language
  const resolved = lng === 'auto' ? undefined : lng;
  if (resolved) i18n.changeLanguage(resolved);
  // Broadcast to other windows
  emit('language-changed', { language: lng });
},
```

On app startup, `loadFromDB` reads `language` from SQLite and calls `i18n.changeLanguage()`.

### Cross-window sync

All windows (main, bubble, spotlight) listen for the Tauri event `language-changed`:

```typescript
// In i18n.ts — after init
import { listen } from '@tauri-apps/api/event';
listen<{ language: string }>('language-changed', (event) => {
  const lng = event.payload.language;
  if (lng === 'auto') {
    // Re-detect from navigator
    i18n.changeLanguage(undefined);
  } else {
    i18n.changeLanguage(lng);
  }
});
```

### settingsStore — NO i18n keys in data objects

`PROVIDER_LABELS` and `MODELS_BY_PROVIDER` keep their current shape. Chinese text stays in the store. The UI layer uses convention-based key derivation:

```tsx
// In SettingsView.tsx — rendering provider labels
const providerLabel = t(`settings:provider.${provider}`, { defaultValue: PROVIDER_LABELS[provider] });

// Model groups
const groupLabel = model.group
  ? t(`settings:model.group.${model.group.replace(/[^a-zA-Z]/g, '').toLowerCase()}`, { defaultValue: model.group })
  : undefined;
```

Translation files provide the mapping, but the store data never changes.

### Key naming convention

All keys use `dot.separated.lowercase`. No camelCase keys. Examples:
- `capture.on` instead of `captureOn`
- `no.type.content` instead of `noTypeContent`
- `image.preview.hint` instead of `imagePreview.hint`

### Date formatting

New helper `src/lib/dateFormat.ts` using `Intl.DateTimeFormat`:

```typescript
import i18n from './i18n';

export function formatDate(date: Date, style: 'short' | 'medium' | 'long' = 'medium'): string {
  const locale = i18n.language === 'zh-CN' ? 'zh-CN' : 'en-US';
  return new Intl.DateTimeFormat(locale, {
    ...(style === 'short' && { month: 'numeric', day: 'numeric' }),
    ...(style === 'medium' && { month: 'short', day: 'numeric', weekday: 'short' }),
    ...(style === 'long' && { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' }),
  }).format(date);
}

export function formatRelativeTime(date: Date, t: (key: string, opts?: any) => string): string {
  const now = Date.now();
  const diff = now - date.getTime();
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);
  const months = Math.floor(days / 30);
  const years = Math.floor(days / 365);

  if (minutes < 1) return t('common:time.just.now');
  if (minutes < 60) return t('common:time.minutes.ago', { count: minutes });
  if (hours < 24) return t('common:time.hours.ago', { count: hours });
  if (days < 30) return t('common:time.days.ago', { count: days });
  if (months < 12) return t('common:time.months.ago', { count: months });
  return t('common:time.years.ago', { count: years });
}
```

### Backend error wrapping

When the frontend catches a backend error, it wraps it with a translated generic message and logs the original:

```typescript
// Pattern for all Rust invoke error handling
try {
  await invoke('some_command');
} catch (e) {
  console.error('Backend error:', e);
  setErrorMessage(t('common:error.generic'));
  // or for specific errors: t('settings:ai.test.failed')
}
```

### Key parity check script

`scripts/check-i18n-keys.ts` — run with `npx tsx scripts/check-i18n-keys.ts`:

```typescript
import { readdirSync, readFileSync } from 'fs';
import { join } from 'path';

const zhDir = join(__dirname, '../src/locales/zh-CN');
const enDir = join(__dirname, '../src/locales/en');

let hasError = false;

for (const file of readdirSync(zhDir)) {
  if (!file.endsWith('.json')) continue;
  const zhKeys = Object.keys(JSON.parse(readFileSync(join(zhDir, file), 'utf-8'))).sort();
  const enPath = join(enDir, file);
  const enKeys = Object.keys(JSON.parse(readFileSync(enPath, 'utf-8'))).sort();

  const missingInEn = zhKeys.filter(k => !enKeys.includes(k));
  const missingInZh = enKeys.filter(k => !zhKeys.includes(k));

  if (missingInEn.length) {
    console.error(`${file}: missing in en: ${missingInEn.join(', ')}`);
    hasError = true;
  }
  if (missingInZh.length) {
    console.error(`${file}: missing in zh-CN: ${missingInZh.join(', ')}`);
    hasError = true;
  }
}

process.exit(hasError ? 1 : 0);
```

---

## Task 1: Install Dependencies + i18n Foundation + All Translation Files

**Files:**
- Modify: `package.json`
- Create: `src/lib/i18n.ts`, `src/lib/dateFormat.ts`
- Create: all 12 translation JSON files (zh-CN + en, complete for both languages)
- Create: `scripts/check-i18n-keys.ts`
- Modify: `src/main.tsx`
- Modify: `src/stores/settingsStore.ts` — add `language` field

- [ ] **Step 1:** Install packages: `npm install i18next react-i18next i18next-browser-languagedetector`
- [ ] **Step 2:** Create directories: `mkdir -p src/locales/zh-CN src/locales/en scripts`
- [ ] **Step 3:** Create all 6 zh-CN JSON files with all keys (using normalized dot.separated.lowercase keys)
- [ ] **Step 4:** Create all 6 en JSON files with all keys (same keys, English values) — both languages ship together per Codex fix #4
- [ ] **Step 5:** Create `src/lib/i18n.ts` with corrected config: `supportedLngs`, `nonExplicitSupportedLngs`, `fallbackLng: 'en'`, Tauri event listener for cross-window sync
- [ ] **Step 6:** Create `src/lib/dateFormat.ts` with `Intl.DateTimeFormat` helpers
- [ ] **Step 7:** Create `scripts/check-i18n-keys.ts` parity check script
- [ ] **Step 8:** Modify `src/stores/settingsStore.ts` — add `language` field with SQLite persistence, `setLanguage` method with Tauri `emit('language-changed')`, load language in `loadFromDB`
- [ ] **Step 9:** Modify `src/main.tsx` — add `import './lib/i18n'` as first import
- [ ] **Step 10:** Run `npx tsx scripts/check-i18n-keys.ts` to verify key parity
- [ ] **Step 11:** Run `npm run build` to verify no errors
- [ ] **Step 12:** Commit: `feat(i18n): add react-i18next foundation with zh-CN/en translations and SQLite persistence`

---

## Task 2: Migrate App.tsx — Nav, Search, Brand Rename

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1:** Add `import { useTranslation } from 'react-i18next'`, add `const { t } = useTranslation()` inside App component
- [ ] **Step 2:** Move TABS array inside component, replace labels with `t('nav.content')`, `t('nav.wiki')`, `t('nav.digest')`, `t('nav.settings')`
- [ ] **Step 3:** Replace brand "小云" → "OpenWiki"
- [ ] **Step 4:** Replace search text: placeholder, status messages, section headers, tooltip — all with `t()` calls
- [ ] **Step 5:** Run `npm run build`, verify
- [ ] **Step 6:** Commit: `feat(i18n): migrate App.tsx — nav tabs, search, brand rename to OpenWiki`

---

## Task 3: Migrate Content Module (List + Cards + Bubble + Spotlight)

**Files:**
- Modify: `src/features/content-list/ContentList.tsx`
- Modify: `src/features/content-list/ContentCard.tsx`
- Modify: `src/features/content-list/ImagePreview.tsx`
- Modify: `src/components/FloatingBubble.tsx`
- Modify: `src/components/BubbleView.tsx`
- Modify: `src/features/spotlight/SpotlightView.tsx`

- [ ] **Step 1:** Migrate ContentList.tsx — add `useTranslation('content')`, replace all filter labels, date filters, export text, status text, empty states
- [ ] **Step 2:** Migrate ContentCard.tsx — add `useTranslation(['content', 'common'])`, replace type labels from `common`, time expressions using `formatRelativeTime()`, card-specific status text
- [ ] **Step 3:** Migrate ImagePreview.tsx — replace hint text
- [ ] **Step 4:** Migrate FloatingBubble.tsx — replace bubble text
- [ ] **Step 5:** Migrate BubbleView.tsx — replace placeholder text
- [ ] **Step 6:** Migrate SpotlightView.tsx — replace placeholder text
- [ ] **Step 7:** Run `npm run build`, verify
- [ ] **Step 8:** Commit: `feat(i18n): migrate content module — list, cards, bubble, spotlight`

---

## Task 4: Migrate Wiki Module

**Files:**
- Modify: `src/features/wiki/WikiView.tsx`
- Modify: `src/features/wiki/WikiBrowseView.tsx`
- Modify: `src/features/wiki/WikiPageDetail.tsx`
- Modify: `src/features/wiki/WikiPageCard.tsx`
- Modify: `src/features/wiki/WikiAskSidebar.tsx`
- Modify: `src/features/wiki/WikiGraphView.tsx`
- Modify: `src/features/wiki/WikiLintSection.tsx`

- [ ] **Step 1:** Migrate WikiView.tsx — title, stats (with interpolation), buttons
- [ ] **Step 2:** Migrate WikiBrowseView.tsx — filter labels, empty state
- [ ] **Step 3:** Migrate WikiPageDetail.tsx — stale warning, delete confirm, sources header, status text, confidence label, compiled footer. Wrap backend error strings with `t('common:error.generic')`
- [ ] **Step 4:** Migrate WikiPageCard.tsx — type labels, time expressions using `formatRelativeTime()`, stale badge
- [ ] **Step 5:** Migrate WikiAskSidebar.tsx — title, session labels, empty state, error prefixes wrapped
- [ ] **Step 6:** Migrate WikiGraphView.tsx — error message, empty state
- [ ] **Step 7:** Migrate WikiLintSection.tsx — title, check button, healthy state, action labels
- [ ] **Step 8:** Run `npm run build`, verify
- [ ] **Step 9:** Commit: `feat(i18n): migrate wiki module — browse, detail, ask, graph, lint`

---

## Task 5: Migrate Digest Module

**Files:**
- Modify: `src/features/digest/RadarView.tsx`
- Modify: `src/features/digest/DigestView.tsx`
- Modify: `src/features/digest/DigestCard.tsx`
- Modify: `src/features/digest/InsightDetail.tsx`

- [ ] **Step 1:** Migrate RadarView.tsx — all section titles/subtitles, empty states, error messages, status text
- [ ] **Step 2:** Migrate DigestView.tsx — title, stats, onboarding, completion, empty state, action buttons
- [ ] **Step 3:** Migrate DigestCard.tsx — time expressions using `formatRelativeTime()`, fallback text
- [ ] **Step 4:** Migrate InsightDetail.tsx — back button, interest tags, meta line, findings/suggestions headers
- [ ] **Step 5:** Run `npm run build`, verify
- [ ] **Step 6:** Commit: `feat(i18n): migrate digest module — radar, digest, insight views`

---

## Task 6: Migrate DataHub + Weekly Report Module

**Files:**
- Modify: `src/features/data-hub/DayDetail.tsx`
- Modify: `src/features/data-hub/DateSidebar.tsx`
- Modify: `src/features/data-hub/ExportPanel.tsx`
- Modify: `src/features/weekly-report/ReportView.tsx`
- Modify: `src/features/weekly-report/ReportCard.tsx`
- Modify: `src/features/weekly-report/ContentPreviewPanel.tsx`
- Modify: `src/features/weekly-report/FeedbackButtons.tsx`
- Modify: `src/features/weekly-report/TextContentList.tsx`
- Modify: `src/features/weekly-report/CompactLinkList.tsx`
- Modify: `src/features/weekly-report/ActivityStatsCard.tsx`
- Modify: `src/features/weekly-report/ImageFilmstrip.tsx`

- [ ] **Step 1:** Migrate DayDetail.tsx — date formatting using `formatDate()` helper, welcome text, stats labels, empty state
- [ ] **Step 2:** Migrate DateSidebar.tsx — stats, date formatting, buttons
- [ ] **Step 3:** Migrate ExportPanel.tsx — title, labels, button text, status
- [ ] **Step 4:** Migrate ReportView.tsx — title, history, generate/error messages. Wrap backend errors
- [ ] **Step 5:** Migrate remaining report components (ReportCard, ContentPreviewPanel, FeedbackButtons, TextContentList, CompactLinkList, ActivityStatsCard, ImageFilmstrip) — all labels, counts, empty states
- [ ] **Step 6:** Run `npm run build`, verify
- [ ] **Step 7:** Commit: `feat(i18n): migrate datahub + weekly report modules`

---

## Task 7: Migrate Settings + Enable Language Switcher (LAST)

**Files:**
- Modify: `src/features/settings/SettingsView.tsx`

This task intentionally comes LAST so the switcher is only exposed when all modules are already translated. Per Codex fix #4.

- [ ] **Step 1:** Add `useTranslation('settings')` + `const { t: tc } = useTranslation('common')`
- [ ] **Step 2:** Move config arrays (THEME_OPTIONS, BUBBLE_POSITION_OPTIONS) inside component, replace Chinese labels with `t()` calls
- [ ] **Step 3:** Replace all category labels, section headings, setting labels, descriptions, button text, status messages with `t()` calls
- [ ] **Step 4:** For provider labels: use `t(\`provider.${provider}\`, { defaultValue: PROVIDER_LABELS[provider] })` — store data unchanged
- [ ] **Step 5:** For model groups: use `t(\`model.group.${groupId}\`, { defaultValue: model.group })` — derive key from group string
- [ ] **Step 6:** Add language switcher as first item in Appearance section. Use `useSettingsStore().setLanguage()` which persists to SQLite + broadcasts via Tauri event. Show bilingual label "语言 / Language"
- [ ] **Step 7:** Replace version text "小云 v0.1.0" → `t('version')` which reads "OpenWiki v0.1.0"
- [ ] **Step 8:** Wrap backend error surfaces (OAuth errors, test connection errors, MCP errors) with translated generic message + console.error for the raw string
- [ ] **Step 9:** Run `npm run build`, verify
- [ ] **Step 10:** Commit: `feat(i18n): migrate settings — language switcher + all settings text (switcher enabled)`

---

## Task 8: Final Verification and Polish

- [ ] **Step 1:** Run key parity check: `npx tsx scripts/check-i18n-keys.ts` — must exit 0
- [ ] **Step 2:** Search for remaining Chinese hardcoded text: `grep -rn '[\x{4e00}-\x{9fff}]' src/ --include="*.tsx" --include="*.ts" | grep -v node_modules | grep -v locales/ | grep -v '.d.ts'` — only comments/console.log should remain
- [ ] **Step 3:** Run `npm run build` — clean build
- [ ] **Step 4:** Run `npm run tauri dev` — manual smoke test:
  - App opens, UI shows correct language based on OS
  - Settings > Appearance: language switcher visible, works
  - Switch to English → ALL text updates immediately, no raw keys visible
  - Switch back to Chinese → text reverts correctly
  - Navigate all tabs (Content, Wiki, Insights, Settings) — no untranslated text
  - Open bubble/spotlight window, switch language in main window → bubble/spotlight updates too
  - Close and reopen app → language preference persists
  - Check no layout overflow with English text (header nav, settings sidebar, segmented controls)
- [ ] **Step 5:** Fix any issues found, commit: `feat(i18n): final polish`

---

## Summary

| Task | Description | Files | Key Codex Fix |
|------|-------------|-------|---------------|
| 1 | Foundation — deps, i18n.ts, 12 JSON files, SQLite lang, parity script | 16 new, 3 modified | #1 #2 #3 #9 |
| 2 | App.tsx — nav, search, brand rename | 1 modified | — |
| 3 | Content module — list, cards, bubble, spotlight | 6 modified | #7 (dateFormat) |
| 4 | Wiki module — all 7 components | 7 modified | #8 (error wrap) |
| 5 | Digest module — radar, digest, insight | 4 modified | #7 (dateFormat) |
| 6 | DataHub + Report — 11 components | 11 modified | #7 (dateFormat) |
| 7 | Settings — language switcher + all text (LAST) | 1 modified | #4 #5 |
| 8 | Final verification | varies | #6 #9 |
| **Total** | | **~33 files** | All 9 fixes |
