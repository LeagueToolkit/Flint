/**
 * Config Store
 * Manages League paths, creator settings, and user preferences
 * Persists to localStorage using Zustand persist middleware
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { RecentProject } from '../types';

interface ConfigState {
  leaguePath: string | null;
  leaguePathPbe: string | null;
  defaultProjectPath: string | null;
  creatorName: string | null;
  autoUpdateEnabled: boolean;
  skippedUpdateVersion: string | null;
  recentProjects: RecentProject[];
  ltkManagerModPath: string | null;
  autoSyncToLauncher: boolean;

  // Actions
  setLeaguePath: (path: string | null) => void;
  setLeaguePathPbe: (path: string | null) => void;
  setDefaultProjectPath: (path: string | null) => void;
  setCreatorName: (name: string | null) => void;
  setAutoUpdateEnabled: (enabled: boolean) => void;
  setSkippedUpdateVersion: (version: string | null) => void;
  setRecentProjects: (projects: RecentProject[]) => void;
  setLtkManagerModPath: (path: string | null) => void;
  setAutoSyncToLauncher: (enabled: boolean) => void;
}

export const useConfigStore = create<ConfigState>()(
  persist(
    (set) => ({
      leaguePath: null,
      leaguePathPbe: null,
      defaultProjectPath: null,
      creatorName: null,
      autoUpdateEnabled: true,
      skippedUpdateVersion: null,
      recentProjects: [],
      ltkManagerModPath: null,
      autoSyncToLauncher: false,

      setLeaguePath: (path) => set({ leaguePath: path }),
      setLeaguePathPbe: (path) => set({ leaguePathPbe: path }),
      setDefaultProjectPath: (path) => set({ defaultProjectPath: path }),
      setCreatorName: (name) => set({ creatorName: name }),
      setAutoUpdateEnabled: (enabled) => set({ autoUpdateEnabled: enabled }),
      setSkippedUpdateVersion: (version) => set({ skippedUpdateVersion: version }),
      setRecentProjects: (projects) => set({ recentProjects: projects }),
      setLtkManagerModPath: (path) => set({ ltkManagerModPath: path }),
      setAutoSyncToLauncher: (enabled) => set({ autoSyncToLauncher: enabled }),
    }),
    {
      name: 'flint_settings', // localStorage key
      partialize: (state) => ({
        leaguePath: state.leaguePath,
        leaguePathPbe: state.leaguePathPbe,
        defaultProjectPath: state.defaultProjectPath,
        creatorName: state.creatorName,
        autoUpdateEnabled: state.autoUpdateEnabled,
        skippedUpdateVersion: state.skippedUpdateVersion,
        recentProjects: state.recentProjects,
        ltkManagerModPath: state.ltkManagerModPath,
        autoSyncToLauncher: state.autoSyncToLauncher,
      }),
    }
  )
);
