import { invoke } from "@tauri-apps/api/core";

export interface XReaderStatus {
  installed: boolean;
  supported_platforms: string[];
  install_command: string;
}

export async function getSettings(): Promise<Record<string, string>> {
  return invoke("get_settings");
}

export async function updateSetting(
  key: string,
  value: string
): Promise<void> {
  return invoke("update_setting", { key, value });
}

export async function checkXReaderStatus(): Promise<XReaderStatus> {
  return invoke("check_xreader_status");
}
