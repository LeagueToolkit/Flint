/**
 * App Metadata Store
 * Manages app status, hash info, logs, and verbose logging settings
 */

import { create } from 'zustand';
import type { LogEntry } from '../types';

interface AppMetadataState {
  status: 'ready' | 'working' | 'error';
  statusMessage: string;
  hashesLoaded: boolean;
  hashCount: number;
  verboseLogging: boolean;
  logs: LogEntry[];
  logPanelExpanded: boolean;
  fileVersions: Record<string, number>; // Track file modification versions for hot reload
  fileTreeVersion: number; // Incremented when file tree structure changes (create/remove)
  fileStatuses: Record<string, 'new' | 'modified'>; // Track file modification status for VFS indicators

  // Actions
  setStatus: (status: AppMetadataState['status'], message: string) => void;
  setWorking: (message?: string) => void;
  setReady: (message?: string) => void;
  setError: (message: string) => void;
  setHashInfo: (loaded: boolean, count: number) => void;
  setVerboseLogging: (enabled: boolean) => void;
  addLog: (level: LogEntry['level'], message: string) => void;
  clearLogs: () => void;
  toggleLogPanel: () => void;
  incrementFileVersion: (filePath: string) => void;
  getFileVersion: (filePath: string) => number;
  incrementFileTreeVersion: () => void;
  setFileStatus: (filePath: string, status: 'new' | 'modified' | null) => void;
  clearFileStatuses: () => void;
}

let logIdCounter = 0;

export const useAppMetadataStore = create<AppMetadataState>((set, get) => ({
  status: 'ready',
  statusMessage: 'Ready',
  hashesLoaded: false,
  hashCount: 0,
  verboseLogging: false,
  logs: [],
  logPanelExpanded: false,
  fileVersions: {},
  fileTreeVersion: 0,
  fileStatuses: {},

  setStatus: (status, message) => set({ status, statusMessage: message }),
  setWorking: (message = 'Working...') => set({ status: 'working', statusMessage: message }),
  setReady: (message = 'Ready') => set({ status: 'ready', statusMessage: message }),
  setError: (message) => set({ status: 'error', statusMessage: message }),
  setHashInfo: (loaded, count) => set({ hashesLoaded: loaded, hashCount: count }),
  setVerboseLogging: (enabled) => set({ verboseLogging: enabled }),
  addLog: (level, message) => set((state) => ({
    logs: [...state.logs, {
      id: ++logIdCounter,
      timestamp: Date.now(),
      level,
      message,
    }].slice(-100), // Keep last 100
  })),
  clearLogs: () => set({ logs: [] }),
  toggleLogPanel: () => set((state) => ({ logPanelExpanded: !state.logPanelExpanded })),
  incrementFileVersion: (filePath) => {
    const key = filePath.replaceAll('\\', '/');
    set((state) => ({
      fileVersions: {
        ...state.fileVersions,
        [key]: (state.fileVersions[key] || 0) + 1,
      },
    }));
  },
  getFileVersion: (filePath) => get().fileVersions[filePath.replaceAll('\\', '/')] || 0,
  incrementFileTreeVersion: () => set((state) => ({ fileTreeVersion: state.fileTreeVersion + 1 })),
  setFileStatus: (filePath, status) => {
    const key = filePath.replaceAll('\\', '/');
    set((state) => {
      if (status === null) {
        const { [key]: _, ...rest } = state.fileStatuses;
        return { fileStatuses: rest };
      }
      return {
        fileStatuses: {
          ...state.fileStatuses,
          [key]: status,
        },
      };
    });
  },
  clearFileStatuses: () => set({ fileStatuses: {} }),
}));
