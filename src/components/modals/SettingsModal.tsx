/**
 * Flint - Settings Modal Component
 * Left sidebar navigation + content panels
 */

import React, { useState, useEffect } from 'react';
import { useAppState, useConfigStore } from '../../lib/stores';
import * as api from '../../lib/api';
import * as updater from '../../lib/updater';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { getIcon } from '../../lib/fileIcons';
import { getVersion } from '@tauri-apps/api/app';

type SettingsTab = 'paths' | 'general' | 'dev';

export const SettingsModal: React.FC = () => {
    const { state, dispatch, closeModal, showToast } = useAppState();
    const configStore = useConfigStore();

    const [activeTab, setActiveTab] = useState<SettingsTab>('general');

    // Form state
    const [leaguePath, setLeaguePath] = useState(state.leaguePath || '');
    const [leaguePathPbe, setLeaguePathPbe] = useState(state.leaguePathPbe || '');
    const [defaultProjectPath, setDefaultProjectPath] = useState(state.defaultProjectPath || '');
    const [creatorName, setCreatorName] = useState(state.creatorName || '');
    const [autoUpdateEnabled, setAutoUpdateEnabled] = useState(state.autoUpdateEnabled);
    const [verboseLogging, setVerboseLogging] = useState(state.verboseLogging);
    const [ltkManagerModPath, setLtkManagerModPath] = useState(state.ltkManagerModPath || '');
    const [autoSyncToLauncher, setAutoSyncToLauncher] = useState(state.autoSyncToLauncher);
    const [binConverterEngine, setBinConverterEngine] = useState<'ltk' | 'jade'>(configStore.binConverterEngine);
    const [jadePath, setJadePath] = useState(configStore.jadePath || '');
    const [quartzPath, setQuartzPath] = useState(configStore.quartzPath || '');
    const [isValidating, setIsValidating] = useState(false);

    // Update checker state
    const [currentVersion, setCurrentVersion] = useState<string>('');
    const [latestVersion, setLatestVersion] = useState<string | null>(null);
    const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
    const [updateAvailable, setUpdateAvailable] = useState(false);

    // Theme state
    const [selectedTheme, setSelectedTheme] = useState(configStore.selectedTheme || '');
    const [availableThemes, setAvailableThemes] = useState<api.ThemeInfo[]>([]);

    // Hash database state
    const [isRebuildingHashes, setIsRebuildingHashes] = useState(false);

    // Schema aggregation state (Dev tab)
    const [isAggregating, setIsAggregating] = useState(false);
    const [schemaProgress, setSchemaProgress] = useState<{
        phase: string;
        current: number;
        total: number;
        bins_parsed: number;
        bins_failed: number;
        classes_found: number;
    } | null>(null);
    const [schemaResult, setSchemaResult] = useState<api.SchemaStats | null>(null);

    // Champion schema aggregation state (Dev tab)
    const [isAggregatingChampion, setIsAggregatingChampion] = useState(false);
    const [championSchemaProgress, setChampionSchemaProgress] = useState<{
        phase: string;
        current: number;
        total: number;
        bins_parsed: number;
        bins_failed: number;
        classes_found: number;
    } | null>(null);
    const [championSchemaResult, setChampionSchemaResult] = useState<api.ChampionSchemaStats | null>(null);

    const isVisible = state.activeModal === 'settings';

    useEffect(() => {
        if (isVisible) {
            setLeaguePath(state.leaguePath || '');
            setLeaguePathPbe(state.leaguePathPbe || '');
            setDefaultProjectPath(state.defaultProjectPath || '');
            setCreatorName(state.creatorName || '');
            setAutoUpdateEnabled(state.autoUpdateEnabled);
            setVerboseLogging(state.verboseLogging);
            setLtkManagerModPath(state.ltkManagerModPath || '');
            setAutoSyncToLauncher(state.autoSyncToLauncher);
            setBinConverterEngine(configStore.binConverterEngine);
            setJadePath(configStore.jadePath || '');
            setQuartzPath(configStore.quartzPath || '');
            setSelectedTheme(configStore.selectedTheme || '');
            getVersion().then(setCurrentVersion).catch(() => setCurrentVersion('0.0.0'));
            api.listThemes().then(setAvailableThemes).catch(() => {});
        }
    }, [isVisible, state.leaguePath, state.leaguePathPbe, state.defaultProjectPath, state.creatorName, state.autoUpdateEnabled, state.verboseLogging, state.ltkManagerModPath, state.autoSyncToLauncher, configStore.binConverterEngine, configStore.jadePath, configStore.quartzPath, configStore.selectedTheme]);

    // Listen for schema aggregation progress events
    useEffect(() => {
        const unlisten = listen<{
            phase: string;
            current: number;
            total: number;
            bins_parsed: number;
            bins_failed: number;
            classes_found: number;
        }>('schema-progress', (event) => {
            setSchemaProgress(event.payload);
        });
        return () => { unlisten.then((fn) => fn()); };
    }, []);

    // Listen for champion schema aggregation progress events
    useEffect(() => {
        const unlisten = listen<{
            phase: string;
            current: number;
            total: number;
            bins_parsed: number;
            bins_failed: number;
            classes_found: number;
        }>('champion-schema-progress', (event) => {
            setChampionSchemaProgress(event.payload);
        });
        return () => { unlisten.then((fn) => fn()); };
    }, []);

    const handleBrowse = async (setter: (v: string) => void, title: string) => {
        const selected = await open({ title, directory: true });
        if (selected) setter(selected as string);
    };

    const handleDetectLeague = async () => {
        setIsValidating(true);
        try {
            const result = await api.detectLeague();
            if (result.path) {
                setLeaguePath(result.path);
                showToast('success', 'League installation detected!');
            }
        } catch {
            showToast('error', 'Could not auto-detect League installation');
        } finally {
            setIsValidating(false);
        }
    };

    const handleDetectPbe = async () => {
        // Try to find PBE by checking common paths relative to the League path
        const basePath = leaguePath || state.leaguePath;
        if (basePath) {
            // League path is like "C:\Riot Games\League of Legends"
            // PBE is usually "C:\Riot Games\League of Legends (PBE)"
            const parent = basePath.replace(/[\\/][^\\/]+$/, '');
            const pbeCandidates = [
                `${parent}\\League of Legends (PBE)`,
                `${parent}\\League of Legends(PBE)`,
                basePath + ' (PBE)',
            ];

            for (const candidate of pbeCandidates) {
                try {
                    const result = await api.validateLeague(candidate);
                    if (result.valid) {
                        setLeaguePathPbe(candidate);
                        showToast('success', 'PBE installation detected!');
                        return;
                    }
                } catch {
                    // continue to next candidate
                }
            }
        }
        showToast('error', 'Could not auto-detect PBE installation');
    };

    const handleDetectLtkManager = async () => {
        setIsValidating(true);
        try {
            const path = await api.getLtkManagerModPath();
            if (path) {
                setLtkManagerModPath(path);
                showToast('success', 'LTK Manager installation detected!');
            } else {
                showToast('error', 'LTK Manager not found. Please install LTK Manager first.');
            }
        } catch {
            showToast('error', 'Failed to detect LTK Manager installation');
        } finally {
            setIsValidating(false);
        }
    };

    const handleDetectJade = async () => {
        setIsValidating(true);
        try {
            const path = await api.detectJadeInstallation();
            if (path) {
                setJadePath(path);
                showToast('success', 'Jade installation detected!');
            } else {
                showToast('error', 'Jade not found. Please install Jade League Bin Editor first.');
            }
        } catch {
            showToast('error', 'Failed to detect Jade installation');
        } finally {
            setIsValidating(false);
        }
    };

    const handleDetectQuartz = async () => {
        setIsValidating(true);
        try {
            const path = await api.detectQuartzInstallation();
            if (path) {
                setQuartzPath(path);
                showToast('success', 'Quartz installation detected!');
            } else {
                showToast('error', 'Quartz not found. Please install Quartz first.');
            }
        } catch {
            showToast('error', 'Failed to detect Quartz installation');
        } finally {
            setIsValidating(false);
        }
    };

    const handleCheckForUpdates = async () => {
        setIsCheckingUpdate(true);
        setLatestVersion(null);
        setUpdateAvailable(false);
        try {
            const result = await updater.checkForUpdates();
            if (result.available && result.newVersion) {
                setLatestVersion(result.newVersion);
                setUpdateAvailable(true);
                showToast('success', `Update available: v${result.newVersion}`);
            } else {
                setLatestVersion(result.currentVersion);
                showToast('info', 'You are running the latest version');
            }
        } catch {
            showToast('error', 'Failed to check for updates');
        } finally {
            setIsCheckingUpdate(false);
        }
    };

    const handleUpdateNow = () => {
        if (latestVersion) {
            dispatch({
                type: 'OPEN_MODAL',
                payload: {
                    modal: 'updateAvailable',
                    options: {
                        available: true,
                        current_version: currentVersion,
                        latest_version: latestVersion,
                        release_notes: 'Check GitHub releases for details',
                        published_at: new Date().toISOString(),
                    } as Record<string, unknown>,
                },
            });
        }
    };

    const handleForceRebuildHashes = async () => {
        setIsRebuildingHashes(true);
        try {
            await api.forceRebuildHashes();

            // Clear WAD Explorer cache to force reload with new hashes
            if (state.wadExplorer.isOpen) {
                // Reset all loaded WADs to trigger fresh chunk loading
                state.wadExplorer.wads.forEach(wad => {
                    if (wad.status === 'loaded') {
                        dispatch({
                            type: 'SET_WAD_EXPLORER_WAD_STATUS',
                            payload: { wadPath: wad.path, status: 'idle', chunks: [], error: null }
                        });
                    }
                });
            }

            showToast('success', 'Hash database rebuilt - collapse/expand WADs to reload');
        } catch (error) {
            console.error('Failed to rebuild hash database:', error);
            showToast('error', 'Failed to rebuild hash database');
        } finally {
            setIsRebuildingHashes(false);
        }
    };

    const handleAggregateBinSchema = async () => {
        if (!state.leaguePath) {
            showToast('error', 'League path not configured. Set it in the Paths tab first.');
            return;
        }
        setIsAggregating(true);
        setSchemaProgress(null);
        setSchemaResult(null);
        try {
            const stats = await api.aggregateBinSchema(state.leaguePath);
            setSchemaResult(stats);
            showToast('success', `Schema aggregated: ${stats.classes_found.toLocaleString()} classes, ${stats.total_fields.toLocaleString()} fields`);
        } catch (error) {
            console.error('Schema aggregation failed:', error);
            showToast('error', 'Schema aggregation failed. Check the log for details.');
        } finally {
            setIsAggregating(false);
        }
    };

    const handleAggregateChampionSchema = async () => {
        if (!state.leaguePath) {
            showToast('error', 'League path not configured. Set it in the Paths tab first.');
            return;
        }
        setIsAggregatingChampion(true);
        setChampionSchemaProgress(null);
        setChampionSchemaResult(null);
        try {
            const stats = await api.aggregateChampionBinSchema(state.leaguePath);
            setChampionSchemaResult(stats);
            showToast('success', `Champion schema built: ${stats.classes_found.toLocaleString()} classes, ${stats.total_fields.toLocaleString()} fields`);
        } catch (error) {
            console.error('Champion schema aggregation failed:', error);
            showToast('error', 'Champion schema aggregation failed. Check the log for details.');
        } finally {
            setIsAggregatingChampion(false);
        }
    };

    const handleSave = async () => {
        if (leaguePath && leaguePath !== state.leaguePath) {
            setIsValidating(true);
            try {
                const result = await api.validateLeague(leaguePath);
                if (!result.valid) {
                    showToast('error', 'Invalid League of Legends path');
                    setIsValidating(false);
                    return;
                }
            } catch {
                showToast('error', 'Failed to validate League path');
                setIsValidating(false);
                return;
            }
            setIsValidating(false);
        }

        dispatch({
            type: 'SET_STATE',
            payload: {
                leaguePath: leaguePath || null,
                leaguePathPbe: leaguePathPbe || null,
                defaultProjectPath: defaultProjectPath || null,
                creatorName: creatorName || null,
                autoUpdateEnabled,
                verboseLogging,
                ltkManagerModPath: ltkManagerModPath || null,
                autoSyncToLauncher,
            },
        });

        // Save to configStore
        configStore.setBinConverterEngine(binConverterEngine);
        configStore.setJadePath(jadePath || null);
        configStore.setQuartzPath(quartzPath || null);
        configStore.setSelectedTheme(selectedTheme || null);

        api.setLogLevel(verboseLogging).catch(() => { });
        showToast('success', 'Settings saved');
        closeModal();
    };

    if (!isVisible) return null;

    const tabs: { id: SettingsTab; label: string; icon: Parameters<typeof getIcon>[0] }[] = [
        { id: 'general', label: 'General', icon: 'settings' },
        { id: 'paths', label: 'Paths', icon: 'folder' },
        { id: 'dev', label: 'Dev', icon: 'code' },
    ];

    return (
        <div className={`modal-overlay ${isVisible ? 'modal-overlay--visible' : ''}`}>
            <div className="modal modal--settings">
                <div className="modal__header">
                    <h2 className="modal__title">Settings</h2>
                    <button className="modal__close" onClick={closeModal}>×</button>
                </div>

                <div className="settings-layout">
                    {/* Left Sidebar */}
                    <div className="settings-sidebar">
                        {tabs.map(tab => (
                            <button
                                key={tab.id}
                                className={`settings-sidebar__item ${activeTab === tab.id ? 'settings-sidebar__item--active' : ''}`}
                                onClick={() => setActiveTab(tab.id)}
                            >
                                <span dangerouslySetInnerHTML={{ __html: getIcon(tab.icon) }} />
                                <span>{tab.label}</span>
                            </button>
                        ))}
                    </div>

                    {/* Content Panel */}
                    <div className="settings-content">
                        {/* Paths Tab */}
                        {activeTab === 'paths' && (
                            <div className="settings-panel">
                                <div className="settings-item">
                                    <label className="settings-item__label">Default Project Path</label>
                                    <div className="form-input--with-button">
                                        <input
                                            type="text"
                                            className="form-input"
                                            placeholder="Where new projects are created"
                                            value={defaultProjectPath}
                                            onChange={(e) => setDefaultProjectPath(e.target.value)}
                                        />
                                        <button className="btn btn--secondary" onClick={() => handleBrowse(setDefaultProjectPath, 'Select Default Project Folder')}>
                                            Browse
                                        </button>
                                    </div>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-item__label">League of Legends Path</label>
                                    <div className="form-input--with-button">
                                        <input
                                            type="text"
                                            className="form-input"
                                            placeholder="C:\Riot Games\League of Legends"
                                            value={leaguePath}
                                            onChange={(e) => setLeaguePath(e.target.value)}
                                        />
                                        <button className="btn btn--secondary" onClick={() => handleBrowse(setLeaguePath, 'Select League of Legends Folder')}>
                                            Browse
                                        </button>
                                    </div>
                                    <button
                                        className="btn btn--ghost btn--sm"
                                        style={{ marginTop: '6px' }}
                                        onClick={handleDetectLeague}
                                        disabled={isValidating}
                                    >
                                        <span dangerouslySetInnerHTML={{ __html: getIcon('search') }} />
                                        <span>Auto-detect</span>
                                    </button>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-item__label">
                                        League of Legends PBE Path
                                        <span className="settings-item__badge">PBE</span>
                                    </label>
                                    <div className="form-input--with-button">
                                        <input
                                            type="text"
                                            className="form-input"
                                            placeholder="C:\Riot Games\League of Legends (PBE)"
                                            value={leaguePathPbe}
                                            onChange={(e) => setLeaguePathPbe(e.target.value)}
                                        />
                                        <button className="btn btn--secondary" onClick={() => handleBrowse(setLeaguePathPbe, 'Select PBE Folder')}>
                                            Browse
                                        </button>
                                    </div>
                                    <button
                                        className="btn btn--ghost btn--sm"
                                        style={{ marginTop: '6px' }}
                                        onClick={handleDetectPbe}
                                        disabled={isValidating}
                                    >
                                        <span dangerouslySetInnerHTML={{ __html: getIcon('search') }} />
                                        <span>Auto-detect PBE</span>
                                    </button>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-item__label">
                                        LTK Manager Mod Path
                                        <span className="settings-item__badge">Launcher</span>
                                    </label>
                                    <div className="form-input--with-button">
                                        <input
                                            type="text"
                                            className="form-input"
                                            placeholder="Path to LTK Manager mod storage"
                                            value={ltkManagerModPath}
                                            onChange={(e) => setLtkManagerModPath(e.target.value)}
                                        />
                                        <button className="btn btn--secondary" onClick={() => handleBrowse(setLtkManagerModPath, 'Select LTK Manager Mod Storage Folder')}>
                                            Browse
                                        </button>
                                    </div>
                                    <button
                                        className="btn btn--ghost btn--sm"
                                        style={{ marginTop: '6px' }}
                                        onClick={handleDetectLtkManager}
                                        disabled={isValidating}
                                    >
                                        <span dangerouslySetInnerHTML={{ __html: getIcon('search') }} />
                                        <span>Auto-detect LTK Manager</span>
                                    </button>
                                    <div className="settings-item__hint">
                                        Configure where LTK Manager stores mods for auto-sync functionality
                                    </div>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-item__label">
                                        Jade Path
                                        <span className="settings-item__badge">External App</span>
                                    </label>
                                    <div className="form-input--with-button">
                                        <input
                                            type="text"
                                            className="form-input"
                                            placeholder="Path to Jade.exe"
                                            value={jadePath}
                                            onChange={(e) => setJadePath(e.target.value)}
                                        />
                                        <button className="btn btn--secondary" onClick={() => handleBrowse(setJadePath, 'Select Jade Executable')}>
                                            Browse
                                        </button>
                                    </div>
                                    <button
                                        className="btn btn--ghost btn--sm"
                                        style={{ marginTop: '6px' }}
                                        onClick={handleDetectJade}
                                        disabled={isValidating}
                                    >
                                        <span dangerouslySetInnerHTML={{ __html: getIcon('search') }} />
                                        <span>Auto-detect Jade</span>
                                    </button>
                                    <div className="settings-item__hint">
                                        Jade League Bin Editor - Alternative BIN viewer with custom converter
                                    </div>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-item__label">
                                        Quartz Path
                                        <span className="settings-item__badge">External App</span>
                                    </label>
                                    <div className="form-input--with-button">
                                        <input
                                            type="text"
                                            className="form-input"
                                            placeholder="Path to Quartz.exe"
                                            value={quartzPath}
                                            onChange={(e) => setQuartzPath(e.target.value)}
                                        />
                                        <button className="btn btn--secondary" onClick={() => handleBrowse(setQuartzPath, 'Select Quartz Executable')}>
                                            Browse
                                        </button>
                                    </div>
                                    <button
                                        className="btn btn--ghost btn--sm"
                                        style={{ marginTop: '6px' }}
                                        onClick={handleDetectQuartz}
                                        disabled={isValidating}
                                    >
                                        <span dangerouslySetInnerHTML={{ __html: getIcon('search') }} />
                                        <span>Auto-detect Quartz</span>
                                    </button>
                                    <div className="settings-item__hint">
                                        Quartz VFX Editor - Tool for recoloring and porting VFX
                                    </div>
                                </div>
                            </div>
                        )}

                        {/* General Tab */}
                        {activeTab === 'general' && (
                            <div className="settings-panel">
                                <div className="settings-item">
                                    <label className="settings-item__label">Creator Name</label>
                                    <input
                                        type="text"
                                        className="form-input"
                                        placeholder="Your name (for mod credits)"
                                        value={creatorName}
                                        onChange={(e) => setCreatorName(e.target.value)}
                                    />
                                </div>

                                <div className="settings-item">
                                    <label className="settings-item__label">Theme</label>
                                    <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                                        <select
                                            className="form-input"
                                            style={{ flex: 1 }}
                                            value={selectedTheme}
                                            onChange={(e) => setSelectedTheme(e.target.value || '')}
                                        >
                                            <option value="">Default (Red)</option>
                                            {availableThemes.map(t => (
                                                <option key={t.id} value={t.id}>{t.name}</option>
                                            ))}
                                        </select>
                                        <button
                                            className="btn btn--secondary btn--sm"
                                            onClick={async () => {
                                                try {
                                                    const path = await api.createDefaultTheme();
                                                    await api.openInExplorer(path.replace(/[^/\\]*$/, ''));
                                                    showToast('success', 'Theme template created — edit custom.json and restart');
                                                    // Refresh theme list
                                                    api.listThemes().then(setAvailableThemes).catch(() => {});
                                                } catch (err) {
                                                    showToast('error', 'Failed to create theme template');
                                                }
                                            }}
                                        >
                                            Create Custom
                                        </button>
                                    </div>
                                    <div className="settings-item__hint">
                                        Drop .json theme files in the themes folder to add more options
                                    </div>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-item__label">
                                        BIN Conversion Engine
                                        <span className="settings-item__badge">Advanced</span>
                                    </label>
                                    <select
                                        className="form-input"
                                        value={binConverterEngine}
                                        onChange={(e) => setBinConverterEngine(e.target.value as 'ltk' | 'jade')}
                                    >
                                        <option value="ltk">LTK (Default)</option>
                                        <option value="jade">Jade Custom</option>
                                    </select>
                                    <div className="settings-item__hint">
                                        Jade Custom converter may handle certain BIN files better than LTK
                                    </div>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-toggle">
                                        <input
                                            type="checkbox"
                                            checked={verboseLogging}
                                            onChange={(e) => setVerboseLogging(e.target.checked)}
                                        />
                                        <div className="settings-toggle__content">
                                            <div className="settings-toggle__label">Verbose Logging</div>
                                            <div className="settings-toggle__description">
                                                Show detailed debug output in the log panel
                                            </div>
                                        </div>
                                    </label>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-label">Test Logging</label>
                                    <button
                                        type="button"
                                        className="form-button form-button--secondary"
                                        onClick={async () => {
                                            try {
                                                await api.testLogging();
                                                showToast('info', 'Test logs emitted - check the output panel');
                                            } catch (err) {
                                                console.error('Test logging failed:', err);
                                                showToast('error', 'Failed to emit test logs');
                                            }
                                        }}
                                    >
                                        Emit Test Logs
                                    </button>
                                    <p className="settings-description">
                                        Emit test logs at all levels to verify logging is working
                                    </p>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-toggle">
                                        <input
                                            type="checkbox"
                                            checked={autoUpdateEnabled}
                                            onChange={(e) => setAutoUpdateEnabled(e.target.checked)}
                                        />
                                        <div className="settings-toggle__content">
                                            <div className="settings-toggle__label">Automatic Updates</div>
                                            <div className="settings-toggle__description">
                                                Check for updates on startup
                                            </div>
                                        </div>
                                    </label>
                                </div>

                                <div className="settings-item">
                                    <label className="settings-toggle">
                                        <input
                                            type="checkbox"
                                            checked={autoSyncToLauncher}
                                            onChange={(e) => setAutoSyncToLauncher(e.target.checked)}
                                            disabled={!ltkManagerModPath}
                                        />
                                        <div className="settings-toggle__content">
                                            <div className="settings-toggle__label">
                                                Auto-Sync to LTK Manager
                                                {ltkManagerModPath && <span className="settings-item__badge" style={{ marginLeft: '8px' }}>Launcher</span>}
                                            </div>
                                            <div className="settings-toggle__description">
                                                {ltkManagerModPath
                                                    ? 'Automatically sync project changes to LTK Manager when files are modified'
                                                    : 'Configure LTK Manager path in Paths tab to enable auto-sync'}
                                            </div>
                                        </div>
                                    </label>
                                </div>

                                {/* Hash Database Management */}
                                <div className="settings-item">
                                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                                        <div className="settings-item__info">
                                            <span dangerouslySetInnerHTML={{ __html: state.hashesLoaded ? getIcon('success') : getIcon('warning') }} />
                                            <div>
                                                <div className="settings-item__label" style={{ marginBottom: 0 }}>Hash Database</div>
                                                <div className="settings-item__value">
                                                    {state.hashesLoaded
                                                        ? `${state.hashCount.toLocaleString()} hashes loaded`
                                                        : 'Hashes not loaded'}
                                                </div>
                                            </div>
                                        </div>
                                        <button
                                            className="btn btn--secondary btn--sm"
                                            onClick={handleForceRebuildHashes}
                                            disabled={isRebuildingHashes}
                                        >
                                            <span dangerouslySetInnerHTML={{ __html: getIcon('refresh') }} />
                                            <span>{isRebuildingHashes ? 'Rebuilding...' : 'Force Rebuild Hashes'}</span>
                                        </button>
                                    </div>
                                    <div className="settings-item__hint">
                                        Rebuild hash database to apply latest fixes (BIN file resolution, etc.)
                                    </div>
                                </div>

                                {/* Version & Update Card */}
                                <div className="version-card">
                                    <div className="version-card__content">
                                        <div className="version-card__current">
                                            <div className="version-card__label">Current</div>
                                            <div className="version-card__version">v{currentVersion}</div>
                                        </div>

                                        {latestVersion && updateAvailable && (
                                            <>
                                                <span
                                                    className="version-card__arrow"
                                                    dangerouslySetInnerHTML={{ __html: getIcon('chevronRight') }}
                                                />
                                                <div className="version-card__latest">
                                                    <div className="version-card__label version-card__label--accent">
                                                        Latest
                                                    </div>
                                                    <div className="version-card__version version-card__version--accent">
                                                        v{latestVersion}
                                                    </div>
                                                </div>
                                            </>
                                        )}
                                    </div>

                                    <div className="version-card__actions">
                                        <button
                                            className="btn btn--secondary"
                                            onClick={handleCheckForUpdates}
                                            disabled={isCheckingUpdate}
                                            style={{ flex: 1 }}
                                        >
                                            <span dangerouslySetInnerHTML={{ __html: getIcon('refresh') }} />
                                            <span>{isCheckingUpdate ? 'Checking...' : 'Check for Updates'}</span>
                                        </button>

                                        {updateAvailable && latestVersion && (
                                            <button
                                                className="btn btn--primary"
                                                onClick={handleUpdateNow}
                                                style={{ flex: 1 }}
                                            >
                                                <span dangerouslySetInnerHTML={{ __html: getIcon('download') }} />
                                                <span>Update Now</span>
                                            </button>
                                        )}
                                    </div>
                                </div>
                            </div>
                        )}

                        {/* Dev Tab */}
                        {activeTab === 'dev' && (
                            <div className="settings-panel">
                                <div className="settings-item">
                                    <label className="settings-item__label">BIN Schema Aggregator</label>
                                    <div className="settings-item__hint" style={{ marginBottom: '8px' }}>
                                        Scans all WAD archives in your League installation and extracts the complete
                                        BIN class/field schema. Parses every BIN, unions all fields per class, and
                                        outputs a ritobin-style schema reference with value ranges.
                                    </div>
                                    <button
                                        className="btn btn--secondary"
                                        onClick={handleAggregateBinSchema}
                                        disabled={isAggregating || !state.leaguePath}
                                    >
                                        <span dangerouslySetInnerHTML={{ __html: getIcon('download') }} />
                                        <span>{isAggregating ? 'Aggregating...' : 'Get BIN Entries'}</span>
                                    </button>
                                    {!state.leaguePath && (
                                        <div className="settings-item__hint" style={{ color: 'var(--color-warning)', marginTop: '4px' }}>
                                            Configure League path in the Paths tab first
                                        </div>
                                    )}
                                </div>

                                {isAggregating && schemaProgress && (
                                    <div className="settings-item">
                                        <div className="settings-item__label">
                                            {schemaProgress.phase === 'complete'
                                                ? 'Complete'
                                                : `Scanning WAD ${schemaProgress.current} / ${schemaProgress.total}`}
                                        </div>
                                        <div style={{
                                            width: '100%',
                                            height: '4px',
                                            background: 'var(--color-surface-2)',
                                            borderRadius: '2px',
                                            overflow: 'hidden',
                                            marginTop: '4px',
                                        }}>
                                            <div style={{
                                                width: `${(schemaProgress.current / Math.max(schemaProgress.total, 1)) * 100}%`,
                                                height: '100%',
                                                background: 'var(--color-accent)',
                                                transition: 'width 0.2s ease',
                                            }} />
                                        </div>
                                        <div className="settings-item__hint" style={{ marginTop: '4px' }}>
                                            {schemaProgress.bins_parsed.toLocaleString()} BINs parsed
                                            {schemaProgress.bins_failed > 0 && ` (${schemaProgress.bins_failed} failed)`}
                                            {' | '}{schemaProgress.classes_found.toLocaleString()} classes found
                                        </div>
                                    </div>
                                )}

                                {schemaResult && !isAggregating && (
                                    <div className="settings-item">
                                        <div className="settings-item__label">Result</div>
                                        <div className="settings-item__hint">
                                            Found {schemaResult.classes_found.toLocaleString()} classes with{' '}
                                            {schemaResult.total_fields.toLocaleString()} fields across{' '}
                                            {schemaResult.bins_parsed.toLocaleString()} BIN files
                                            {schemaResult.bins_failed > 0 && ` (${schemaResult.bins_failed} failed to parse)`}
                                            {' from '}{schemaResult.wads_scanned.toLocaleString()} WADs
                                        </div>
                                        <div className="settings-item__hint" style={{ marginTop: '4px' }}>
                                            Output: {schemaResult.output_path}
                                        </div>
                                        <button
                                            className="btn btn--ghost btn--sm"
                                            style={{ marginTop: '6px' }}
                                            onClick={() => {
                                                const dir = schemaResult.output_path.replace(/[\\/][^\\/]+$/, '');
                                                api.openInExplorer(dir).catch(() => {});
                                            }}
                                        >
                                            <span dangerouslySetInnerHTML={{ __html: getIcon('folder') }} />
                                            <span>Open in Explorer</span>
                                        </button>
                                    </div>
                                )}

                                <div className="settings-item" style={{ marginTop: '16px', paddingTop: '16px', borderTop: '1px solid var(--color-border)' }}>
                                    <label className="settings-item__label">Champion BIN Schema Creator</label>
                                    <div className="settings-item__hint" style={{ marginBottom: '8px' }}>
                                        Walks only the Champions WAD folder, picks skin BINs and the data BINs they
                                        link to — excludes champion-root, root.bin, animation, and corrupt BINs.
                                        Merges every property of every class globally and emits ONE synthetic ritobin
                                        file in real block syntax (with brackets). Copy any block straight into a
                                        .ritobin file.
                                    </div>
                                    <button
                                        className="btn btn--secondary"
                                        onClick={handleAggregateChampionSchema}
                                        disabled={isAggregatingChampion || !state.leaguePath}
                                    >
                                        <span dangerouslySetInnerHTML={{ __html: getIcon('download') }} />
                                        <span>{isAggregatingChampion ? 'Building...' : 'Build Champion Schema'}</span>
                                    </button>
                                    {!state.leaguePath && (
                                        <div className="settings-item__hint" style={{ color: 'var(--color-warning)', marginTop: '4px' }}>
                                            Configure League path in the Paths tab first
                                        </div>
                                    )}
                                </div>

                                {isAggregatingChampion && championSchemaProgress && (
                                    <div className="settings-item">
                                        <div className="settings-item__label">
                                            {championSchemaProgress.phase === 'complete'
                                                ? 'Complete'
                                                : `Scanning WAD ${championSchemaProgress.current} / ${championSchemaProgress.total}`}
                                        </div>
                                        <div style={{
                                            width: '100%',
                                            height: '4px',
                                            background: 'var(--color-surface-2)',
                                            borderRadius: '2px',
                                            overflow: 'hidden',
                                            marginTop: '4px',
                                        }}>
                                            <div style={{
                                                width: `${(championSchemaProgress.current / Math.max(championSchemaProgress.total, 1)) * 100}%`,
                                                height: '100%',
                                                background: 'var(--color-accent)',
                                                transition: 'width 0.2s ease',
                                            }} />
                                        </div>
                                        <div className="settings-item__hint" style={{ marginTop: '4px' }}>
                                            {championSchemaProgress.bins_parsed.toLocaleString()} BINs parsed
                                            {championSchemaProgress.bins_failed > 0 && ` (${championSchemaProgress.bins_failed} failed)`}
                                            {' | '}{championSchemaProgress.classes_found.toLocaleString()} classes found
                                        </div>
                                    </div>
                                )}

                                {championSchemaResult && !isAggregatingChampion && (
                                    <div className="settings-item">
                                        <div className="settings-item__label">Result</div>
                                        <div className="settings-item__hint">
                                            Built {championSchemaResult.classes_found.toLocaleString()} classes with{' '}
                                            {championSchemaResult.total_fields.toLocaleString()} fields across{' '}
                                            {championSchemaResult.bins_parsed.toLocaleString()} LinkedData BINs
                                            {championSchemaResult.bins_failed > 0 && ` (${championSchemaResult.bins_failed} failed to parse)`}
                                            {' from '}{championSchemaResult.wads_scanned.toLocaleString()} WADs
                                        </div>
                                        <div className="settings-item__hint" style={{ marginTop: '4px' }}>
                                            Output: {championSchemaResult.output_path}
                                        </div>
                                        <button
                                            className="btn btn--ghost btn--sm"
                                            style={{ marginTop: '6px' }}
                                            onClick={() => {
                                                const dir = championSchemaResult.output_path.replace(/[\\/][^\\/]+$/, '');
                                                api.openInExplorer(dir).catch(() => {});
                                            }}
                                        >
                                            <span dangerouslySetInnerHTML={{ __html: getIcon('folder') }} />
                                            <span>Open in Explorer</span>
                                        </button>
                                    </div>
                                )}
                            </div>
                        )}
                    </div>
                </div>

                <div className="modal__footer">
                    <button className="btn btn--secondary" onClick={closeModal}>
                        Cancel
                    </button>
                    <button className="btn btn--primary" onClick={handleSave} disabled={isValidating}>
                        <span dangerouslySetInnerHTML={{ __html: getIcon('success') }} />
                        <span>Save Settings</span>
                    </button>
                </div>
            </div>
        </div>
    );
};
