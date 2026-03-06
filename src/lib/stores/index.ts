/**
 * Root Store Index
 * Combines all domain stores and provides backward-compatible useAppState() hook
 */

import React from 'react';
import { useAppMetadataStore } from './appMetadataStore';
import { useConfigStore } from './configStore';
import { useProjectTabStore } from './projectTabStore';
import { useNavigationStore } from './navigationStore';
import { useWadExtractStore } from './wadExtractStore';
import { useWadExplorerStore } from './wadExplorerStore';
import { useChampionStore } from './championStore';
import { useModalStore } from './modalStore';
import { useNotificationStore } from './notificationStore';
import { navigationCoordinator } from './navigationCoordinator';
import type { AppState } from '../types';

// Re-export individual stores
export {
  useAppMetadataStore,
  useConfigStore,
  useProjectTabStore,
  useNavigationStore,
  useWadExtractStore,
  useWadExplorerStore,
  useChampionStore,
  useModalStore,
  useNotificationStore,
};

/**
 * Combined hook that provides backward compatibility with the old useAppState() API
 * Components can continue using state.xyz pattern or migrate to individual stores
 */
export function useAppState() {
  const appMetadata = useAppMetadataStore();
  const config = useConfigStore();
  const projectTab = useProjectTabStore();
  const navigation = useNavigationStore();
  const wadExtract = useWadExtractStore();
  const wadExplorer = useWadExplorerStore();
  const champion = useChampionStore();
  const modal = useModalStore();
  const notification = useNotificationStore();

  // Combined state object (for components that read state.xyz)
  const state: AppState = {
    // App metadata
    status: appMetadata.status,
    statusMessage: appMetadata.statusMessage,
    hashesLoaded: appMetadata.hashesLoaded,
    hashCount: appMetadata.hashCount,
    verboseLogging: appMetadata.verboseLogging,
    logs: appMetadata.logs,
    logPanelExpanded: appMetadata.logPanelExpanded,

    // Config
    leaguePath: config.leaguePath,
    leaguePathPbe: config.leaguePathPbe,
    defaultProjectPath: config.defaultProjectPath,
    creatorName: config.creatorName,
    autoUpdateEnabled: config.autoUpdateEnabled,
    skippedUpdateVersion: config.skippedUpdateVersion,

    // Project tabs
    openTabs: projectTab.openTabs,
    activeTabId: projectTab.activeTabId,
    recentProjects: config.recentProjects,

    // Navigation
    currentView: navigation.currentView,

    // WAD extract
    extractSessions: wadExtract.extractSessions,
    activeExtractId: wadExtract.activeExtractId,

    // WAD explorer
    wadExplorer: {
      isOpen: wadExplorer.isOpen,
      wads: wadExplorer.wads,
      scanStatus: wadExplorer.scanStatus,
      scanError: wadExplorer.scanError,
      selected: wadExplorer.selected,
      expandedWads: wadExplorer.expandedWads,
      expandedFolders: wadExplorer.expandedFolders,
      searchQuery: wadExplorer.searchQuery,
      checkedFiles: wadExplorer.checkedFiles,
    },

    // Champions
    champions: champion.champions,
    championsLoaded: champion.championsLoaded,

    // Modals
    activeModal: modal.activeModal,
    modalOptions: modal.modalOptions,
    confirmDialog: modal.confirmDialog,
    contextMenu: modal.contextMenu,

    // Notifications
    toasts: notification.toasts,
  };

  // Legacy dispatch function for backward compatibility
  // Maps old action types to new store calls
  const dispatch = (action: any) => {
    switch (action.type) {
      // App metadata
      case 'SET_STATUS':
        appMetadata.setStatus(action.payload.status, action.payload.message);
        break;
      case 'ADD_LOG':
        appMetadata.addLog(action.payload.level, action.payload.message);
        break;
      case 'CLEAR_LOGS':
        appMetadata.clearLogs();
        break;
      case 'TOGGLE_LOG_PANEL':
        appMetadata.toggleLogPanel();
        break;

      // Modals
      case 'OPEN_MODAL':
        modal.openModal(action.payload.modal, action.payload.options);
        break;
      case 'CLOSE_MODAL':
        modal.closeModal();
        break;
      case 'OPEN_CONTEXT_MENU':
        modal.openContextMenu(action.payload.x, action.payload.y, action.payload.options);
        break;
      case 'CLOSE_CONTEXT_MENU':
        modal.closeContextMenu();
        break;
      case 'OPEN_CONFIRM_DIALOG':
        modal.openConfirmDialog(action.payload);
        break;
      case 'CLOSE_CONFIRM_DIALOG':
        modal.closeConfirmDialog();
        break;

      // Notifications
      case 'ADD_TOAST':
        notification.showToast(action.payload.type, action.payload.message, { suggestion: action.payload.suggestion });
        break;
      case 'REMOVE_TOAST':
        notification.dismissToast(action.payload);
        break;

      // Project tabs
      case 'ADD_TAB':
        projectTab.addTab(action.payload.project, action.payload.path);
        navigation.setView('preview');
        break;
      case 'REMOVE_TAB':
        navigationCoordinator.removeTabWithFallback(action.payload);
        break;
      case 'SWITCH_TAB':
        projectTab.switchTab(action.payload);
        navigation.setView('preview');
        break;
      case 'UPDATE_TAB':
        projectTab.updateTab(action.payload.tabId, action.payload.updates);
        break;
      case 'SET_TAB_FILE_TREE':
        projectTab.setFileTree(action.payload.tabId, action.payload.fileTree);
        break;
      case 'TOGGLE_TAB_FOLDER':
        projectTab.toggleFolder(action.payload.tabId, action.payload.folderPath);
        break;
      case 'SET_TAB_SELECTED_FILE':
        projectTab.setSelectedFile(action.payload.tabId, action.payload.filePath);
        break;

      // Legacy project actions (redirect to tab actions)
      case 'SET_PROJECT':
        if (action.payload.project && action.payload.path) {
          projectTab.addTab(action.payload.project, action.payload.path);
          navigation.setView('preview');
        } else {
          // Close all tabs
          projectTab.openTabs.forEach(t => navigationCoordinator.removeTabWithFallback(t.id));
        }
        break;
      case 'SET_FILE_TREE':
        // Use getState() to get current activeTabId, not captured value
        const currentTabId = useProjectTabStore.getState().activeTabId;
        if (currentTabId) {
          useProjectTabStore.getState().setFileTree(currentTabId, action.payload);
        }
        break;
      case 'TOGGLE_FOLDER':
        {
          const currentTabId = useProjectTabStore.getState().activeTabId;
          if (currentTabId) {
            useProjectTabStore.getState().toggleFolder(currentTabId, action.payload);
          }
        }
        break;
      case 'BULK_SET_FOLDERS':
        {
          const currentTabId = useProjectTabStore.getState().activeTabId;
          if (currentTabId) {
            useProjectTabStore.getState().bulkSetFolders(currentTabId, action.payload.paths, action.payload.expand);
          }
        }
        break;

      // Config
      case 'SET_RECENT_PROJECTS':
        config.setRecentProjects(action.payload);
        break;

      // Champions
      case 'SET_CHAMPIONS':
        champion.setChampions(action.payload);
        break;

      // WAD Explorer
      case 'OPEN_WAD_EXPLORER':
        navigationCoordinator.openWadExplorer();
        break;
      case 'CLOSE_WAD_EXPLORER':
        navigationCoordinator.closeWadExplorerWithFallback();
        break;
      case 'SET_WAD_EXPLORER_SCAN':
        wadExplorer.setScan(action.payload.status, action.payload.wads, action.payload.error);
        break;
      case 'SET_WAD_EXPLORER_WAD_STATUS':
        wadExplorer.setWadStatus(action.payload.wadPath, action.payload.status, action.payload.chunks, action.payload.error);
        break;
      case 'BATCH_SET_WAD_STATUSES':
        wadExplorer.batchSetWadStatuses(action.payload);
        break;
      case 'SET_WAD_EXPLORER_SELECTED':
        if (action.payload) {
          wadExplorer.setSelected(action.payload.wadPath, action.payload.hash);
        } else {
          wadExplorer.setSelected(null, null);
        }
        break;
      case 'TOGGLE_WAD_EXPLORER_WAD':
        wadExplorer.toggleWad(action.payload);
        break;
      case 'TOGGLE_WAD_EXPLORER_FOLDER':
        wadExplorer.toggleFolder(action.payload);
        break;
      case 'BULK_SET_WAD_EXPLORER_FOLDERS':
        wadExplorer.bulkSetFolders(action.payload.keys, action.payload.expand);
        break;
      case 'SET_WAD_EXPLORER_SEARCH':
        wadExplorer.setSearch(action.payload);
        break;
      case 'WAD_EXPLORER_TOGGLE_CHECK':
        wadExplorer.toggleCheck(action.payload.keys, action.payload.checked);
        break;
      case 'WAD_EXPLORER_CLEAR_CHECKS':
        wadExplorer.clearChecks();
        break;

      // Extract sessions
      case 'OPEN_EXTRACT_SESSION':
        wadExtract.openSession(action.payload.id, action.payload.wadPath);
        navigation.setView('extract');
        break;
      case 'CLOSE_EXTRACT_SESSION':
        navigationCoordinator.closeExtractSessionWithFallback(action.payload);
        break;
      case 'SWITCH_EXTRACT_TAB':
        wadExtract.switchSession(action.payload);
        navigation.setView('extract');
        break;
      case 'SET_EXTRACT_CHUNKS':
        wadExtract.setChunks(action.payload.sessionId, action.payload.chunks);
        break;
      case 'SET_EXTRACT_PREVIEW':
        wadExtract.setPreview(action.payload.sessionId, action.payload.hash);
        break;
      case 'TOGGLE_EXTRACT_FOLDER':
        wadExtract.toggleFolder(action.payload.sessionId, action.payload.folderPath);
        break;
      case 'TOGGLE_EXTRACT_CHUNK':
        wadExtract.toggleChunk(action.payload.sessionId, action.payload.hash);
        break;
      case 'SET_EXTRACT_SEARCH':
        wadExtract.setSearch(action.payload.sessionId, action.payload.query);
        break;
      case 'SET_EXTRACT_LOADING':
        wadExtract.setLoading(action.payload.sessionId, action.payload.loading);
        break;

      // Generic SET_STATE for partial updates
      case 'SET_STATE':
        if (action.payload.status !== undefined) appMetadata.setStatus(action.payload.status, action.payload.statusMessage || '');
        if (action.payload.hashesLoaded !== undefined) appMetadata.setHashInfo(action.payload.hashesLoaded, action.payload.hashCount || 0);
        if (action.payload.verboseLogging !== undefined) appMetadata.setVerboseLogging(action.payload.verboseLogging);
        if (action.payload.leaguePath !== undefined) config.setLeaguePath(action.payload.leaguePath);
        if (action.payload.leaguePathPbe !== undefined) config.setLeaguePathPbe(action.payload.leaguePathPbe);
        if (action.payload.defaultProjectPath !== undefined) config.setDefaultProjectPath(action.payload.defaultProjectPath);
        if (action.payload.creatorName !== undefined) config.setCreatorName(action.payload.creatorName);
        if (action.payload.autoUpdateEnabled !== undefined) config.setAutoUpdateEnabled(action.payload.autoUpdateEnabled);
        if (action.payload.skippedUpdateVersion !== undefined) config.setSkippedUpdateVersion(action.payload.skippedUpdateVersion);
        if (action.payload.currentView !== undefined) navigation.setView(action.payload.currentView);
        break;

      default:
        console.warn('[Zustand Migration] Unknown action type:', action.type);
    }
  };

  return {
    state,
    dispatch, // Legacy dispatch for backward compatibility

    // Convenience methods (delegate to appropriate stores)
    setStatus: appMetadata.setStatus,
    setWorking: appMetadata.setWorking,
    setReady: appMetadata.setReady,
    setError: appMetadata.setError,
    openModal: modal.openModal,
    closeModal: modal.closeModal,
    showToast: notification.showToast,
    dismissToast: notification.dismissToast,
    addLog: appMetadata.addLog,
    clearLogs: appMetadata.clearLogs,
    toggleLogPanel: appMetadata.toggleLogPanel,
    openContextMenu: modal.openContextMenu,
    closeContextMenu: modal.closeContextMenu,
    openConfirmDialog: modal.openConfirmDialog,
    closeConfirmDialog: modal.closeConfirmDialog,

    // Direct store access (for components that want to use individual stores)
    stores: {
      appMetadata,
      config,
      projectTab,
      navigation,
      wadExtract,
      wadExplorer,
      champion,
      modal,
      notification,
    },
  };
}

/**
 * Legacy AppProvider component for backward compatibility
 * With Zustand, we don't need a provider at the root, but we keep this
 * export to avoid breaking components that import AppProvider
 */
export function AppProvider({ children }: { children: React.ReactNode }) {
  return React.createElement(React.Fragment, null, children);
}
