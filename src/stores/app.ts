import { create } from 'zustand';
import type { VaultEntry, VaultFolder, VaultTag } from '../types';
import * as api from '../api';

export interface Notification {
  id: string;
  message: string;
  type: 'success' | 'error' | 'info';
}

interface AppState {
  // Theme
  darkMode: boolean;
  toggleDarkMode: () => void;

  // Language
  language: 'en' | 'fr';
  setLanguage: (lang: 'en' | 'fr') => void;

  // Vault state
  vaultOpen: boolean;
  setVaultOpen: (open: boolean) => void;

  // Data
  entries: VaultEntry[];
  folders: VaultFolder[];
  tags: VaultTag[];
  setEntries: (entries: VaultEntry[]) => void;
  setFolders: (folders: VaultFolder[]) => void;
  setTags: (tags: VaultTag[]) => void;
  refreshData: () => Promise<void>;

  // Selection
  selectedEntryId: string | null;
  selectedFolderId: string | null;
  selectedTagId: string | null;
  viewFilter: 'all' | 'favorites' | 'folder' | 'tag';
  setSelectedEntry: (id: string | null) => void;
  setSelectedFolder: (id: string | null) => void;
  setSelectedTag: (id: string | null) => void;
  setViewFilter: (filter: 'all' | 'favorites' | 'folder' | 'tag', id?: string | null) => void;

  // Editing
  editingEntry: VaultEntry | null;
  showEntryForm: boolean;
  startNewEntry: () => void;
  startEditEntry: (entry: VaultEntry) => void;
  closeEntryForm: () => void;

  // Generator
  showGenerator: boolean;
  toggleGenerator: () => void;

  // Search
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  searchResults: VaultEntry[] | null;
  setSearchResults: (results: VaultEntry[] | null) => void;

  // Notifications
  notifications: Notification[];
  addNotification: (message: string, type: 'success' | 'error' | 'info') => void;
  removeNotification: (id: string) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  // Theme
  darkMode: true,
  toggleDarkMode: () => set((state) => ({ darkMode: !state.darkMode })),

  // Language
  language: 'en',
  setLanguage: (lang) => set({ language: lang }),

  // Vault state
  vaultOpen: false,
  setVaultOpen: (open) => set({ vaultOpen: open }),

  // Data
  entries: [],
  folders: [],
  tags: [],
  setEntries: (entries) => set({ entries }),
  setFolders: (folders) => set({ folders }),
  setTags: (tags) => set({ tags }),
  refreshData: async () => {
    try {
      const [entries, folders, tags] = await Promise.all([
        api.getAllEntries(),
        api.getAllFolders(),
        api.getAllTags(),
      ]);
      set({ entries, folders, tags });
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Failed to refresh data';
      get().addNotification(message, 'error');
    }
  },

  // Selection
  selectedEntryId: null,
  selectedFolderId: null,
  selectedTagId: null,
  viewFilter: 'all',
  setSelectedEntry: (id) => set({ selectedEntryId: id }),
  setSelectedFolder: (id) => set({ selectedFolderId: id }),
  setSelectedTag: (id) => set({ selectedTagId: id }),
  setViewFilter: (filter, id) => {
    if (filter === 'folder') set({ viewFilter: 'folder', selectedFolderId: id ?? null });
    else if (filter === 'tag') set({ viewFilter: 'tag', selectedTagId: id ?? null });
    else set({ viewFilter: filter, selectedFolderId: null, selectedTagId: null });
  },

  // Editing
  editingEntry: null,
  showEntryForm: false,
  startNewEntry: () => set({ editingEntry: null, showEntryForm: true }),
  startEditEntry: (entry) => set({ editingEntry: { ...entry }, showEntryForm: true }),
  closeEntryForm: () => set({ editingEntry: null, showEntryForm: false }),

  // Generator
  showGenerator: false,
  toggleGenerator: () => set((state) => ({ showGenerator: !state.showGenerator })),

  // Search
  searchQuery: '',
  setSearchQuery: (query) => set({ searchQuery: query }),
  searchResults: null,
  setSearchResults: (results) => set({ searchResults: results }),

  // Notifications
  notifications: [],
  addNotification: (message, type) => {
    const id = Date.now().toString() + Math.random().toString(36).slice(2);
    set((state) => ({
      notifications: [...state.notifications, { id, message, type }],
    }));
    setTimeout(() => {
      get().removeNotification(id);
    }, 3000);
  },
  removeNotification: (id) => {
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    }));
  },
}));
