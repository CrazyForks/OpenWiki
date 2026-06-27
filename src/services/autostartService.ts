import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";

export async function isLaunchAtStartupEnabled(): Promise<boolean> {
  return isEnabled();
}

export async function setLaunchAtStartupEnabled(enabled: boolean): Promise<void> {
  if (enabled) {
    await enable();
  } else {
    await disable();
  }
}
