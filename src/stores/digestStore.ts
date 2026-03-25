import { create } from "zustand";
import type { CapturedContent } from "../types/content";
import {
  getDigestItems,
  digestItem,
  type DigestAction,
} from "../services/digestService";

interface DigestState {
  items: CapturedContent[];
  remaining: number;
  isLoading: boolean;
  dailyTarget: number;
  digestedToday: number;
  error: string | null;

  loadItems: () => Promise<void>;
  doDigest: (id: string, action: DigestAction) => Promise<void>;
}

export const useDigestStore = create<DigestState>((set, get) => ({
  items: [],
  remaining: 0,
  isLoading: false,
  dailyTarget: 5,
  digestedToday: 0,
  error: null,

  loadItems: async () => {
    set({ isLoading: true, error: null });
    try {
      const resp = await getDigestItems();
      set({
        items: resp.items,
        remaining: resp.remaining,
        isLoading: false,
      });
    } catch (e) {
      set({
        isLoading: false,
        error: e instanceof Error ? e.message : String(e),
      });
    }
  },

  doDigest: async (id: string, action: DigestAction) => {
    try {
      await digestItem(id, action);
      const { items, digestedToday, remaining } = get();
      set({
        items: items.filter((item) => item.id !== id),
        digestedToday: digestedToday + 1,
        remaining: Math.max(0, remaining - 1),
        error: null,
      });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : String(e) });
    }
  },
}));
