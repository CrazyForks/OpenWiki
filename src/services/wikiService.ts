import { invoke } from "@tauri-apps/api/core";
import type {
  WikiPage,
  WikiPageSource,
  WikiConversation,
  WikiLintResult,
  WikiStats,
  WikiGraphData,
} from "../types/wiki";

// ===== Browse =====

export async function getWikiPages(opts?: {
  page_type?: string;
  limit?: number;
  offset?: number;
}): Promise<WikiPage[]> {
  return invoke("get_wiki_pages", {
    pageType: opts?.page_type ?? null,
    limit: opts?.limit ?? 100,
    offset: opts?.offset ?? 0,
  });
}

export async function getWikiPage(id: string): Promise<WikiPage | null> {
  return invoke("get_wiki_page", { id });
}

export async function searchWiki(query: string): Promise<WikiPage[]> {
  return invoke("search_wiki", { query });
}

export async function getWikiStats(): Promise<WikiStats> {
  return invoke("get_wiki_stats");
}

export async function deleteWikiPage(id: string): Promise<void> {
  return invoke("delete_wiki_page", { id });
}

// ===== Graph =====

export async function getWikiGraph(): Promise<WikiGraphData> {
  return invoke("get_wiki_graph");
}

// ===== Compile =====

export async function compileContentToWiki(contentId: string): Promise<string[]> {
  return invoke("compile_content_to_wiki", { contentId });
}

export async function triggerWikiAutoCompile(): Promise<{
  processed: number;
  compiled: number;
  errors: number;
}> {
  return invoke("trigger_wiki_auto_compile");
}

// ===== Q&A =====

export async function wikiAsk(question: string): Promise<{
  conversation_id: string;
  answer: string;
  pages_used: string[];
  confidence: number;
  suggested_followup: string;
}> {
  return invoke("wiki_ask", { question });
}

export async function getWikiConversations(limit?: number): Promise<WikiConversation[]> {
  return invoke("get_wiki_conversations", { limit: limit ?? 20 });
}

export async function saveAnswerAsPage(conversationId: string): Promise<WikiPage> {
  return invoke("save_answer_as_page", { conversationId });
}

// ===== Lint =====

export async function triggerWikiLint(): Promise<WikiLintResult[]> {
  return invoke("trigger_wiki_lint");
}

export async function getWikiLintResults(): Promise<WikiLintResult[]> {
  return invoke("get_wiki_lint_results");
}

export async function wikiLintKeep(lintId: number): Promise<void> {
  return invoke("wiki_lint_keep", { lintId });
}

export async function wikiLintDelete(lintId: number): Promise<void> {
  return invoke("wiki_lint_delete", { lintId });
}

export async function wikiLintRecompile(lintId: number): Promise<void> {
  return invoke("wiki_lint_recompile", { lintId });
}

// ===== Sources =====

export async function getPageSources(pageId: string): Promise<WikiPageSource[]> {
  return invoke("get_page_sources", { pageId });
}

export async function getContentWikiPages(contentId: string): Promise<WikiPage[]> {
  return invoke("get_content_wiki_pages", { contentId });
}
