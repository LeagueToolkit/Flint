/**
 * Project Tab Store
 * Manages multi-tab workspace, file trees, and folder expansion state
 */

import { create } from 'zustand';
import type { ProjectTab, Project, FileTreeNode } from '../types';

interface ProjectTabState {
  openTabs: ProjectTab[];
  activeTabId: string | null;

  // Actions
  addTab: (project: Project, path: string) => void;
  removeTab: (tabId: string) => { newActiveId: string | null; remainingTabs: ProjectTab[] };
  switchTab: (tabId: string) => void;
  updateTab: (tabId: string, updates: Partial<ProjectTab>) => void;
  setFileTree: (tabId: string, tree: FileTreeNode | null) => void;
  toggleFolder: (tabId: string, folderPath: string) => void;
  bulkSetFolders: (tabId: string, paths: string[], expand: boolean) => void;
  setSelectedFile: (tabId: string, filePath: string | null) => void;
}

// Helper to generate unique tab IDs
let tabIdCounter = 0;
function generateTabId(): string {
  return `tab-${Date.now()}-${++tabIdCounter}`;
}

export const useProjectTabStore = create<ProjectTabState>((set, get) => ({
  openTabs: [],
  activeTabId: null,

  addTab: (project, path) => {
    const { openTabs } = get();
    // Check if this project is already open
    const existingTab = openTabs.find(t => t.projectPath === path);
    if (existingTab) {
      // Switch to existing tab
      set({ activeTabId: existingTab.id });
      return;
    }
    // Create new tab
    const newTab: ProjectTab = {
      id: generateTabId(),
      project,
      projectPath: path,
      selectedFile: null,
      fileTree: null,
      expandedFolders: new Set(),
    };
    set({
      openTabs: [...openTabs, newTab],
      activeTabId: newTab.id,
    });
  },

  removeTab: (tabId) => {
    const { openTabs, activeTabId } = get();
    const newTabs = openTabs.filter(t => t.id !== tabId);
    let newActiveId = activeTabId;

    // If we closed the active tab, switch to another
    if (activeTabId === tabId) {
      const closedIndex = openTabs.findIndex(t => t.id === tabId);
      if (newTabs.length > 0) {
        // Switch to previous tab, or first if we closed the first
        const newIndex = Math.max(0, closedIndex - 1);
        newActiveId = newTabs[newIndex]?.id || null;
      } else {
        newActiveId = null;
      }
    }

    set({
      openTabs: newTabs,
      activeTabId: newActiveId,
    });

    return { newActiveId, remainingTabs: newTabs };
  },

  switchTab: (tabId) => {
    const { openTabs } = get();
    const tab = openTabs.find(t => t.id === tabId);
    if (tab) {
      set({ activeTabId: tabId });
    }
  },

  updateTab: (tabId, updates) => {
    set((state) => ({
      openTabs: state.openTabs.map(t =>
        t.id === tabId ? { ...t, ...updates } : t
      ),
    }));
  },

  setFileTree: (tabId, tree) => {
    set((state) => ({
      openTabs: state.openTabs.map(t =>
        t.id === tabId ? { ...t, fileTree: tree } : t
      ),
    }));
  },

  toggleFolder: (tabId, folderPath) => {
    set((state) => ({
      openTabs: state.openTabs.map(t => {
        if (t.id !== tabId) return t;
        const newExpanded = new Set(t.expandedFolders);
        if (newExpanded.has(folderPath)) {
          newExpanded.delete(folderPath);
        } else {
          newExpanded.add(folderPath);
        }
        return { ...t, expandedFolders: newExpanded };
      }),
    }));
  },

  bulkSetFolders: (tabId, paths, expand) => {
    set((state) => ({
      openTabs: state.openTabs.map(t => {
        if (t.id !== tabId) return t;
        const newExpanded = new Set(t.expandedFolders);
        for (const p of paths) {
          if (expand) newExpanded.add(p);
          else newExpanded.delete(p);
        }
        return { ...t, expandedFolders: newExpanded };
      }),
    }));
  },

  setSelectedFile: (tabId, filePath) => {
    set((state) => ({
      openTabs: state.openTabs.map(t =>
        t.id === tabId ? { ...t, selectedFile: filePath } : t
      ),
    }));
  },
}));
