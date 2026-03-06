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
}

let logIdCounter = 0;

export const useAppMetadataStore = create<AppMetadataState>((set) => ({
  status: 'ready',
  statusMessage: 'Ready',
  hashesLoaded: false,
  hashCount: 0,
  verboseLogging: false,
  logs: [],
  logPanelExpanded: false,

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
}));
