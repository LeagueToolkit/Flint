/**
 * Flint - Project List Modal
 *
 * Modernized "open existing project" view: searchable, sortable, card-based.
 * Each row shows a champion monogram tile, project + path, relative time, and
 * a trash control. Footer hosts "Open from disk" and the green "Import Mod"
 * primary action.
 */

import React, { useState, useCallback, useEffect, useMemo } from 'react';
import { useAppState, useConfigStore } from '../../lib/stores';
import { formatRelativeTime } from '../../lib/utils';
import { open } from '@tauri-apps/plugin-dialog';
import { appDataDir } from '@tauri-apps/api/path';
import { Button, Icon, Input, Modal, ModalBody, ModalFooter, ModalHeader, Picker } from '../ui';
import * as api from '../../lib/api';
import { listen } from '@tauri-apps/api/event';
import type { SavedProject } from '../../lib/types';

type SortMode = 'recent' | 'name' | 'champion';

const SORT_OPTIONS = [
    { value: 'recent',   label: 'Recently opened' },
    { value: 'name',     label: 'Project name (A–Z)' },
    { value: 'champion', label: 'Champion (A–Z)' },
] as const;

/** Two-letter monogram for the champion tile (e.g. "Aatrox" → "AA"). */
function monogram(champion: string): string {
    const c = (champion || '?').trim();
    if (!c) return '?';
    if (c.length <= 2) return c.toUpperCase();
    // Use first two letters
    return (c[0] + c[1]).toUpperCase();
}

/** Stable hue 0–360 derived from the champion name — gives each tile its own
 *  color without persistence. */
function hueFor(champion: string): number {
    let h = 0;
    for (let i = 0; i < champion.length; i++) h = (h * 31 + champion.charCodeAt(i)) >>> 0;
    return h % 360;
}

export const ProjectListModal: React.FC = () => {
    const { state, dispatch, closeModal, setWorking, setReady, setError, openConfirmDialog } = useAppState();
    const configStore = useConfigStore();
    const [removingId, setRemovingId] = useState<string | null>(null);
    const [search, setSearch] = useState('');
    const [sortMode, setSortMode] = useState<SortMode>('recent');

    const isVisible = state.activeModal === 'projectList';
    const savedProjects = state.savedProjects || [];

    // Reset search when modal opens
    useEffect(() => {
        if (isVisible) {
            setSearch('');
            setSortMode('recent');
        }
    }, [isVisible]);

    // Listen for import progress events
    useEffect(() => {
        const unlistenFantome = listen<{ status: string; message: string }>('fantome-import-progress', (event) => {
            const { status, message } = event.payload;
            if (status === 'error') setError(message);
            else setWorking(message);
        });
        const unlistenModpkg = listen<{ status: string; message: string }>('modpkg-import-progress', (event) => {
            const { status, message } = event.payload;
            if (status === 'error') setError(message);
            else setWorking(message);
        });
        return () => {
            unlistenFantome.then((fn) => fn());
            unlistenModpkg.then((fn) => fn());
        };
    }, [setWorking, setError]);

    const handleOpenProject = useCallback(async (projectPath: string) => {
        closeModal();
        try {
            setWorking('Opening project…');
            let normalizedPath = projectPath;
            if (normalizedPath.endsWith('.json')) {
                normalizedPath = normalizedPath.replace(/[\\/](mod\.config|flint|project)\.json$/, '');
            }

            const project = await api.openProject(normalizedPath);
            dispatch({ type: 'SET_PROJECT', payload: { project, path: normalizedPath } });

            try {
                const files = await api.listProjectFiles(normalizedPath);
                dispatch({ type: 'SET_FILE_TREE', payload: files });
            } catch (filesError) {
                console.error('Failed to load project files:', filesError);
            }

            setReady();

            const recent = state.recentProjects.filter((p) => p.path !== normalizedPath);
            recent.unshift({
                name: project.display_name || project.name,
                champion: project.champion,
                skin: project.skin_id,
                path: normalizedPath,
                lastOpened: new Date().toISOString(),
            });
            dispatch({ type: 'SET_RECENT_PROJECTS', payload: recent.slice(0, 10) });
        } catch (error) {
            console.error('Failed to open project:', error);
            const flintError = error as api.FlintError;
            setError(flintError.getUserMessage?.() || 'Failed to open project');
        }
    }, [state.recentProjects, dispatch, closeModal, setWorking, setReady, setError]);

    const handleBrowseFiles = useCallback(async () => {
        try {
            const selected = await open({
                title: 'Open Flint Project',
                filters: [{ name: 'Flint Project', extensions: ['json'] }],
                multiple: false,
            });
            if (selected) await handleOpenProject(selected as string);
        } catch (error) {
            console.error('Failed to open project:', error);
        }
    }, [handleOpenProject]);

    const handleRemoveProject = useCallback((e: React.MouseEvent, projectId: string) => {
        e.stopPropagation();
        const project = savedProjects.find((p) => p.id === projectId);
        if (!project) return;

        openConfirmDialog({
            title: 'Delete Project',
            message: `Are you sure you want to delete "${project.name}"?\n\nThis will permanently delete all project files and cannot be undone.`,
            confirmLabel: 'Delete',
            cancelLabel: 'Cancel',
            danger: true,
            onConfirm: async () => {
                try {
                    setRemovingId(projectId);
                    setWorking('Deleting project files…');
                    try {
                        await api.deleteProject(project.path);
                    } catch (deleteError) {
                        console.warn('Project folder may not exist, removing from list anyway:', deleteError);
                    }
                    setTimeout(() => {
                        dispatch({ type: 'REMOVE_SAVED_PROJECT', payload: projectId });
                        setRemovingId(null);
                        setReady();
                    }, 200);
                } catch (error) {
                    console.error('Failed to delete project:', error);
                    const flintError = error as api.FlintError;
                    setError(flintError.getUserMessage?.() || 'Failed to delete project');
                    setRemovingId(null);
                }
            },
        });
    }, [savedProjects, dispatch, setWorking, setReady, setError, openConfirmDialog]);

    const handleImportMod = useCallback(async () => {
        try {
            const selected = await open({
                title: 'Import Mod File',
                filters: [
                    { name: 'Mod Packages', extensions: ['fantome', 'modpkg'] },
                    { name: 'Fantome Package', extensions: ['fantome'] },
                    { name: 'ModPkg Package', extensions: ['modpkg'] },
                    { name: 'WAD Archive', extensions: ['wad', 'client'] },
                    { name: 'ZIP Archive', extensions: ['zip'] },
                    { name: 'All Files', extensions: ['*'] },
                ],
                multiple: false,
                directory: false,
            });
            if (!selected) return;

            const filePath = selected as string;
            const isModpkg = filePath.toLowerCase().endsWith('.modpkg');

            let champion: string;
            let skinId: number;
            let creatorName: string;
            let modName: string;

            if (isModpkg) {
                setWorking('Analyzing ModPkg…');
                const analysis = await api.analyzeModpkg(filePath);
                if (!analysis.is_champion_mod) {
                    setError('This does not appear to be a champion mod. Only champion mods are supported.');
                    return;
                }
                champion = analysis.champion || 'Unknown';
                skinId = analysis.skin_ids[0] || 0;
                creatorName = analysis.authors[0] || state.creatorName || 'Unknown';
                modName = analysis.display_name || analysis.name || `${champion}_Skin${skinId}_Imported`;
            } else {
                setWorking('Analyzing Fantome mod…');
                const analysis = await api.analyzeFantome(filePath);
                if (!analysis.is_champion_mod) {
                    setError('This does not appear to be a champion mod. Only champion mods are supported.');
                    return;
                }
                champion = analysis.champion || 'Unknown';
                skinId = analysis.skin_ids[0] || 0;
                creatorName = analysis.metadata?.author || state.creatorName || 'Unknown';
                modName = analysis.metadata?.name || `${champion}_Skin${skinId}_Imported`;
            }

            const appData = await appDataDir();
            const parts = appData.replace(/\\/g, '/').split('/');
            parts.pop();
            const defaultProjectsDir = `${parts.join('/')}/RitoShark/Flint/Projects`;

            const sanitizedModName = modName.replace(/[^a-zA-Z0-9_-]/g, '_');
            const dirName = `${champion}_Skin${skinId}_${sanitizedModName}`;

            setWorking('Importing and refathering mod files…');
            const projectDir = `${defaultProjectsDir}/${dirName}`;

            const options: api.ImportOptions = {
                refather: true,
                creator_name: creatorName,
                project_name: modName,
                target_skin_id: skinId,
                cleanup_unused: false,
                match_from_league: !isModpkg,
                league_path: state.leaguePath || null,
                use_jade: configStore.binConverterEngine === 'jade',
            };

            const project = isModpkg
                ? await api.importModpkg(filePath, projectDir, options)
                : await api.importFantomeWad(filePath, projectDir, options);

            setWorking('Opening project…');
            dispatch({ type: 'SET_PROJECT', payload: { project, path: projectDir } });

            try {
                const files = await api.listProjectFiles(projectDir);
                dispatch({ type: 'SET_FILE_TREE', payload: files });
            } catch (filesError) {
                console.error('Failed to load project files:', filesError);
            }

            const recent = state.recentProjects.filter((p) => p.path !== projectDir);
            recent.unshift({
                name: project.display_name || project.name,
                champion: project.champion,
                skin: project.skin_id,
                path: projectDir,
                lastOpened: new Date().toISOString(),
            });
            dispatch({ type: 'SET_RECENT_PROJECTS', payload: recent.slice(0, 10) });

            closeModal();
            setReady();
        } catch (error) {
            console.error('Failed to import mod:', error);
            const flintError = error as api.FlintError;
            closeModal();
            setError(flintError.getUserMessage?.() || 'Failed to import mod');
        }
    }, [state.leaguePath, state.creatorName, state.recentProjects, dispatch, closeModal, setWorking, setReady, setError, configStore.binConverterEngine]);

    // Filtered + sorted view
    const visibleProjects = useMemo(() => {
        const q = search.trim().toLowerCase();
        let list = q
            ? savedProjects.filter((p) =>
                `${p.champion} ${p.name} ${p.path}`.toLowerCase().includes(q))
            : savedProjects.slice();

        switch (sortMode) {
            case 'name':
                list.sort((a, b) => a.name.localeCompare(b.name));
                break;
            case 'champion':
                list.sort((a, b) => (a.champion || '').localeCompare(b.champion || '') || a.name.localeCompare(b.name));
                break;
            case 'recent':
            default:
                list.sort((a, b) => (b.lastOpened || '').localeCompare(a.lastOpened || ''));
                break;
        }
        return list;
    }, [savedProjects, search, sortMode]);

    return (
        <Modal open={isVisible} onClose={closeModal} modifier="modal--project-list">
            <ModalHeader
                title={
                    <span className="pl-title">
                        <span className="pl-title__icon"><Icon name="folder" /></span>
                        <span>
                            <span className="pl-title__name">My Projects</span>
                            <span className="pl-title__sub">
                                {savedProjects.length === 0
                                    ? 'No saved projects yet'
                                    : `${savedProjects.length} saved · open or import`}
                            </span>
                        </span>
                    </span>
                }
                onClose={closeModal}
            />

            {savedProjects.length > 0 && (
                <div className="pl-toolbar">
                    <div className="pl-search">
                        <Icon name="search" />
                        <Input
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            placeholder="Search by champion, name, or path…"
                        />
                    </div>
                    <Picker<SortMode>
                        value={sortMode}
                        onChange={(v) => setSortMode(v as SortMode)}
                        options={SORT_OPTIONS as unknown as { value: SortMode; label: string }[]}
                        width={200}
                    />
                </div>
            )}

            <ModalBody className="pl-body">
                {savedProjects.length === 0 ? (
                    <ProjectsEmpty onBrowse={handleBrowseFiles} onImport={handleImportMod} />
                ) : visibleProjects.length === 0 ? (
                    <div className="pl-no-match">
                        <Icon name="search" />
                        <span>No projects match “{search}”.</span>
                    </div>
                ) : (
                    <div className="pl-grid">
                        {visibleProjects.map((project: SavedProject, i) => (
                            <button
                                key={project.id}
                                type="button"
                                className={`pl-card ${removingId === project.id ? 'pl-card--removing' : ''}`}
                                onClick={() => handleOpenProject(project.path)}
                                style={{
                                    animationDelay: `${Math.min(i, 12) * 28}ms`,
                                    ['--pl-hue' as never]: hueFor(project.champion),
                                }}
                                title={`Open ${project.champion} — ${project.name}`}
                            >
                                <span className="pl-card__tile">
                                    <span className="pl-card__monogram">{monogram(project.champion)}</span>
                                </span>
                                <span className="pl-card__body">
                                    <span className="pl-card__name">{project.name}</span>
                                    <span className="pl-card__champ">{project.champion}</span>
                                    <span className="pl-card__path" title={project.path}>{project.path}</span>
                                </span>
                                <span className="pl-card__meta">
                                    <span className="pl-card__time">{formatRelativeTime(project.lastOpened)}</span>
                                    <span
                                        className="pl-card__remove"
                                        role="button"
                                        tabIndex={-1}
                                        onClick={(e) => handleRemoveProject(e, project.id)}
                                        title="Delete project"
                                    >
                                        <Icon name="trash" />
                                    </span>
                                </span>
                            </button>
                        ))}
                    </div>
                )}
            </ModalBody>

            <ModalFooter>
                <Button variant="secondary" icon="folder" onClick={handleBrowseFiles}>
                    Open from disk
                </Button>
                <Button variant="success" icon="download" onClick={handleImportMod}>
                    Import Mod
                </Button>
            </ModalFooter>
        </Modal>
    );
};

// =============================================================================
// Empty state
// =============================================================================

const ProjectsEmpty: React.FC<{ onBrowse: () => void; onImport: () => void }> = ({ onBrowse, onImport }) => (
    <div className="pl-empty">
        <div className="pl-empty__art">
            <span className="pl-empty__ring" />
            <span className="pl-empty__ring pl-empty__ring--2" />
            <span className="pl-empty__icon"><Icon name="folder" /></span>
        </div>
        <h3 className="pl-empty__title">No projects yet</h3>
        <p className="pl-empty__desc">
            Create a new project, open one from disk, or import a <code>.fantome</code> /
            <code>.modpkg</code> to get started.
        </p>
        <div className="pl-empty__actions">
            <Button icon="folder" onClick={onBrowse}>Open from disk</Button>
            <Button variant="success" icon="download" onClick={onImport}>Import Mod</Button>
        </div>
    </div>
);
