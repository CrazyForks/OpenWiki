# OpenWiki MarkItDown Bundle

This directory is the packaging location for OpenWiki's bundled document
converter.

Run `src-tauri/scripts/setup_markitdown.sh` on macOS/Linux or
`src-tauri/scripts/setup_markitdown.ps1` on Windows before creating a release
build. The scripts build a bundled converter executable with only the
MarkItDown extras needed for PDF, DOCX, and PPTX imports.

Tauri may place that executable in the final app bundle as either
`markitdown/openwiki-markitdown` or `markitdown/bin/openwiki-markitdown`;
on Windows the executable is named `openwiki-markitdown.exe`. The app checks
all of these locations.
