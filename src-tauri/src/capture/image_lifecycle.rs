use std::path::{Path, PathBuf};

const APP_DATA_DIR: &str = "com.openwiki.app";

fn app_data_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Library").join("Application Support")))
        .map(|base| base.join(APP_DATA_DIR))
        .ok_or_else(|| "Cannot determine application data directory".to_string())
}

fn ensure_dir(name: &str) -> Result<PathBuf, String> {
    let dir = app_data_dir()?.join(name);
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Failed to create {} directory: {}", name, error))?;
    Ok(dir)
}

pub fn captures_dir() -> Result<PathBuf, String> {
    ensure_dir("captures")
}

pub fn pending_images_dir() -> Result<PathBuf, String> {
    let dir = captures_dir()?.join(".pending");
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Failed to create pending image directory: {}", error))?;
    Ok(dir)
}

pub fn thumbnails_dir() -> Result<PathBuf, String> {
    ensure_dir("thumbnails")
}

pub(crate) fn is_owned_pending_image_in(path: &Path, pending_dir: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let Ok(pending) = pending_dir.canonicalize() else {
        return false;
    };
    path.canonicalize()
        .ok()
        .and_then(|resolved| resolved.parent().map(|parent| parent == pending))
        .unwrap_or(false)
}

pub(crate) fn cleanup_pending_image_in(path: &Path, pending_dir: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    if !is_owned_pending_image_in(path, pending_dir) {
        return Err("Refusing to delete an image outside captures/.pending".to_string());
    }
    std::fs::remove_file(path)
        .map(|_| true)
        .map_err(|error| format!("Failed to remove pending image: {}", error))
}

pub fn cleanup_pending_image(path: &str) -> Result<bool, String> {
    let path = Path::new(path);
    let pending = pending_images_dir()?;
    cleanup_pending_image_in(path, &pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_rejects_external_files() {
        let dir = tempfile::tempdir().unwrap();
        let pending = dir.path().join("captures/.pending");
        std::fs::create_dir_all(&pending).unwrap();
        let file = dir.path().join("external.png");
        std::fs::write(&file, b"image").unwrap();
        assert!(cleanup_pending_image_in(&file, &pending).is_err());
        assert!(file.exists());
    }

    #[test]
    fn cleanup_removes_only_direct_pending_children() {
        let dir = tempfile::tempdir().unwrap();
        let pending = dir.path().join("captures/.pending");
        let nested = pending.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let owned = pending.join("owned.png");
        let nested_file = nested.join("nested.png");
        std::fs::write(&owned, b"image").unwrap();
        std::fs::write(&nested_file, b"image").unwrap();

        assert_eq!(cleanup_pending_image_in(&owned, &pending), Ok(true));
        assert!(!owned.exists());
        assert!(cleanup_pending_image_in(&nested_file, &pending).is_err());
        assert!(nested_file.exists());
    }
}
