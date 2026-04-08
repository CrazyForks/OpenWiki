/// Wiki knowledge base compilation engine.
///
/// Core operations:
/// - assess: evaluate if content has knowledge value
/// - compile: incrementally build wiki pages from content
/// - query: answer questions based on compiled wiki
/// - lint: health-check the wiki

use crate::storage::database::Database;
use crate::storage::models::{CapturedContent, WikiPage};
use crate::storage::repository::Repository;
use std::sync::Arc;

use super::wiki_prompts;

/// Compute a hash of the content's current state for change detection.
pub fn compute_content_hash(content: &CapturedContent) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.raw_text.as_deref().unwrap_or("").hash(&mut hasher);
    content.summary.as_deref().unwrap_or("").hash(&mut hasher);
    content.tags.as_deref().unwrap_or("").hash(&mut hasher);
    content.digest.as_deref().unwrap_or("").hash(&mut hasher);
    content.user_note.as_deref().unwrap_or("").hash(&mut hasher);
    content.source_url.as_deref().unwrap_or("").hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Generate a URL-safe slug from a title.
fn slugify(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c.to_lowercase().next().unwrap_or(c)
            } else if c == ' ' {
                '-'
            } else {
                // Keep CJK characters as-is
                if c as u32 > 0x2E80 {
                    c
                } else {
                    '-'
                }
            }
        })
        .collect();
    // Collapse multiple dashes
    let mut result = String::new();
    let mut last_was_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !last_was_dash {
                result.push(c);
            }
            last_was_dash = true;
        } else {
            result.push(c);
            last_was_dash = false;
        }
    }
    result.trim_matches('-').to_string()
}

/// Call AI using the project's existing multi-provider infrastructure.
/// Reuses the same provider/model resolution as spawn_summary_task.
async fn call_ai(
    db: Arc<Database>,
    system_prompt: &str,
    user_message: &str,
    max_tokens: u32,
) -> Result<String, String> {
    let repo = Repository::new(db.clone());

    let provider_str = repo
        .get_setting("ai_provider")
        .ok()
        .flatten()
        .unwrap_or_else(|| "anthropic".to_string());

    // Try OAuth paths first
    if provider_str == "openai" {
        if let Some(result) = crate::ai::attention_analyzer::try_codex_call(
            db.clone(),
            system_prompt,
            user_message,
            0.3,
            false,
        )
        .await
        {
            return result;
        }
    }

    if provider_str == "google" {
        if let Some(result) = crate::ai::attention_analyzer::try_gemini_call(
            db.clone(),
            system_prompt,
            user_message,
            0.3,
            false,
        )
        .await
        {
            return result;
        }
    }

    // API key path
    let provider_key = format!("ai_api_key_{}", provider_str);
    let api_key = repo
        .get_setting(&provider_key)
        .ok()
        .flatten()
        .or_else(|| repo.get_setting("ai_api_key").ok().flatten())
        .unwrap_or_default();

    if api_key.is_empty() {
        return Err("未配置 AI API Key".to_string());
    }

    let model = repo
        .get_setting("ai_model")
        .ok()
        .flatten()
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

    let provider = crate::ai::attention_analyzer::AnalysisProvider::from_str(&provider_str);
    crate::ai::attention_analyzer::call_analysis_api(
        &provider,
        &api_key,
        &model,
        system_prompt,
        user_message,
        max_tokens,
    )
    .await
}

/// Parse JSON from AI response, stripping markdown code blocks if present.
fn parse_ai_json(raw: &str) -> Result<serde_json::Value, String> {
    let trimmed = raw.trim();
    let cleaned = if trimmed.starts_with("```") {
        let without_prefix = if let Some(rest) = trimmed.strip_prefix("```json") {
            rest
        } else {
            &trimmed[3..]
        };
        without_prefix
            .strip_suffix("```")
            .unwrap_or(without_prefix)
            .trim()
    } else {
        trimmed
    };
    serde_json::from_str(cleaned).map_err(|e| format!("JSON 解析失败: {} — 原文: {}", e, &cleaned[..cleaned.len().min(200)]))
}

/// Assess whether a content item has knowledge value.
/// Returns (should_compile, knowledge_score, reason).
pub async fn assess_content(
    db: Arc<Database>,
    content: &CapturedContent,
) -> Result<(bool, f64, String), String> {
    let system = wiki_prompts::assessment_system_prompt();
    let user = wiki_prompts::assessment_user_message(
        content.content_type.as_str(),
        content.raw_text.as_deref().unwrap_or(""),
        content.summary.as_deref().unwrap_or(""),
        content.user_note.as_deref().unwrap_or(""),
        content.source_url.as_deref().unwrap_or(""),
        &content.source_app,
    );

    let raw = call_ai(db, &system, &user, 256).await?;
    let json = parse_ai_json(&raw)?;

    let should = json
        .get("should_compile")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let score = json
        .get("knowledge_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let reason = json
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    Ok((should, score, reason))
}

/// Compile a content item into the wiki (two-stage process).
/// Returns the list of page IDs touched.
pub async fn compile_content(
    db: Arc<Database>,
    content: &CapturedContent,
) -> Result<Vec<String>, String> {
    let repo = Repository::new(db.clone());
    let current_hash = compute_content_hash(content);
    let content_text = content.raw_text.as_deref().unwrap_or("");
    let content_summary = content.summary.as_deref().unwrap_or("");
    let content_tags = content.tags.as_deref().unwrap_or("");
    let user_note = content.user_note.as_deref().unwrap_or("");

    // --- Stage 1: Discovery ---
    let existing_pages = repo
        .get_wiki_page_summaries()
        .map_err(|e| format!("获取页面索引失败: {}", e))?;

    let discover_system = wiki_prompts::compile_discover_system_prompt();
    let discover_user = wiki_prompts::compile_discover_user_message(
        content_text,
        content_summary,
        content_tags,
        user_note,
        &existing_pages,
    );

    let discover_raw = call_ai(db.clone(), &discover_system, &discover_user, 1024).await?;
    let discover_json = parse_ai_json(&discover_raw)?;

    let creates = discover_json
        .get("creates")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let updates = discover_json
        .get("updates")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if creates.is_empty() && updates.is_empty() {
        log::info!("Wiki compile: no pages to create or update for {}", content.id);
        return Ok(vec![]);
    }

    let mut touched_ids = Vec::new();
    let execute_system = wiki_prompts::compile_execute_system_prompt();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // --- Stage 2: Execute creates ---
    for create_item in &creates {
        let title = create_item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let page_type = create_item
            .get("page_type")
            .and_then(|v| v.as_str())
            .unwrap_or("concept");

        let execute_user = wiki_prompts::compile_execute_create_message(
            content_text,
            content_summary,
            user_note,
            title,
            page_type,
        );

        let execute_raw = call_ai(db.clone(), &execute_system, &execute_user, 2048).await?;
        let page_json = parse_ai_json(&execute_raw)?;

        let page_id = uuid::Uuid::new_v4().to_string();
        let page_title = page_json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(title);
        let slug = slugify(page_title);
        let body = page_json
            .get("body_markdown")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let summary = page_json
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let tags = page_json.get("tags").map(|v| v.to_string());
        let pt = page_json
            .get("page_type")
            .and_then(|v| v.as_str())
            .unwrap_or(page_type);

        // Ensure slug is unique
        let final_slug = if repo.get_wiki_page_by_slug(&slug).map_err(|e| e.to_string())?.is_some() {
            format!("{}-{}", slug, &page_id[..8])
        } else {
            slug
        };

        let page = WikiPage {
            id: page_id.clone(),
            title: page_title.to_string(),
            slug: final_slug,
            page_type: pt.to_string(),
            body_markdown: body.to_string(),
            summary,
            tags,
            status: "active".to_string(),
            confidence: 1.0,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_compiled_at: Some(now.clone()),
        };
        repo.save_wiki_page(&page).map_err(|e| format!("保存页面失败: {}", e))?;
        repo.add_page_source(&page_id, &content.id, &current_hash)
            .map_err(|e| format!("保存来源关系失败: {}", e))?;

        // Process edges
        if let Some(edges) = page_json.get("edges").and_then(|v| v.as_array()) {
            for edge in edges {
                let target_title = edge.get("target_title").and_then(|v| v.as_str()).unwrap_or("");
                let relation = edge.get("relation").and_then(|v| v.as_str()).unwrap_or("related");
                if let Some(target) = find_page_by_title(&repo, target_title)? {
                    let _ = repo.save_wiki_edge(&page_id, &target.id, relation, 1.0);
                }
            }
        }

        touched_ids.push(page_id);
        log::info!("Wiki: created page \"{}\" ({})", page_title, pt);
    }

    // --- Stage 2: Execute updates ---
    for update_item in &updates {
        let page_id = update_item
            .get("page_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if page_id.is_empty() {
            continue;
        }

        let existing_page = match repo.get_wiki_page_by_id(page_id).map_err(|e| e.to_string())? {
            Some(p) => p,
            None => {
                log::warn!("Wiki compile: page {} not found for update, skipping", page_id);
                continue;
            }
        };

        // Get source stats for this page
        let sources = repo.get_sources_for_page(page_id).map_err(|e| e.to_string())?;
        let active_count = sources.iter().filter(|s| s.source_status == "active").count();
        let stale_count = sources.iter().filter(|s| s.source_status == "stale").count();

        let execute_user = wiki_prompts::compile_execute_update_message(
            content_text,
            content_summary,
            user_note,
            &existing_page.body_markdown,
            &existing_page.title,
            active_count,
            stale_count,
        );

        let execute_raw = call_ai(db.clone(), &execute_system, &execute_user, 2048).await?;
        let page_json = parse_ai_json(&execute_raw)?;

        let body = page_json
            .get("body_markdown")
            .and_then(|v| v.as_str())
            .unwrap_or(&existing_page.body_markdown);
        let summary = page_json
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(existing_page.summary.clone());
        let tags = page_json
            .get("tags")
            .map(|v| v.to_string())
            .or(existing_page.tags.clone());

        let updated_page = WikiPage {
            id: page_id.to_string(),
            title: existing_page.title.clone(),
            slug: existing_page.slug.clone(),
            page_type: existing_page.page_type.clone(),
            body_markdown: body.to_string(),
            summary,
            tags,
            status: "active".to_string(),
            confidence: existing_page.confidence,
            created_at: existing_page.created_at.clone(),
            updated_at: now.clone(),
            last_compiled_at: Some(now.clone()),
        };
        repo.update_wiki_page(&updated_page)
            .map_err(|e| format!("更新页面失败: {}", e))?;
        repo.add_page_source(page_id, &content.id, &current_hash)
            .map_err(|e| format!("保存来源关系失败: {}", e))?;

        // Process new edges
        if let Some(edges) = page_json.get("edges").and_then(|v| v.as_array()) {
            for edge in edges {
                let target_title = edge.get("target_title").and_then(|v| v.as_str()).unwrap_or("");
                let relation = edge.get("relation").and_then(|v| v.as_str()).unwrap_or("related");
                if let Some(target) = find_page_by_title(&repo, target_title)? {
                    if target.id != page_id {
                        let _ = repo.save_wiki_edge(page_id, &target.id, relation, 1.0);
                    }
                }
            }
        }

        touched_ids.push(page_id.to_string());
        log::info!("Wiki: updated page \"{}\"", existing_page.title);
    }

    Ok(touched_ids)
}

/// Find a wiki page by title (approximate match).
fn find_page_by_title(
    repo: &Repository,
    title: &str,
) -> Result<Option<WikiPage>, String> {
    if title.is_empty() {
        return Ok(None);
    }
    // Try exact search first
    let results = repo
        .search_wiki_pages(title, 1)
        .map_err(|e| e.to_string())?;
    if let Some(page) = results.into_iter().find(|p| p.title == title) {
        return Ok(Some(page));
    }
    // Try slug
    let slug = slugify(title);
    repo.get_wiki_page_by_slug(&slug).map_err(|e| e.to_string())
}

/// Auto-compile: assess + compile if worthy. Updates hashes.
pub async fn auto_compile(db: Arc<Database>, content_id: &str) -> Result<(), String> {
    let repo = Repository::new(db.clone());
    let content = repo
        .get_content_by_id(content_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Content {} not found", content_id))?;

    let current_hash = compute_content_hash(&content);

    // Check if already assessed at this version
    if content.wiki_assessed_hash.as_deref() == Some(&current_hash) {
        return Ok(());
    }

    // Acquire compile lock
    if !repo
        .acquire_compile_lock(content_id, &current_hash)
        .map_err(|e| e.to_string())?
    {
        log::info!("Wiki compile lock busy for {}, skipping", content_id);
        return Ok(());
    }

    // Assess
    let (should_compile, score, reason) = match assess_content(db.clone(), &content).await {
        Ok(result) => result,
        Err(e) => {
            let _ = repo.release_compile_lock(content_id, "error", None, None, Some(&e));
            return Err(e);
        }
    };

    log::info!(
        "Wiki assess {}: score={:.2}, should={}, reason={}",
        content_id, score, should_compile, reason
    );

    if !should_compile || score < 0.5 {
        // Not worth compiling — update assessed hash to avoid re-assessment
        let _ = repo.update_content_assessed_hash(content_id, &current_hash);
        let _ = repo.release_compile_lock(content_id, "skipped", None, None, None);
        return Ok(());
    }

    // Compile
    match compile_content(db.clone(), &content).await {
        Ok(touched_ids) => {
            let pages_json = serde_json::to_string(&touched_ids).unwrap_or_default();
            let _ = repo.update_content_compile_hash(content_id, &current_hash);
            let _ = repo.release_compile_lock(
                content_id,
                "completed",
                Some(&pages_json),
                None,
                None,
            );
            log::info!(
                "Wiki compile done for {}: {} pages touched",
                content_id,
                touched_ids.len()
            );
            Ok(())
        }
        Err(e) => {
            // Don't update compile_hash on failure — will retry next time
            let _ = repo.update_content_assessed_hash(content_id, &current_hash);
            let _ = repo.release_compile_lock(content_id, "error", None, None, Some(&e));
            Err(e)
        }
    }
}

/// Manual compile: skip assessment, compile directly. Updates both hashes.
pub async fn manual_compile(db: Arc<Database>, content_id: &str) -> Result<Vec<String>, String> {
    let repo = Repository::new(db.clone());
    let content = repo
        .get_content_by_id(content_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Content {} not found", content_id))?;

    let current_hash = compute_content_hash(&content);

    // Acquire compile lock
    if !repo
        .acquire_compile_lock(content_id, &current_hash)
        .map_err(|e| e.to_string())?
    {
        return Err("编译正在进行中，请稍后再试".to_string());
    }

    match compile_content(db.clone(), &content).await {
        Ok(touched_ids) => {
            let pages_json = serde_json::to_string(&touched_ids).unwrap_or_default();
            let _ = repo.update_content_compile_hash(content_id, &current_hash);
            let _ = repo.release_compile_lock(
                content_id,
                "completed",
                Some(&pages_json),
                None,
                None,
            );
            Ok(touched_ids)
        }
        Err(e) => {
            let _ = repo.release_compile_lock(content_id, "error", None, None, Some(&e));
            Err(e)
        }
    }
}

/// Handle content deletion: update source status and page confidence.
pub fn on_content_deleted(
    db: Arc<Database>,
    content_id: &str,
) -> Result<(), String> {
    let repo = Repository::new(db);

    // Mark all sources from this content as deleted
    repo.update_source_status_by_content(content_id, "deleted")
        .map_err(|e| e.to_string())?;

    // Find all affected pages
    let affected = repo
        .get_pages_for_content(content_id)
        .map_err(|e| e.to_string())?;

    for source_record in &affected {
        let page_id = &source_record.page_id;

        // Recalculate confidence
        let confidence = repo
            .recalculate_page_confidence(page_id)
            .map_err(|e| e.to_string())?;

        let (active, _total) = repo
            .count_active_sources(page_id)
            .map_err(|e| e.to_string())?;

        if active > 0 {
            // Has remaining sources — mark for recompile
            let _ = repo.update_wiki_page_status(page_id, "needs_recompile", confidence);
            // Get page title for lint message
            if let Ok(Some(page)) = repo.get_wiki_page_by_id(page_id) {
                let _ = repo.save_lint_result(
                    "orphan",
                    "warning",
                    &format!("「{}」的部分来源已删除", page.title),
                    &format!("页面置信度下降到 {:.0}%，建议重新编译", confidence * 100.0),
                    &format!("[\"{}\"]", page_id),
                );
            }
        } else {
            // No active sources — tombstone
            let _ = repo.update_wiki_page_status(page_id, "draft", 0.3);
            if let Ok(Some(page)) = repo.get_wiki_page_by_id(page_id) {
                let _ = repo.save_lint_result(
                    "orphan",
                    "critical",
                    &format!("「{}」的所有来源已删除", page.title),
                    "知识可能失效，请决定保留或删除此页面",
                    &format!("[\"{}\"]", page_id),
                );
            }
        }
    }

    Ok(())
}

/// Public wrapper for call_ai (used by wiki commands).
pub async fn call_ai_pub(
    db: Arc<Database>,
    system_prompt: &str,
    user_message: &str,
    max_tokens: u32,
) -> Result<String, String> {
    call_ai(db, system_prompt, user_message, max_tokens).await
}

/// Public wrapper for parse_ai_json (used by wiki commands).
pub fn parse_ai_json_pub(raw: &str) -> Result<serde_json::Value, String> {
    parse_ai_json(raw)
}

/// Handle content update: mark sources as stale if hash changed.
pub fn on_content_updated(
    db: Arc<Database>,
    content_id: &str,
    new_hash: &str,
) -> Result<(), String> {
    let repo = Repository::new(db);

    let sources = repo
        .get_pages_for_content(content_id)
        .map_err(|e| e.to_string())?;

    for source_record in &sources {
        if source_record.compile_hash != new_hash && source_record.source_status == "active" {
            let _ = repo.update_source_status(&source_record.page_id, content_id, "stale");
            let _ = repo.update_wiki_page_status(
                &source_record.page_id,
                "needs_recompile",
                // Keep existing confidence for now
                1.0, // Will be recalculated on recompile
            );
        }
    }

    Ok(())
}
