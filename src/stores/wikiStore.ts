import { create } from "zustand";
import type { WikiPage, WikiStats, WikiGraphData } from "../types/wiki";
import * as wikiService from "../services/wikiService";

interface WikiState {
  // Browse
  pages: WikiPage[];
  selectedPage: WikiPage | null;
  isLoadingPages: boolean;
  filterType: string | null;
  searchQuery: string;

  // Graph
  graphData: WikiGraphData | null;
  isLoadingGraph: boolean;

  // Stats
  stats: WikiStats | null;

  // Error
  error: string | null;

  // Actions
  loadPages: (opts?: { page_type?: string }) => Promise<void>;
  searchPages: (query: string) => Promise<void>;
  selectPage: (id: string) => Promise<void>;
  clearSelection: () => void;
  setFilterType: (type: string | null) => void;
  loadGraph: () => Promise<void>;
  loadStats: () => Promise<void>;
  deletePage: (id: string) => Promise<void>;
}

export const useWikiStore = create<WikiState>((set, get) => ({
  pages: [],
  selectedPage: null,
  isLoadingPages: false,
  filterType: null,
  searchQuery: "",
  graphData: null,
  isLoadingGraph: false,
  stats: null,
  error: null,

  loadPages: async (opts) => {
    set({ isLoadingPages: true, error: null });
    try {
      const pages = await wikiService.getWikiPages({
        page_type: opts?.page_type ?? get().filterType ?? undefined,
      });
      set({ pages, isLoadingPages: false });
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : String(e),
        isLoadingPages: false,
      });
    }
  },

  searchPages: async (query: string) => {
    set({ isLoadingPages: true, error: null, searchQuery: query });
    try {
      if (!query.trim()) {
        const pages = await wikiService.getWikiPages({
          page_type: get().filterType ?? undefined,
        });
        set({ pages, isLoadingPages: false });
      } else {
        const pages = await wikiService.searchWiki(query);
        set({ pages, isLoadingPages: false });
      }
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : String(e),
        isLoadingPages: false,
      });
    }
  },

  selectPage: async (id: string) => {
    try {
      const page = await wikiService.getWikiPage(id);
      set({ selectedPage: page });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : String(e) });
    }
  },

  clearSelection: () => set({ selectedPage: null }),

  setFilterType: (type: string | null) => {
    set({ filterType: type });
    get().loadPages({ page_type: type ?? undefined });
  },

  loadGraph: async () => {
    set({ isLoadingGraph: true });
    try {
      const graphData = await wikiService.getWikiGraph();
      set({ graphData, isLoadingGraph: false });
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : String(e),
        isLoadingGraph: false,
      });
    }
  },

  loadStats: async () => {
    try {
      const stats = await wikiService.getWikiStats();
      set({ stats });
    } catch (e) {
      console.error("Failed to load wiki stats:", e);
    }
  },

  deletePage: async (id: string) => {
    try {
      await wikiService.deleteWikiPage(id);
      set({
        pages: get().pages.filter((p) => p.id !== id),
        selectedPage: get().selectedPage?.id === id ? null : get().selectedPage,
      });
      get().loadStats();
    } catch (e) {
      set({ error: e instanceof Error ? e.message : String(e) });
    }
  },
}));
