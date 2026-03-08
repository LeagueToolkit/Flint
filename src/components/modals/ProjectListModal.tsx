/**
 * Flint - Project List Modal Component
 * Shows saved projects with animations and an Import Projects (TODO) button
 */

import React, { useState, useCallback } from 'react';
import { useAppState } from '../../lib/stores';
import { formatRelativeTime } from '../../lib/utils';
import { open } from '@tauri-apps/plugin-dialog';
import { appDataDir } from '@tauri-apps/api/path';
import { getIcon } from '../../lib/fileIcons';
import * as api from '../../lib/api';

export const ProjectListModal: React.FC = () => {
    const { state, dispatch, closeModal, setWorking, setReady, setError } = useAppState();
    const [removingId, setRemovingId] = useState<string | null>(null);

    const isVisible = state.activeModal === 'projectList';
    const savedProjects = state.savedProjects || [];

    const handleOpenProject = useCallback(async (projectPath: string) => {
        closeModal();

        try {
            setWorking('Opening project...');

            // Normalize path - strip project file name if present
            let normalizedPath = projectPath;
            if (normalizedPath.endsWith('mod.config.json') || normalizedPath.endsWith('project.json')) {
                normalizedPath = normalizedPath.replace(/[\\/](mod\.config|project)\.json$/, '');
            }

            const project = await api.openProject(normalizedPath);

            dispatch({
                type: 'SET_PROJECT',
                payload: { project, path: normalizedPath },
            });

            // Determine project directory
            let projectDir = normalizedPath;

            // Fetch file tree
            try {
                const files = await api.listProjectFiles(projectDir);
                dispatch({ type: 'SET_FILE_TREE', payload: files });
            } catch (filesError) {
                console.error('Failed to load project files:', filesError);
            }

            setReady();

            // Update recent projects
            const recent = state.recentProjects.filter(p => p.path !== projectPath);
            recent.unshift({
                name: project.display_name || project.name,
                champion: project.champion,
                skin: project.skin_id,
                path: projectPath,
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

            if (selected) {
                await handleOpenProject(selected as string);
            }
        } catch (error) {
            console.error('Failed to open project:', error);
        }
    }, [handleOpenProject]);

    const handleRemoveProject = useCallback((e: React.MouseEvent, projectId: string) => {
        e.stopPropagation();
        setRemovingId(projectId);

        // Allow fade-out animation to play
        setTimeout(() => {
            dispatch({ type: 'REMOVE_SAVED_PROJECT', payload: projectId });
            setRemovingId(null);
        }, 200);
    }, [dispatch]);

    const handleImportFantome = useCallback(async () => {
        try {
            const selected = await open({
                title: 'Select Fantome Mod or WAD File',
                filters: [
                    { name: 'All Supported Formats', extensions: ['fantome', 'wad', 'client'] },
                    { name: 'Fantome Package', extensions: ['fantome'] },
                    { name: 'WAD Archive', extensions: ['wad', 'client'] },
                ],
                multiple: false,
                directory: false,
            });

            if (!selected) return;

            closeModal();
            setWorking('Analyzing Fantome mod...');

            const analysis = await api.analyzeFantome(selected as string);

            if (!analysis.is_champion_mod) {
                setError('This does not appear to be a champion mod. Only champion mods are supported.');
                return;
            }

            const champion = analysis.champion || 'Unknown';
            const skinId = analysis.skin_ids[0] || 0;

            // Extract metadata from Fantome package (if available)
            const creatorName = analysis.metadata?.author || state.creatorName || 'Unknown';
            const modName = analysis.metadata?.name || `${champion}_Skin${skinId}_Imported`;

            // Get default project directory: {AppData}/RitoShark/Flint/Projects
            const appData = await appDataDir();
            const parts = appData.replace(/\\/g, '/').split('/');
            parts.pop();
            const defaultProjectsDir = `${parts.join('/')}/RitoShark/Flint/Projects`;

            // Create a proper Flint project
            setWorking('Creating project...');
            // Sanitize project name for filesystem (remove special characters)
            const projectName = modName.replace(/[^a-zA-Z0-9_-]/g, '_');

            const project = await api.createProject({
                name: projectName,
                champion,
                skin: skinId,
                creatorName,
                projectPath: defaultProjectsDir,
                leaguePath: state.leaguePath || '',
            });

            // Extract WAD files into the project's content folder with refathering and missing file matching
            setWorking('Extracting and refathering mod files...');
            const projectDir = `${defaultProjectsDir}/${projectName}`;
            const contentDir = `${projectDir}/content`;

            const options: api.ImportOptions = {
                refather: true, // Enable refathering to apply proper mod structure
                creator_name: creatorName,
                project_name: projectName,
                target_skin_id: skinId,
                cleanup_unused: true,
                match_from_league: true, // Match missing files from League installation
                league_path: state.leaguePath || null,
            };

            await api.importFantomeWad(selected as string, contentDir, options);

            // Open the project
            setWorking('Opening project...');

            dispatch({
                type: 'SET_PROJECT',
                payload: { project, path: projectDir },
            });

            // Fetch file tree
            try {
                const files = await api.listProjectFiles(projectDir);
                dispatch({ type: 'SET_FILE_TREE', payload: files });
            } catch (filesError) {
                console.error('Failed to load project files:', filesError);
            }

            setReady();

            // Update recent projects
            const recent = state.recentProjects.filter(p => p.path !== projectDir);
            recent.unshift({
                name: project.display_name || project.name,
                champion: project.champion,
                skin: project.skin_id,
                path: projectDir,
                lastOpened: new Date().toISOString(),
            });
            dispatch({ type: 'SET_RECENT_PROJECTS', payload: recent.slice(0, 10) });

        } catch (error) {
            console.error('Failed to import Fantome mod:', error);
            const flintError = error as api.FlintError;
            setError(flintError.getUserMessage?.() || 'Failed to import Fantome mod');
        }
    }, [state.leaguePath, state.creatorName, state.recentProjects, dispatch, closeModal, setWorking, setReady, setError]);

    if (!isVisible) return null;

    return (
        <div className={`modal-overlay ${isVisible ? 'modal-overlay--visible' : ''}`}>
            <div className="modal modal--project-list">
                <div className="modal__header">
                    <h2 className="modal__title">
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                            <path d="M3 7V17C3 18.1046 3.89543 19 5 19H19C20.1046 19 21 18.1046 21 17V9C21 7.89543 20.1046 7 19 7H13L11 5H5C3.89543 5 3 5.89543 3 7Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                        </svg>
                        My Projects
                    </h2>
                    <button className="modal__close" onClick={closeModal}>×</button>
                </div>

                <div className="modal__body project-list__body">
                    {savedProjects.length === 0 ? (
                        <div className="project-list__empty">
                            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" style={{ opacity: 0.3 }}>
                                <path d="M3 7V17C3 18.1046 3.89543 19 5 19H19C20.1046 19 21 18.1046 21 17V9C21 7.89543 20.1046 7 19 7H13L11 5H5C3.89543 5 3 5.89543 3 7Z" stroke="currentColor" strokeWidth="1.5" />
                            </svg>
                            <div className="project-list__empty-title">No Projects Yet</div>
                            <div className="project-list__empty-desc">
                                Create a new project or browse for an existing one to get started.
                            </div>
                        </div>
                    ) : (
                        <div className="project-list__items">
                            {savedProjects.map((project, index) => (
                                <div
                                    key={project.id}
                                    className={`project-list__item ${removingId === project.id ? 'project-list__item--removing' : ''}`}
                                    onClick={() => handleOpenProject(project.path)}
                                    style={{ animationDelay: `${index * 50}ms` }}
                                >
                                    <div className="project-list__item-icon">
                                        <span dangerouslySetInnerHTML={{ __html: getIcon('folder') }} />
                                    </div>
                                    <div className="project-list__item-info">
                                        <div className="project-list__item-name">
                                            {project.champion} — {project.name}
                                        </div>
                                        <div className="project-list__item-path" title={project.path}>
                                            {project.path}
                                        </div>
                                    </div>
                                    <div className="project-list__item-meta">
                                        <span className="project-list__item-date">
                                            {formatRelativeTime(project.lastOpened)}
                                        </span>
                                        <button
                                            className="project-list__item-remove"
                                            onClick={(e) => handleRemoveProject(e, project.id)}
                                            title="Remove from list"
                                        >
                                            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                                                <path d="M4.5 4.5l7 7m0-7l-7 7" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                                            </svg>
                                        </button>
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}
                </div>

                <div className="modal__footer project-list__footer">
                    <button className="btn btn--secondary" onClick={handleImportFantome} title="Import Fantome WAD mod">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                        </svg>
                        Import Fantome Mod
                    </button>
                    <button className="btn btn--primary" onClick={handleBrowseFiles}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                            <path d="M3 7V17C3 18.1046 3.89543 19 5 19H19C20.1046 19 21 18.1046 21 17V9C21 7.89543 20.1046 7 19 7H13L11 5H5C3.89543 5 3 5.89543 3 7Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                        </svg>
                        Browse Files
                    </button>
                </div>
            </div>
        </div>
    );
};
