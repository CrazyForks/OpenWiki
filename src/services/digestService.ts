import { invoke } from "@tauri-apps/api/core";
import type { CapturedContent } from "../types/content";

export interface DigestResponse {
  items: CapturedContent[];
  remaining: number;
}

export type DigestAction = "keep" | "archive" | "pin";

export async function getDigestItems(): Promise<DigestResponse> {
  return invoke<DigestResponse>("get_digest_items");
}

export async function digestItem(
  id: string,
  action: DigestAction
): Promise<void> {
  return invoke("digest_item", { id, action });
}
