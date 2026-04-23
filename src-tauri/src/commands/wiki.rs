use crate::ai::wiki_engine;
use crate::commands::capture::AppState;
use crate::storage::models::{WikiConversation, WikiLintResult, WikiPage};
use crate::storage::repository::Repository;
use tauri::{AppHandle, Emitter, State};

// ===== Browse =====

#[tauri::command]
pub fn get_wiki_pages(
    state: State<'_, AppState>,
    page_type: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<WikiPage>, String> {
    let repo = Repository::new(state.db.clone());
    let lim = limit.unwrap_or(100);
    let off = offset.unwrap_or(0);
    if let Some(pt) = page_type {
        repo.get_wiki_pages_by_type(&pt).map_err(|e| e.to_string())
    } else {
        repo.get_all_wiki_pages(lim, off)
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_wiki_page(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<WikiPage>, String> {
    let repo = Repository::new(state.db.clone());
    repo.get_wiki_page_by_id(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_wiki(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<WikiPage>, String> {
    let repo = Repository::new(state.db.clone());
    repo.search_wiki_pages(&query, 20)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_wiki_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let repo = Repository::new(state.db.clone());
    repo.get_wiki_stats().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_wiki_page(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let repo = Repository::new(state.db.clone());
    repo.delete_edges_for_page(&id).map_err(|e| e.to_string())?;
    repo.delete_sources_for_page(&id)
        .map_err(|e| e.to_string())?;
    repo.delete_wiki_page(&id).map_err(|e| e.to_string())
}

// ===== Graph =====

#[tauri::command]
pub fn get_wiki_graph(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let repo = Repository::new(state.db.clone());
    let pages = repo
        .get_all_wiki_pages(500, 0)
        .map_err(|e| e.to_string())?;
    let edges = repo.get_all_wiki_edges().map_err(|e| e.to_string())?;

    let nodes: Vec<serde_json::Value> = pages
        .iter()
        .map(|p| {
            let edge_count = edges
                .iter()
                .filter(|e| e.source_page_id == p.id || e.target_page_id == p.id)
                .count();
            serde_json::json!({
                "id": p.id,
                "title": p.title,
                "page_type": p.page_type,
                "status": p.status,
                "confidence": p.confidence,
                "edge_count": edge_count,
            })
        })
        .collect();

    let edge_data: Vec<serde_json::Value> = edges
        .iter()
        .map(|e| {
            serde_json::json!({
                "source": e.source_page_id,
                "target": e.target_page_id,
                "relation": e.relation,
                "weight": e.weight,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "nodes": nodes,
        "edges": edge_data,
    }))
}

// ===== Compile =====

#[tauri::command]
pub async fn compile_content_to_wiki(
    app: AppHandle,
    state: State<'_, AppState>,
    content_id: String,
) -> Result<Vec<String>, String> {
    let db = state.db.clone();
    let _ = app.emit("wiki-compile-progress", "compiling");

    match wiki_engine::manual_compile(db.clone(), &content_id).await {
        Ok(touched_ids) => {
            // Auto-link pages by shared tags after compilation
            let _ = wiki_engine::link_pages_by_shared_tags(db);
            let _ = app.emit("wiki-compile-complete", &touched_ids);
            Ok(touched_ids)
        }
        Err(e) => {
            let _ = app.emit("wiki-compile-error", &e);
            Err(e)
        }
    }
}


// ===== Q&A (3-stage: rewrite → retrieve → answer) =====

use crate::storage::models::{WikiChatSession, WikiChatMessage};

#[tauri::command]
pub async fn wiki_ask(
    state: State<'_, AppState>,
    session_id: String,
    question: String,
) -> Result<serde_json::Value, String> {
    let db = state.db.clone();
    let repo = Repository::new(db.clone());
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Ensure session exists
    let sessions = repo.get_chat_sessions(100).map_err(|e| e.to_string())?;
    if !sessions.iter().any(|s| s.id == session_id) {
        let title: String = question.chars().take(30).collect();
        repo.create_chat_session(&session_id, Some(&title))
            .map_err(|e| e.to_string())?;
    }

    // Save user message
    let user_turn = repo.get_next_turn_index(&session_id).map_err(|e| e.to_string())?;
    let user_msg = WikiChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: question.clone(),
        pages_used: None,
        source_mode: None,
        turn_index: user_turn,
        created_at: now.clone(),
    };
    repo.add_chat_message(&user_msg).map_err(|e| e.to_string())?;

    // Build conversation context from recent turns
    let messages = repo.get_chat_messages(&session_id).map_err(|e| e.to_string())?;
    let recent_context = build_conversation_context(&messages, 3);

    // Stage 0: Query rewrite (if multi-turn)
    let search_query = if messages.len() > 1 {
        match rewrite_query(db.clone(), &question, &recent_context).await {
            Ok(q) => q,
            Err(_) => question.clone(), // fallback to original
        }
    } else {
        question.clone()
    };

    // Stage 1: Retrieve relevant page IDs via AI
    let page_index = repo.get_wiki_page_summaries_for_qa().map_err(|e| e.to_string())?;
    let relevant_ids = if page_index.is_empty() {
        vec![]
    } else {
        match retrieve_relevant_pages(db.clone(), &search_query, &recent_context, &page_index).await {
            Ok(ids) => ids,
            Err(e) => {
                log::warn!("Q&A stage 1 (retrieve) failed: {}", e);
                vec![] // fall back to ai_only
            }
        }
    };

    // Stage 2: Load full pages and answer
    let relevant_pages: Vec<(String, String, String)> = relevant_ids
        .iter()
        .filter_map(|id| {
            repo.get_wiki_page_by_id(id)
                .ok()
                .flatten()
                .filter(|p| p.status == "active" && p.confidence >= 0.5)
                .map(|p| (p.id, p.title, p.body_markdown))
        })
        .collect();

    let locale = crate::locale::resolve_locale(&db);
    let answer_system = crate::ai::wiki_prompts::query_answer_system_prompt(&locale);
    let answer_user = crate::ai::wiki_prompts::query_answer_user_message(
        &question, &recent_context, &relevant_pages, &page_index, &locale,
    );

    let raw = wiki_engine::call_ai_pub(db.clone(), &answer_system, &answer_user, 2048).await?;

    // Parse response — graceful fallback
    let (answer, page_ids_used, source_mode, confidence) =
        match wiki_engine::parse_ai_json_pub(&raw) {
            Ok(json) => {
                let a = json.get("answer").and_then(|v| v.as_str()).unwrap_or(&raw).to_string();
                let pids: Vec<String> = json.get("page_ids_used")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let sm = json.get("source_mode").and_then(|v| v.as_str()).unwrap_or(
                    if pids.is_empty() { "ai_only" } else { "knowledge_base" }
                ).to_string();
                let c = json.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
                (a, pids, sm, c)
            }
            Err(_) => {
                // Malformed JSON — try to extract "answer" field via regex
                let extracted = extract_answer_from_malformed_json(&raw);
                (extracted, vec![], "ai_only".to_string(), 0.3)
            }
        };

    // Save assistant message
    let asst_turn = repo.get_next_turn_index(&session_id).map_err(|e| e.to_string())?;
    let pages_json = serde_json::to_string(&page_ids_used).unwrap_or_else(|_| "[]".to_string());
    let asst_msg = WikiChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.clone(),
        role: "assistant".to_string(),
        content: answer.clone(),
        pages_used: Some(pages_json.clone()),
        source_mode: Some(source_mode.clone()),
        turn_index: asst_turn,
        created_at: now.clone(),
    };
    repo.add_chat_message(&asst_msg).map_err(|e| e.to_string())?;
    let _ = repo.touch_chat_session(&session_id);

    // Resolve page titles for frontend display
    let page_titles: Vec<serde_json::Value> = page_ids_used.iter().filter_map(|id| {
        repo.get_wiki_page_by_id(id).ok().flatten().map(|p| {
            serde_json::json!({"id": p.id, "title": p.title})
        })
    }).collect();

    Ok(serde_json::json!({
        "message_id": asst_msg.id,
        "answer": answer,
        "pages_used": page_titles,
        "source_mode": source_mode,
        "confidence": confidence,
    }))
}

/// Try to extract the "answer" field from malformed JSON.
/// Handles cases like: {"answer": "内容...", "page_ids_used": ...}
/// where the overall JSON is broken but the answer value is recoverable.
fn extract_answer_from_malformed_json(raw: &str) -> String {
    // Strategy 1: find "answer" key and extract its string value
    if let Some(start) = raw.find("\"answer\"") {
        let after_key = &raw[start + 8..]; // skip "answer"
        // Skip whitespace and colon
        let after_colon = after_key.trim_start();
        if let Some(rest) = after_colon.strip_prefix(':') {
            let rest = rest.trim_start();
            if rest.starts_with('"') {
                // Walk the string, handling escaped quotes
                let chars: Vec<char> = rest.chars().collect();
                let mut i = 1; // skip opening quote
                let mut result = String::new();
                while i < chars.len() {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        match chars[i + 1] {
                            'n' => result.push('\n'),
                            't' => result.push('\t'),
                            '"' => result.push('"'),
                            '\\' => result.push('\\'),
                            other => { result.push('\\'); result.push(other); }
                        }
                        i += 2;
                    } else if chars[i] == '"' {
                        break; // closing quote
                    } else {
                        result.push(chars[i]);
                        i += 1;
                    }
                }
                if !result.is_empty() {
                    return result;
                }
            }
        }
    }
    // Strategy 2: if nothing worked, strip obvious JSON wrapper
    raw.to_string()
}

/// Build conversation context string from recent messages (last N turns).
fn build_conversation_context(messages: &[WikiChatMessage], max_turns: usize) -> String {
    let recent: Vec<&WikiChatMessage> = messages.iter().rev().take(max_turns * 2).collect();
    let mut parts = Vec::new();
    let mut budget = 2000i64;
    for msg in recent.iter().rev() {
        let role_label = if msg.role == "user" { "User" } else { "Assistant" };
        let content: String = msg.content.chars().take(budget.max(0) as usize).collect();
        budget -= content.len() as i64;
        parts.push(format!("{}: {}", role_label, content));
        if budget <= 0 { break; }
    }
    parts.join("\n")
}

/// Stage 0: Rewrite a follow-up question into a standalone query.
async fn rewrite_query(
    db: std::sync::Arc<crate::storage::database::Database>,
    question: &str,
    context: &str,
) -> Result<String, String> {
    let locale = crate::locale::resolve_locale(&db);
    let system = crate::ai::wiki_prompts::query_rewrite_system_prompt(&locale);
    let user = crate::ai::wiki_prompts::query_rewrite_user_message(question, context, &locale);
    let raw = wiki_engine::call_ai_pub(db, &system, &user, 256).await?;
    Ok(raw.trim().to_string())
}

/// Stage 1: Ask AI to pick relevant page IDs from the index.
async fn retrieve_relevant_pages(
    db: std::sync::Arc<crate::storage::database::Database>,
    query: &str,
    context: &str,
    page_index: &[(String, String, String)],
) -> Result<Vec<String>, String> {
    let locale = crate::locale::resolve_locale(&db);
    let system = crate::ai::wiki_prompts::query_retrieve_system_prompt(&locale);
    let user = crate::ai::wiki_prompts::query_retrieve_user_message(query, context, page_index, &locale);
    let raw = wiki_engine::call_ai_pub(db, &system, &user, 512).await?;
    let json = wiki_engine::parse_ai_json_pub(&raw)?;
    let ids: Vec<String> = json.get("page_ids")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    Ok(ids)
}

// ===== Chat Session Management =====

#[tauri::command]
pub fn get_chat_sessions(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<WikiChatSession>, String> {
    let repo = Repository::new(state.db.clone());
    repo.get_chat_sessions(limit.unwrap_or(20))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_chat_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<WikiChatMessage>, String> {
    let repo = Repository::new(state.db.clone());
    repo.get_chat_messages(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_chat_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let repo = Repository::new(state.db.clone());
    repo.delete_chat_session(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_message_as_page(
    state: State<'_, AppState>,
    session_id: String,
    message_id: String,
) -> Result<WikiPage, String> {
    let repo = Repository::new(state.db.clone());
    let messages = repo.get_chat_messages(&session_id).map_err(|e| e.to_string())?;

    let asst_msg = messages.iter().find(|m| m.id == message_id && m.role == "assistant")
        .ok_or_else(|| "Message not found".to_string())?;

    // Anti-contamination: only allow saving if source_mode is not ai_only
    let source_mode = asst_msg.source_mode.as_deref().unwrap_or("ai_only");
    if source_mode == "ai_only" {
        return Err("AI-only answers cannot be saved as wiki pages (no knowledge base sources)".to_string());
    }

    // Dedup: check if this message was already saved (DB-enforced via UNIQUE index)
    if let Ok(Some(existing)) = repo.get_wiki_page_by_source_message_id(&message_id) {
        return Ok(existing);
    }

    // Find the preceding user question
    let user_question = messages.iter().rev()
        .find(|m| m.turn_index < asst_msg.turn_index && m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_else(|| "Q&A".to_string());

    let page_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let title: String = user_question.chars().take(40).collect();

    let page = WikiPage {
        id: page_id.clone(),
        title,
        slug: format!("qa-{}", &page_id[..8]),
        page_type: "qa".to_string(),
        body_markdown: format!("## Question\n\n{}\n\n## Answer\n\n{}", user_question, asst_msg.content),
        summary: Some(format!("Q&A: {}", &user_question.chars().take(30).collect::<String>())),
        tags: None,
        status: "active".to_string(),
        confidence: 0.7,
        created_at: now.clone(),
        updated_at: now.clone(),
        last_compiled_at: Some(now),
        source_message_id: Some(message_id.clone()),
    };

    repo.save_wiki_page(&page).map_err(|e| e.to_string())?;

    // Create deterministic edges from QA page to referenced pages (from pages_used)
    if let Some(ref pages_json) = asst_msg.pages_used {
        let referenced_ids: Vec<String> = serde_json::from_str(pages_json).unwrap_or_default();
        for ref_item in &referenced_ids {
            // pages_used may contain {id, title} objects or plain strings
            let ref_id = if let Ok(obj) = serde_json::from_str::<serde_json::Value>(ref_item) {
                obj.get("id").and_then(|v| v.as_str()).unwrap_or(ref_item).to_string()
            } else {
                ref_item.clone()
            };
            if !ref_id.is_empty() {
                let _ = repo.save_wiki_edge(&page_id, &ref_id, "related", 1.0);
                let _ = repo.save_wiki_edge(&ref_id, &page_id, "related", 1.0); // bidirectional
            }
        }
    }

    Ok(page)
}

/// Check which message IDs have already been saved as wiki pages.
#[tauri::command]
pub fn get_saved_message_ids(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    let repo = Repository::new(state.db.clone());
    let mut saved = Vec::new();
    for mid in &message_ids {
        if let Ok(Some(_)) = repo.get_wiki_page_by_source_message_id(mid) {
            saved.push(mid.clone());
        }
    }
    Ok(saved)
}

// Legacy compatibility — keep old commands but delegate
#[tauri::command]
pub fn get_wiki_conversations(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<WikiConversation>, String> {
    let repo = Repository::new(state.db.clone());
    repo.get_wiki_conversations(limit.unwrap_or(20))
        .map_err(|e| e.to_string())
}

// ===== Tag-based linking =====

#[tauri::command]
pub fn wiki_link_by_tags(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.clone();
    let count = wiki_engine::link_pages_by_shared_tags(db)?;
    Ok(serde_json::json!({ "edges_created": count }))
}

// ===== Lint =====

#[tauri::command]
pub async fn trigger_wiki_lint(
    state: State<'_, AppState>,
) -> Result<Vec<WikiLintResult>, String> {
    let repo = Repository::new(state.db.clone());

    // Local checks first (no AI needed)
    let mut results = Vec::new();

    // Check for needs_recompile pages
    let stale_pages = repo
        .get_wiki_pages_by_status("needs_recompile")
        .map_err(|e| e.to_string())?;
    for page in &stale_pages {
        let _ = repo.save_lint_result(
            "stale",
            "warning",
            &format!("\"{}\" has stale sources", page.title),
            "Some sources have been updated or deleted, recompilation recommended",
            &format!("[\"{}\"]", page.id),
        );
    }

    // Check for draft (tombstone) pages
    let draft_pages = repo
        .get_wiki_pages_by_status("draft")
        .map_err(|e| e.to_string())?;
    for page in &draft_pages {
        let _ = repo.save_lint_result(
            "orphan",
            "critical",
            &format!("\"{}\" is invalid", page.title),
            "All sources have been deleted, please decide to keep or remove",
            &format!("[\"{}\"]", page.id),
        );
    }

    results = repo
        .get_open_lint_results()
        .map_err(|e| e.to_string())?;

    Ok(results)
}

#[tauri::command]
pub fn get_wiki_lint_results(
    state: State<'_, AppState>,
) -> Result<Vec<WikiLintResult>, String> {
    let repo = Repository::new(state.db.clone());
    repo.get_open_lint_results().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wiki_lint_keep(
    state: State<'_, AppState>,
    lint_id: i64,
) -> Result<(), String> {
    let repo = Repository::new(state.db.clone());
    // Get the lint result to find affected page
    let lints = repo.get_open_lint_results().map_err(|e| e.to_string())?;
    if let Some(lint) = lints.iter().find(|l| l.id == lint_id) {
        let page_ids: Vec<String> =
            serde_json::from_str(&lint.page_ids).unwrap_or_default();
        for pid in &page_ids {
            // Restore draft pages to active
            if let Ok(Some(page)) = repo.get_wiki_page_by_id(pid) {
                if page.status == "draft" {
                    let _ = repo.update_wiki_page_status(pid, "active", page.confidence);
                }
            }
        }
    }
    repo.resolve_lint_result(lint_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wiki_lint_delete(
    state: State<'_, AppState>,
    lint_id: i64,
) -> Result<(), String> {
    let repo = Repository::new(state.db.clone());
    let lints = repo.get_open_lint_results().map_err(|e| e.to_string())?;
    if let Some(lint) = lints.iter().find(|l| l.id == lint_id) {
        let page_ids: Vec<String> =
            serde_json::from_str(&lint.page_ids).unwrap_or_default();
        for pid in &page_ids {
            let _ = repo.delete_edges_for_page(pid);
            let _ = repo.delete_sources_for_page(pid);
            let _ = repo.delete_wiki_page(pid);
        }
    }
    repo.resolve_lint_result(lint_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wiki_lint_recompile(
    app: AppHandle,
    state: State<'_, AppState>,
    lint_id: i64,
) -> Result<(), String> {
    let repo = Repository::new(state.db.clone());
    let lints = repo.get_open_lint_results().map_err(|e| e.to_string())?;
    if let Some(lint) = lints.iter().find(|l| l.id == lint_id) {
        let page_ids: Vec<String> =
            serde_json::from_str(&lint.page_ids).unwrap_or_default();
        for pid in &page_ids {
            let (active, _) = repo
                .count_active_sources(pid)
                .map_err(|e| e.to_string())?;
            if active == 0 {
                return Err("No active sources, cannot recompile".to_string());
            }
            // Get active source content IDs and re-compile each
            let sources = repo
                .get_sources_for_page(pid)
                .map_err(|e| e.to_string())?;
            for src in sources.iter().filter(|s| s.source_status == "active") {
                let _ =
                    wiki_engine::auto_compile(state.db.clone(), &src.content_id).await;
            }
        }
    }
    repo.resolve_lint_result(lint_id)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("wiki-lint-recompile-complete", "done");
    Ok(())
}

// ===== Page Sources (for frontend) =====

#[tauri::command]
pub fn get_page_sources(
    state: State<'_, AppState>,
    page_id: String,
) -> Result<Vec<crate::storage::models::WikiPageSource>, String> {
    let repo = Repository::new(state.db.clone());
    repo.get_sources_for_page(&page_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_content_wiki_pages(
    state: State<'_, AppState>,
    content_id: String,
) -> Result<Vec<WikiPage>, String> {
    let repo = Repository::new(state.db.clone());
    let sources = repo
        .get_pages_for_content(&content_id)
        .map_err(|e| e.to_string())?;
    let mut pages = Vec::new();
    for src in &sources {
        if let Ok(Some(page)) = repo.get_wiki_page_by_id(&src.page_id) {
            if page.status == "active" || page.status == "needs_recompile" {
                pages.push(page);
            }
        }
    }
    Ok(pages)
}
