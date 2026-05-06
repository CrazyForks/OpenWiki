import { invoke } from "@tauri-apps/api/core";

/**
 * In-app update notification — Plan B (simple GitHub Releases polling).
 *
 * Backend: src-tauri/src/update/mod.rs
 */

export interface UpdateInfo {
  /** Stripped latest version, e.g. "0.1.3" */
  version: string;
  /** Current running version */
  current_version: string;
  /** Release title */
  name: string;
  /** Release notes (Markdown) */
  body: string;
  /** GitHub release page URL */
  url: string;
  /** ISO-8601 publish timestamp */
  published_at: string;
}

export interface UpdateSettings {
  check_enabled: boolean;
  current_version: string;
  releases_url: string;
}

/**
 * Manually check for updates right now.
 * Bypasses the "dismissed version" state — always returns the result.
 * Returns null when already on the latest version.
 */
export async function checkForUpdateManual(): Promise<UpdateInfo | null> {
  return invoke("check_for_update_manual");
}

/** Toggle the on-launch auto-check. */
export async function setUpdateCheckEnabled(enabled: boolean): Promise<void> {
  return invoke("set_update_check_enabled", { enabled });
}

/** Fetch the current update settings for the Settings UI. */
export async function getUpdateSettings(): Promise<UpdateSettings> {
  return invoke("get_update_settings");
}
