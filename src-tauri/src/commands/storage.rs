use crate::commands::capture::AppState;
use crate::storage::models::CapturedContent;
use crate::storage::repository::Repository;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ContentPage {
    items: Vec<CapturedContent>,
    total: i64,
    counts: std::collections::HashMap<String, i64>,
}

#[tauri::command]
pub fn get_all_content(
    state: State<'_, AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<CapturedContent>, String> {
    let repo = Repository::new(state.db.clone());
    repo.get_all_content(limit.unwrap_or(50), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn query_content(
    state: State<'_, AppState>,
    filter: String,
    start_at: Option<String>,
    hide_sensitive: bool,
    limit: i64,
    offset: i64,
) -> Result<ContentPage, String> {
    let repo = Repository::new(state.db.clone());
    let (items, total, counts) = repo
        .query_content(
            &filter,
            start_at.as_deref(),
            hide_sensitive,
            limit.min(5000).max(1),
            offset.max(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(ContentPage {
        items,
        total,
        counts,
    })
}

#[tauri::command]
pub fn get_content_position(
    state: State<'_, AppState>,
    id: String,
    hide_sensitive: bool,
) -> Result<Option<i64>, String> {
    Repository::new(state.db.clone())
        .content_position(&id, hide_sensitive)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_content(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let repo = Repository::new(state.db.clone());
    // Wiki lifecycle: update source status and page confidence
    if let Err(e) = crate::ai::wiki_engine::on_content_deleted(state.db.clone(), &id) {
        log::warn!("Wiki content deletion hook failed for {}: {}", id, e);
    }
    repo.delete_content(&id).map_err(|e| e.to_string())
}
