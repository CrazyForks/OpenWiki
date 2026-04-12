import { create } from "zustand";
import type { CapturedContent } from "../types/content";

interface ContentState {
  contents: CapturedContent[];
  isLoading: boolean;
  isLoadingMore: boolean;
  hasMore: boolean;
  totalCount: number;
  highlightedIds: string[];
  scrollToId: string | null;
  setContents: (contents: CapturedContent[]) => void;
  setIsLoading: (loading: boolean) => void;
  setIsLoadingMore: (loading: boolean) => void;
  setHasMore: (v: boolean) => void;
  setTotalCount: (n: number) => void;
  addContent: (content: CapturedContent) => void;
  appendContents: (items: CapturedContent[]) => void;
  removeContent: (id: string) => void;
  updateContent: (updated: CapturedContent) => void;
  setHighlightedIds: (ids: string[]) => void;
  setScrollToId: (id: string | null) => void;
  clearHighlights: () => void;
}

export const useContentStore = create<ContentState>((set) => ({
  contents: [],
  isLoading: false,
  isLoadingMore: false,
  hasMore: true,
  totalCount: 0,
  highlightedIds: [],
  scrollToId: null,
  setContents: (contents) => set({ contents }),
  setIsLoading: (loading) => set({ isLoading: loading }),
  setIsLoadingMore: (loading) => set({ isLoadingMore: loading }),
  setHasMore: (v) => set({ hasMore: v }),
  setTotalCount: (n) => set({ totalCount: n }),
  addContent: (content) =>
    set((state) => ({ contents: [content, ...state.contents] })),
  appendContents: (items) =>
    set((state) => {
      const existingIds = new Set(state.contents.map((c) => c.id));
      const newItems = items.filter((c) => !existingIds.has(c.id));
      return { contents: [...state.contents, ...newItems] };
    }),
  removeContent: (id) =>
    set((state) => ({
      contents: state.contents.filter((c) => c.id !== id),
    })),
  updateContent: (updated) =>
    set((state) => ({
      contents: state.contents.map((c) => (c.id === updated.id ? updated : c)),
    })),
  setHighlightedIds: (ids) => set({ highlightedIds: ids, scrollToId: ids[0] ?? null }),
  setScrollToId: (id) => set({ scrollToId: id }),
  clearHighlights: () => set({ highlightedIds: [], scrollToId: null }),
}));
