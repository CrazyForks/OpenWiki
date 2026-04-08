use crate::commands::capture::AppState;
use crate::storage::models::CapturedContent;
use crate::storage::repository::Repository;
use tauri::State;

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
pub fn delete_content(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let repo = Repository::new(state.db.clone());
    // Wiki lifecycle: update source status and page confidence
    if let Err(e) = crate::ai::wiki_engine::on_content_deleted(state.db.clone(), &id) {
        log::warn!("Wiki content deletion hook failed for {}: {}", id, e);
    }
    repo.delete_content(&id).map_err(|e| e.to_string())
}
