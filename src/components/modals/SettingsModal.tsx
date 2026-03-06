/**
 * Flint - Settings Modal Component
 * Left sidebar navigation + content panels
 */

import React, { useState, useEffect } from 'react';
import { useAppState } from '../../lib/stores';
import * as api from '../../lib/api';
import * as updater from '../../lib/updater';
import { open } from '@tauri-apps/plugin-dialog';
import { getIcon } from '../../lib/fileIcons';
import { getVersion } from '@tauri-apps/api/app';

type SettingsTab = 'paths' | 'general';

export const SettingsModal: React.FC = () => {
    const { state, dispatch, closeModal, showToast } = useAppState();

    const [activeTab, setActiveTab] = useState<SettingsTab>('paths');

    // Form state
    const [leaguePath, setLeaguePath] = useState(state.leaguePath || '');
    const [leaguePathPbe, setLeaguePathPbe] = useState(state.leaguePathPbe || '');
    const [defaultProjectPath, setDefaultProjectPath] = useState(state.defaultProjectPath || '');
    const [creatorName, setCreatorName] = useState(state.creatorName || '');
    const [autoUpdateEnabled, setAutoUpdateEnabled] = useState(state.autoUpdateEnabled);
    const [verboseLogging, setVerboseLogging] = useState(state.verboseLogging);
    const [isValidating, setIsValidating] = useState(false);

    // Update checker state
    const [currentVersion, setCurrentVersion] = useState<string>('');
    const [latestVersion, setLatestVersion] = useState<string | null>(null);
    const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
    const [updateAvailable, setUpdateAvailable] = useState(false);

    const isVisible = state.activeModal === 'settings';

    useEffect(() => {
        if (isVisible) {
            setLeaguePath(state.leaguePath || '');
            setLeaguePathPbe(state.leaguePathPbe || '');
            setDefaultProjectPath(state.defaultProjectPath || '');
            setCreatorName(state.creatorName || '');
            setAutoUpdateEnabled(state.autoUpdateEnabled);
            setVerboseLogging(state.verboseLogging);
            getVersion().then(setCurrentVersion).catch(() => setCurrentVersion('0.0.0'));
        }
    }, [isVisible, state.leaguePath, state.leaguePathPbe, state.defaultProjectPath, state.creatorName, state.autoUpdateEnabled, state.verboseLogging]);

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
            },
        });

        api.setLogLevel(verboseLogging).catch(() => { });
        showToast('success', 'Settings saved');
        closeModal();
    };

    if (!isVisible) return null;

    const tabs: { id: SettingsTab; label: string; icon: Parameters<typeof getIcon>[0] }[] = [
        { id: 'paths', label: 'Paths', icon: 'folder' },
        { id: 'general', label: 'General', icon: 'settings' },
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

                                <div className="settings-item">
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
                                </div>
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
