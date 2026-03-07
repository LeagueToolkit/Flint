/**
 * Flint - Main Application Component
 */

import React, { useEffect, useCallback, useRef, useState } from 'react';
import { useAppState, useAppMetadataStore, useConfigStore } from '../lib/stores';
import { navigationCoordinator } from '../lib/stores/navigationCoordinator';
import { initShortcuts, registerShortcut } from '../lib/utils';
import * as api from '../lib/api';
import * as updater from '../lib/updater';
import { listen } from '@tauri-apps/api/event';
import { invalidateCachedImage } from '../lib/imageCache';

import { TitleBar } from './TitleBar';
import { LeftPanel } from './FileTree';
import { WadBrowserPanel } from './WadBrowser';
import { WadExplorer } from './WadExplorer';
import { CenterPanel } from './CenterPanel';
import { StatusBar } from './StatusBar';
import { ContextMenu } from './ContextMenu';
import { ConfirmDialog } from './ConfirmDialog';
import { NewProjectModal } from './modals/NewProjectModal';
import { SettingsModal } from './modals/SettingsModal';
import { ExportModal } from './modals/ExportModal';
import { FirstTimeSetupModal } from './modals/FirstTimeSetupModal';
import { UpdateModal } from './modals/UpdateModal';
import { RecolorModal } from './modals/RecolorModal';
import { FixerModal } from './modals/FixerModal';
import { ToastContainer } from './Toast';

// Helper to get active tab from state
function getActiveTab(state: { activeTabId: string | null; openTabs: Array<{ id: string; project: any; projectPath: string; selectedFile: string | null }> }) {
    if (!state.activeTabId) return null;
    return state.openTabs.find(t => t.id === state.activeTabId) || null;
}

export const App: React.FC = () => {
    const { state, openModal, closeModal, setWorking, setReady, showToast } = useAppState();
    const [leftPanelWidth, setLeftPanelWidth] = useState(280);
    const resizerRef = useRef<HTMLDivElement>(null);
    const isResizingRef = useRef(false);

    // Keep a ref to always-current state so shortcut handlers are never stale
    const stateRef = useRef(state);
    useEffect(() => { stateRef.current = state; });

    // Initialize shortcuts and load data on mount
    useEffect(() => {
        initShortcuts();

        // Register shortcuts — use stateRef.current so handlers always see latest state
        registerShortcut('ctrl+n', () => openModal('newProject'));
        registerShortcut('ctrl+s', async () => {
            const activeTab = getActiveTab(stateRef.current);
            if (activeTab) {
                try {
                    setWorking('Saving...');
                    await api.saveProject(activeTab.project);
                    setReady('Saved');
                } catch (error) {
                    console.error('Failed to save:', error);
                    showToast('error', 'Save failed');
                }
            }
        });
        registerShortcut('ctrl+,', () => openModal('settings'));
        registerShortcut('ctrl+e', () => {
            const activeTab = getActiveTab(stateRef.current);
            if (activeTab) {
                openModal('export');
            }
        });
        registerShortcut('ctrl+w', () => {
            const s = stateRef.current;
            if (s.currentView === 'wad-explorer') {
                navigationCoordinator.closeWadExplorerWithFallback();
            } else if (s.currentView === 'extract' && s.activeExtractId) {
                navigationCoordinator.closeExtractSessionWithFallback(s.activeExtractId);
            } else if (s.currentView === 'preview' && s.activeTabId) {
                navigationCoordinator.removeTabWithFallback(s.activeTabId);
            }
        });
        registerShortcut('escape', () => {
            if (stateRef.current.activeModal) {
                closeModal();
            }
        });

        // Load initial data
        loadInitialData();
        // Clean stale projects
        cleanStaleProjects();
    }, []); // eslint-disable-line react-hooks/exhaustive-deps

    // Manage file watcher for auto-sync
    // Compute the current project path (useMemo to avoid triggering effect on every state change)
    const currentProjectPath = React.useMemo(() => {
        const activeTab = getActiveTab(state);
        return activeTab?.projectPath || null;
    }, [state.activeTabId, state.openTabs]);

    useEffect(() => {
        const shouldWatch = state.autoSyncToLauncher &&
                          state.ltkManagerModPath &&
                          currentProjectPath;

        if (shouldWatch) {
            console.log('[Auto-sync] Starting watcher for:', currentProjectPath);
            // Start watcher
            api.startProjectWatcher(currentProjectPath, state.ltkManagerModPath!)
                .catch(err => {
                    console.error('[Auto-sync] Failed to start watcher:', err);
                    showToast('error', 'Failed to start auto-sync watcher');
                });
        } else {
            console.log('[Auto-sync] Stopping watcher');
            // Stop watcher
            api.stopProjectWatcher().catch(err => {
                // Silently ignore if no watcher was running
                if (!err.toString().includes('No active watcher')) {
                    console.error('[Auto-sync] Failed to stop watcher:', err);
                }
            });
        }

        // Cleanup on unmount
        return () => {
            api.stopProjectWatcher().catch(() => {});
        };
    }, [currentProjectPath, state.autoSyncToLauncher, state.ltkManagerModPath]);

    // Listen for auto-sync events from Rust
    useEffect(() => {
        const unlistenComplete = listen('auto-sync-complete', (event) => {
            showToast('success', `Auto-synced to LTK Manager! Mod ID: ${event.payload}`);
        });

        const unlistenError = listen('auto-sync-error', (event) => {
            showToast('error', `Auto-sync failed: ${event.payload}`);
        });

        return () => {
            unlistenComplete.then((unlisten) => unlisten());
            unlistenError.then((unlisten) => unlisten());
        };
    }, [showToast]);

    // Manage preview file watcher for hot reload
    useEffect(() => {
        if (currentProjectPath) {
            console.log('[Preview Hot Reload] Starting watcher for:', currentProjectPath);
            api.startPreviewWatcher(currentProjectPath)
                .catch(err => {
                    console.error('[Preview Hot Reload] Failed to start watcher:', err);
                });
        } else {
            api.stopPreviewWatcher().catch(() => {});
        }

        // Cleanup on unmount
        return () => {
            api.stopPreviewWatcher().catch(() => {});
        };
    }, [currentProjectPath]);

    // Listen for file-changed events from Rust (hot reload)
    useEffect(() => {
        const unlistenFileChanged = listen<{ path: string; kind: string }>('file-changed', (event) => {
            const changedPath = event.payload.path;
            console.log('[Preview Hot Reload] File changed:', changedPath);

            // Invalidate cache for the changed file
            const wasInvalidated = invalidateCachedImage(changedPath);
            if (wasInvalidated) {
                console.log('[Preview Hot Reload] Cache invalidated for:', changedPath);
            }

            // Increment file version to trigger re-render in preview components
            useAppMetadataStore.getState().incrementFileVersion(changedPath);

            // Force re-render of preview if the changed file is currently selected
            const activeTab = getActiveTab(stateRef.current);
            if (activeTab && activeTab.selectedFile) {
                const selectedFilePath = `${activeTab.projectPath}/${activeTab.selectedFile}`;
                if (selectedFilePath === changedPath) {
                    console.log('[Preview Hot Reload] Currently previewed file changed, triggering reload');
                }
            }
        });

        return () => {
            unlistenFileChanged.then((unlisten) => unlisten());
        };
    }, []);

    const loadInitialData = async () => {
        // Sync log level setting to Rust backend
        api.setLogLevel(stateRef.current.verboseLogging).catch(() => {});

        try {
            const hashStatus = await api.getHashStatus();
            useAppMetadataStore.getState().setHashInfo(
                hashStatus.loaded_count > 0,
                hashStatus.loaded_count
            );

            if (hashStatus.loaded_count === 0) {
                pollHashStatus();
            }

            if (!state.leaguePath) {
                try {
                    const leagueResult = await api.detectLeague();
                    if (leagueResult.path) {
                        useConfigStore.getState().setLeaguePath(leagueResult.path);
                        console.log('[Flint] Auto-detected League path:', leagueResult.path);
                    }
                } catch {
                    console.log('[Flint] League auto-detection failed');
                }
            }

            // Check for updates after a short delay (don't block startup)
            setTimeout(checkForUpdates, 3000);
        } catch (error) {
            console.error('[Flint] Failed to load initial data:', error);
        }
    };

    const pollHashStatus = async () => {
        const maxAttempts = 30;
        let attempts = 0;

        const poll = async () => {
            try {
                const status = await api.getHashStatus();
                if (status.loaded_count > 0) {
                    useAppMetadataStore.getState().setHashInfo(true, status.loaded_count);
                    console.log(`[Flint] Hashes loaded: ${status.loaded_count.toLocaleString()}`);
                    return;
                }

                attempts++;
                if (attempts < maxAttempts) {
                    setTimeout(poll, 1000);
                }
            } catch (error) {
                console.error('[Flint] Error polling hash status:', error);
            }
        };

        setTimeout(poll, 1000);
    };

    const checkForUpdates = async () => {
        // Check if auto-updates are enabled
        if (!stateRef.current.autoUpdateEnabled) {
            console.log('[Flint] Auto-updates disabled, skipping update check');
            return;
        }

        try {
            console.log('[Flint] Checking for updates...');
            const result = await updater.checkForUpdates();

            if (result.available && result.newVersion) {
                // Skip if user already skipped this version
                if (stateRef.current.skippedUpdateVersion === result.newVersion) {
                    console.log(`[Flint] Update ${result.newVersion} was skipped by user`);
                    return;
                }

                console.log(`[Flint] Update available: ${result.currentVersion} → ${result.newVersion}`);

                // Convert to the format expected by UpdateModal
                const updateInfo = {
                    available: true,
                    current_version: result.currentVersion,
                    latest_version: result.newVersion,
                    release_notes: result.body || 'No release notes available',
                    published_at: result.date || new Date().toISOString(),
                    download_url: '', // Not needed with Tauri updater plugin
                };

                openModal('updateAvailable', updateInfo as unknown as Record<string, unknown>);
            } else {
                console.log('[Flint] Application is up to date');
            }
        } catch (error) {
            // Silently fail - don't bother user if update check fails
            console.log('[Flint] Update check failed:', error);
        }
    };

    const cleanStaleProjects = async () => {
        try {
            const recent = stateRef.current.recentProjects;
            if (recent.length === 0) return;

            // Validate all projects in parallel instead of sequentially
            const results = await Promise.allSettled(
                recent.map(project => api.listProjectFiles(project.path).then(() => project))
            );

            const validProjects = results
                .filter((r): r is PromiseFulfilledResult<typeof recent[number]> => r.status === 'fulfilled')
                .map(r => r.value);

            if (validProjects.length !== recent.length) {
                useConfigStore.getState().setRecentProjects(validProjects);
            }
        } catch (error) {
            console.error('[Flint] Failed to clean stale projects:', error);
        }
    };

    // Resizer handling
    const handleMouseDown = useCallback(() => {
        isResizingRef.current = true;
        document.body.style.cursor = 'col-resize';
        document.body.style.userSelect = 'none';
    }, []);

    useEffect(() => {
        const handleMouseMove = (e: MouseEvent) => {
            if (!isResizingRef.current) return;
            const newWidth = Math.min(400, Math.max(200, e.clientX));
            setLeftPanelWidth(newWidth);
        };

        const handleMouseUp = () => {
            if (isResizingRef.current) {
                isResizingRef.current = false;
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
            }
        };

        document.addEventListener('mousemove', handleMouseMove);
        document.addEventListener('mouseup', handleMouseUp);

        return () => {
            document.removeEventListener('mousemove', handleMouseMove);
            document.removeEventListener('mouseup', handleMouseUp);
        };
    }, []);

    const handleResizerDoubleClick = useCallback(() => {
        setLeftPanelWidth(prev => (prev === 48 ? 280 : 48));
    }, []);

    // Use currentView as the single source of truth for what's displayed
    const isWadExplorer = state.currentView === 'wad-explorer';
    const isExtractMode = state.currentView === 'extract';
    // Show a left panel for any view that isn't the welcome screen or WAD Explorer
    const hasProject = !isWadExplorer && state.currentView !== 'welcome';

    // Check if first-time setup is needed
    useEffect(() => {
        if (!state.creatorName && !state.activeModal) {
            openModal('firstTimeSetup');
        }
    }, [state.creatorName, state.activeModal, openModal]);

    return (
        <>
            <TitleBar />
            <div className="main-content" id="main-content">
                {/* Keep WadExplorer mounted when open — toggling display avoids the ~10s rescan on every switch */}
                {state.wadExplorer.isOpen && (
                    <div style={{ display: isWadExplorer ? 'contents' : 'none' }}>
                        <WadExplorer />
                    </div>
                )}
                {!isWadExplorer && (
                    <>
                        {hasProject && (
                            <>
                                {isExtractMode
                                    ? <WadBrowserPanel style={{ width: leftPanelWidth }} />
                                    : <LeftPanel style={{ width: leftPanelWidth }} />
                                }
                                <div
                                    ref={resizerRef}
                                    className="panel-resizer"
                                    id="panel-resizer"
                                    onMouseDown={handleMouseDown}
                                    onDoubleClick={handleResizerDoubleClick}
                                />
                            </>
                        )}
                        <CenterPanel />
                    </>
                )}
            </div>
            <StatusBar />

            {/* Modals */}
            <NewProjectModal />
            <SettingsModal />
            <ExportModal />
            <FirstTimeSetupModal />
            <UpdateModal />
            <RecolorModal />
            <FixerModal />

            {/* Toast notifications */}
            <ToastContainer />

            {/* Context Menu */}
            <ContextMenu />

            {/* Confirm Dialog */}
            <ConfirmDialog />
        </>
    );
};
