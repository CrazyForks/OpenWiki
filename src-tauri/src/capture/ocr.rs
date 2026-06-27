use std::path::PathBuf;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
fn is_cjk_or_cjk_punctuation(ch: char) -> bool {
    is_cjk_char(ch) || is_cjk_punctuation(ch)
}

#[cfg(target_os = "windows")]
fn is_cjk_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{3400}'..='\u{9fff}' | '\u{f900}'..='\u{faff}'
    )
}

#[cfg(target_os = "windows")]
fn is_cjk_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '\u{3000}'..='\u{303f}' | '\u{ff01}'..='\u{ff60}'
    )
}

#[cfg(target_os = "windows")]
fn normalize_windows_ocr_text(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut normalized = String::with_capacity(text.len());

    for (idx, ch) in chars.iter().enumerate() {
        if ch.is_whitespace() && *ch != '\n' && *ch != '\r' {
            let prev = idx.checked_sub(1).and_then(|i| chars.get(i)).copied();
            let next = chars.get(idx + 1).copied();
            if prev.is_some_and(is_cjk_or_cjk_punctuation)
                && next.is_some_and(is_cjk_or_cjk_punctuation)
            {
                continue;
            }
            if next.is_some_and(is_cjk_punctuation) {
                continue;
            }
            if prev.is_some_and(is_cjk_punctuation) && next.is_some_and(is_cjk_char) {
                continue;
            }
        }
        normalized.push(*ch);
    }

    normalized
}

#[cfg(target_os = "windows")]
fn hide_command_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

/// Path to the pre-compiled OCR helper binary, resolved at app startup
/// from the Tauri resource directory. See `lib.rs` setup hook.
#[cfg(target_os = "macos")]
static OCR_BINARY_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Register the OCR helper binary path. Called once from the Tauri setup hook
/// with the resource directory resolved via `AppHandle::path().resource_dir()`.
pub fn init_ocr_binary_path(path: PathBuf) {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
    }

    #[cfg(target_os = "macos")]
    let _ = OCR_BINARY_PATH.set(path);
}

/// Locate the OCR helper binary.
///
/// Normally this just returns the path registered at startup, but it also
/// falls back to searching next to the current executable so things keep
/// working in cargo tests and unusual launch contexts.
#[cfg(target_os = "macos")]
fn resolve_ocr_binary() -> Result<PathBuf, String> {
    if let Some(p) = OCR_BINARY_PATH.get() {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidates = [
                parent.join("openwiki_ocr_bin"),
                parent.join("../Resources/openwiki_ocr_bin"),
                parent.join("../Resources/resources/openwiki_ocr_bin"),
            ];
            for c in candidates.iter() {
                if c.exists() {
                    return Ok(c.clone());
                }
            }
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(manifest_dir).join("resources/openwiki_ocr_bin");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        for candidate in [
            current_dir.join("resources/openwiki_ocr_bin"),
            current_dir.join("src-tauri/resources/openwiki_ocr_bin"),
        ] {
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Err("OCR helper binary not found — the app bundle may be corrupted.".to_string())
}

/// Perform OCR on an image file using macOS Vision framework.
/// Returns the recognized text, supporting Chinese + English.
///
/// The helper binary is pre-compiled at build time and shipped inside the
/// app bundle, so end users do NOT need to install Xcode Command Line Tools.
pub fn recognize_text(image_path: &str) -> Result<String, String> {
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = image_path;
        return Err("OCR is not available on this platform yet.".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        return recognize_text_windows(image_path);
    }

    #[cfg(target_os = "macos")]
    {
        let binary_path = resolve_ocr_binary()?;

        let output = Command::new(&binary_path)
            .arg(image_path)
            .output()
            .map_err(|e| format!("Failed to run OCR: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("OCR failed: {}", stderr.trim()));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if text.is_empty() {
            return Err("未识别到文字内容".to_string());
        }

        log::info!("[OCR] 识别完成: {} chars from {}", text.len(), image_path);
        Ok(text)
    }
}

#[cfg(target_os = "windows")]
fn recognize_text_windows(image_path: &str) -> Result<String, String> {
    const WINDOWS_OCR_SCRIPT: &str = r#"
param([string]$ImagePath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime

$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
  $_.Name -eq 'AsTask' -and
  $_.GetParameters().Count -eq 1 -and
  $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
})[0]

function Await($Operation, [Type]$ResultType) {
  $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
  $task = $asTask.Invoke($null, @($Operation))
  $task.Wait() | Out-Null
  $task.Result
}

[Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime] | Out-Null
[Windows.Storage.FileAccessMode, Windows.Storage, ContentType = WindowsRuntime] | Out-Null
[Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics.Imaging, ContentType = WindowsRuntime] | Out-Null
[Windows.Graphics.Imaging.SoftwareBitmap, Windows.Graphics.Imaging, ContentType = WindowsRuntime] | Out-Null
[Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime] | Out-Null
[Windows.Media.Ocr.OcrResult, Windows.Foundation, ContentType = WindowsRuntime] | Out-Null

$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($ImagePath)) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) {
  throw 'No Windows OCR engine is available for the current user profile languages.'
}
$result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
$text = $result.Text
if ($null -eq $text) {
  $text = ''
}
[Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($text))
"#;

    let script_path = std::env::temp_dir().join("openwiki_windows_ocr.ps1");
    std::fs::write(&script_path, WINDOWS_OCR_SCRIPT)
        .map_err(|e| format!("Failed to prepare Windows OCR script: {}", e))?;

    let mut command = Command::new("powershell");
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .arg("-ImagePath")
        .arg(image_path);
    hide_command_window(&mut command);

    let output = command
        .output()
        .map_err(|e| format!("Failed to run Windows OCR: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Windows OCR failed: {}", stderr.trim()));
    }

    use base64::Engine;

    let encoded = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let text_bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .map_err(|e| format!("Windows OCR returned invalid base64: {}", e))?;
    let text = String::from_utf8(text_bytes)
        .map_err(|e| format!("Windows OCR returned invalid UTF-8: {}", e))?
        .trim()
        .to_string();
    let text = normalize_windows_ocr_text(&text);
    if text.is_empty() {
        return Err("No text recognized".to_string());
    }

    log::info!(
        "[OCR] Windows OCR completed: {} chars from {}",
        text.len(),
        image_path
    );
    Ok(text)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::normalize_windows_ocr_text;

    #[test]
    fn windows_ocr_normalization_removes_spaces_between_cjk_chars() {
        let input = "已 修 好 openwiki. exe ， 进 程 正 常\nnpm run build 通 过";
        let output = normalize_windows_ocr_text(input);
        assert_eq!(output, "已修好 openwiki. exe，进程正常\nnpm run build 通过");
    }
}
