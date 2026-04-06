/**
 * Config Store
 * Manages League paths, creator settings, and user preferences
 * Persists to %APPDATA%/Flint/settings.json via Tauri IPC
 */

import { create } from 'zustand';
import { getSettings, saveSettings, migrateFromLocalStorage, migrateProjects, loadTheme } from '../api';
import type { FlintSettings } from '../api';
import type { RecentProject, SavedProject } from '../types';

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
  savedProjects: SavedProject[];
  binConverterEngine: 'ltk' | 'jade';
  jadePath: string | null;
  quartzPath: string | null;
  selectedTheme: string | null;

  /** Whether the store has finished loading from disk */
  _hydrated: boolean;

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
  setSavedProjects: (projects: SavedProject[]) => void;
  addSavedProject: (project: SavedProject) => void;
  removeSavedProject: (projectId: string) => void;
  setBinConverterEngine: (engine: 'ltk' | 'jade') => void;
  setJadePath: (path: string | null) => void;
  setQuartzPath: (path: string | null) => void;
  setSelectedTheme: (themeId: string | null) => void;

  /** Load settings from disk (called once at startup) */
  hydrate: () => Promise<void>;
}

/** Persist current state to disk (fire-and-forget) */
function persistToDisk() {
  const s = useConfigStore.getState();
  if (!s._hydrated) return; // don't write until initial load is done

  const settings: FlintSettings = {
    schemaVersion: 1,
    leaguePath: s.leaguePath,
    leaguePathPbe: s.leaguePathPbe,
    defaultProjectPath: s.defaultProjectPath,
    creatorName: s.creatorName,
    autoUpdateEnabled: s.autoUpdateEnabled,
    skippedUpdateVersion: s.skippedUpdateVersion,
    recentProjects: s.recentProjects,
    savedProjects: s.savedProjects,
    ltkManagerModPath: s.ltkManagerModPath,
    autoSyncToLauncher: s.autoSyncToLauncher,
    binConverterEngine: s.binConverterEngine,
    jadePath: s.jadePath,
    quartzPath: s.quartzPath,
    selectedTheme: s.selectedTheme,
  };

  saveSettings(settings).catch((err) => {
    console.error('[Config] Failed to persist settings:', err);
  });
}

/** Apply a theme's CSS variables to :root */
export function applyThemeColors(colors: Record<string, string>) {
  const root = document.documentElement;
  for (const [variable, value] of Object.entries(colors)) {
    root.style.setProperty(variable, value);
  }
}

/** Clear all theme overrides (revert to CSS defaults) */
export function clearThemeOverrides() {
  const root = document.documentElement;
  // Remove inline style properties — the CSS :root declarations take over
  root.removeAttribute('style');
}

/** Load and apply a theme by ID. Returns true if applied. */
export async function applyThemeById(themeId: string | null): Promise<boolean> {
  if (!themeId) {
    clearThemeOverrides();
    return true;
  }
  try {
    const theme = await loadTheme(themeId);
    const colors = (theme as { colors?: Record<string, string> }).colors;
    if (colors && typeof colors === 'object') {
      clearThemeOverrides();
      applyThemeColors(colors);
      return true;
    }
  } catch (err) {
    console.warn(`[Theme] Failed to load theme '${themeId}':`, err);
  }
  return false;
}

export const useConfigStore = create<ConfigState>()((set) => ({
  leaguePath: null,
  leaguePathPbe: null,
  defaultProjectPath: null,
  creatorName: null,
  autoUpdateEnabled: true,
  skippedUpdateVersion: null,
  recentProjects: [],
  ltkManagerModPath: null,
  autoSyncToLauncher: false,
  savedProjects: [],
  binConverterEngine: 'ltk',
  jadePath: null,
  quartzPath: null,
  selectedTheme: null,
  _hydrated: false,

  setLeaguePath: (path) => { set({ leaguePath: path }); persistToDisk(); },
  setLeaguePathPbe: (path) => { set({ leaguePathPbe: path }); persistToDisk(); },
  setDefaultProjectPath: (path) => { set({ defaultProjectPath: path }); persistToDisk(); },
  setCreatorName: (name) => { set({ creatorName: name }); persistToDisk(); },
  setAutoUpdateEnabled: (enabled) => { set({ autoUpdateEnabled: enabled }); persistToDisk(); },
  setSkippedUpdateVersion: (version) => { set({ skippedUpdateVersion: version }); persistToDisk(); },
  setRecentProjects: (projects) => { set({ recentProjects: projects }); persistToDisk(); },
  setLtkManagerModPath: (path) => { set({ ltkManagerModPath: path }); persistToDisk(); },
  setAutoSyncToLauncher: (enabled) => { set({ autoSyncToLauncher: enabled }); persistToDisk(); },
  setBinConverterEngine: (engine) => { set({ binConverterEngine: engine }); persistToDisk(); },
  setJadePath: (path) => { set({ jadePath: path }); persistToDisk(); },
  setQuartzPath: (path) => { set({ quartzPath: path }); persistToDisk(); },
  setSavedProjects: (projects) => { set({ savedProjects: projects }); persistToDisk(); },
  addSavedProject: (project) => {
    set((state) => {
      const filtered = state.savedProjects.filter(p => p.path !== project.path);
      return { savedProjects: [project, ...filtered] };
    });
    persistToDisk();
  },
  removeSavedProject: (projectId) => {
    set((state) => ({
      savedProjects: state.savedProjects.filter(p => p.id !== projectId),
    }));
    persistToDisk();
  },
  setSelectedTheme: (themeId) => {
    set({ selectedTheme: themeId });
    persistToDisk();
    applyThemeById(themeId);
  },

  hydrate: async () => {
    // 1. Check if there's old localStorage data to migrate
    const legacyRaw = localStorage.getItem('flint_settings');
    if (legacyRaw) {
      try {
        await migrateFromLocalStorage(legacyRaw);
        localStorage.removeItem('flint_settings');
      } catch (err) {
        console.warn('[Config] localStorage migration failed:', err);
      }
    }

    // 2. Migrate projects from old RitoShark/Flint/Projects to Flint/projects/
    try {
      const result = await migrateProjects();
      if (result.moved > 0) {
        console.log(`[Config] Migrated ${result.moved} projects to Flint home`);
      }
    } catch (err) {
      console.warn('[Config] Project migration failed:', err);
    }

    // 3. Load settings from disk (after migration so paths are up-to-date)
    try {
      const s = await getSettings();
      set({
        leaguePath: s.leaguePath,
        leaguePathPbe: s.leaguePathPbe,
        defaultProjectPath: s.defaultProjectPath,
        creatorName: s.creatorName,
        autoUpdateEnabled: s.autoUpdateEnabled,
        skippedUpdateVersion: s.skippedUpdateVersion,
        recentProjects: (s.recentProjects ?? []) as RecentProject[],
        savedProjects: (s.savedProjects ?? []) as SavedProject[],
        ltkManagerModPath: s.ltkManagerModPath,
        autoSyncToLauncher: s.autoSyncToLauncher,
        binConverterEngine: (s.binConverterEngine === 'jade' ? 'jade' : 'ltk') as 'ltk' | 'jade',
        jadePath: s.jadePath,
        quartzPath: s.quartzPath,
        selectedTheme: s.selectedTheme ?? null,
        _hydrated: true,
      });

      // 4. Apply saved theme
      if (s.selectedTheme) {
        applyThemeById(s.selectedTheme);
      }
    } catch (err) {
      console.error('[Config] Failed to load settings from disk:', err);
      set({ _hydrated: true }); // still mark hydrated so the app can function with defaults
    }
  },
}));
