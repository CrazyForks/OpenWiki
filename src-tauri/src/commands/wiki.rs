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

    match wiki_engine::manual_compile(db, &content_id).await {
        Ok(touched_ids) => {
            let _ = app.emit("wiki-compile-complete", &touched_ids);
            Ok(touched_ids)
        }
        Err(e) => {
            let _ = app.emit("wiki-compile-error", &e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn trigger_wiki_auto_compile(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.clone();
    let repo = Repository::new(db.clone());

    // Find content that hasn't been assessed at current version
    let all_content = repo
        .get_all_content(200, 0)
        .map_err(|e| e.to_string())?;

    let mut compiled = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for content in &all_content {
        let current_hash = wiki_engine::compute_content_hash(content);
        if content.wiki_assessed_hash.as_deref() == Some(&current_hash) {
            continue; // Already assessed at this version
        }
        match wiki_engine::auto_compile(db.clone(), &content.id).await {
            Ok(()) => compiled += 1,
            Err(e) => {
                log::warn!("Wiki auto-compile error for {}: {}", content.id, e);
                errors += 1;
            }
        }
        skipped += 1;
    }

    let _ = app.emit("wiki-auto-compile-complete", "done");

    Ok(serde_json::json!({
        "processed": compiled + skipped,
        "compiled": compiled,
        "errors": errors,
    }))
}

// ===== Q&A =====

#[tauri::command]
pub async fn wiki_ask(
    state: State<'_, AppState>,
    question: String,
) -> Result<serde_json::Value, String> {
    let db = state.db.clone();
    let repo = Repository::new(db.clone());

    // Search for relevant pages (active + high confidence only for Q&A)
    let pages = repo
        .search_wiki_pages(&question, 10)
        .map_err(|e| e.to_string())?;
    let qa_pages: Vec<_> = pages
        .into_iter()
        .filter(|p| p.status == "active" && p.confidence >= 0.5)
        .collect();

    let context: Vec<(String, String, String)> = qa_pages
        .iter()
        .map(|p| (p.id.clone(), p.title.clone(), p.body_markdown.clone()))
        .collect();

    let system = crate::ai::wiki_prompts::query_system_prompt();
    let user = crate::ai::wiki_prompts::query_user_message(&question, &context);

    let raw = wiki_engine::call_ai_pub(db.clone(), &system, &user, 2048).await?;
    let json = wiki_engine::parse_ai_json_pub(&raw)?;

    // Save conversation
    let conv_id = uuid::Uuid::new_v4().to_string();
    let answer = json
        .get("answer")
        .and_then(|v| v.as_str())
        .unwrap_or("无法回答")
        .to_string();
    let pages_used = json
        .get("page_ids_used")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "[]".to_string());

    let conv = WikiConversation {
        id: conv_id.clone(),
        question: question.clone(),
        answer: answer.clone(),
        pages_used: pages_used.clone(),
        saved_as_page: None,
        model_used: None,
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    let _ = repo.save_wiki_conversation(&conv);

    Ok(serde_json::json!({
        "conversation_id": conv_id,
        "answer": answer,
        "pages_used": json.get("page_ids_used"),
        "confidence": json.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0),
        "suggested_followup": json.get("suggested_followup").and_then(|v| v.as_str()).unwrap_or(""),
    }))
}

#[tauri::command]
pub fn get_wiki_conversations(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<WikiConversation>, String> {
    let repo = Repository::new(state.db.clone());
    repo.get_wiki_conversations(limit.unwrap_or(20))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_answer_as_page(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<WikiPage, String> {
    let repo = Repository::new(state.db.clone());
    let convs = repo
        .get_wiki_conversations(100)
        .map_err(|e| e.to_string())?;
    let conv = convs
        .into_iter()
        .find(|c| c.id == conversation_id)
        .ok_or_else(|| "对话不存在".to_string())?;

    let page_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let title = if conv.question.len() > 30 {
        format!("{}...", &conv.question[..30])
    } else {
        conv.question.clone()
    };

    let page = WikiPage {
        id: page_id.clone(),
        title,
        slug: format!("qa-{}", &page_id[..8]),
        page_type: "concept".to_string(),
        body_markdown: format!("## 问题\n\n{}\n\n## 回答\n\n{}", conv.question, conv.answer),
        summary: Some(format!("Q&A: {}", &conv.question.chars().take(30).collect::<String>())),
        tags: None,
        status: "active".to_string(),
        confidence: 0.8,
        created_at: now.clone(),
        updated_at: now.clone(),
        last_compiled_at: Some(now),
    };

    repo.save_wiki_page(&page).map_err(|e| e.to_string())?;
    let _ = repo.update_conversation_saved_page(&conversation_id, &page_id);

    Ok(page)
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
            &format!("「{}」有过时来源", page.title),
            "部分来源已更新或删除，建议重新编译",
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
            &format!("「{}」已失效", page.title),
            "所有来源已删除，请决定保留或删除",
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
                return Err("没有活跃来源，无法重编".to_string());
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
