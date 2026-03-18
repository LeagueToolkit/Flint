/**
 * Flint - File Tree Component
 */

import React, { useState, useMemo, useCallback, useRef, useEffect, CSSProperties } from 'react';
import { useAppState } from '../lib/stores';
import { getFileIcon, getExpanderIcon, getIcon } from '../lib/fileIcons';
import * as api from '../lib/api';
import { openPath } from '@tauri-apps/plugin-opener';
import type { FileTreeNode, ProjectTab, ContextMenuOption } from '../lib/types';

// Helper to get active tab
function getActiveTab(state: { activeTabId: string | null; openTabs: ProjectTab[] }): ProjectTab | null {
    if (!state.activeTabId) return null;
    return state.openTabs.find(t => t.id === state.activeTabId) || null;
}

interface LeftPanelProps {
    style?: CSSProperties;
}

export const LeftPanel: React.FC<LeftPanelProps> = ({ style }) => {
    const { state } = useAppState();
    const [searchQuery, setSearchQuery] = useState('');

    const activeTab = getActiveTab(state);
    const hasProject = !!activeTab;

    if (!hasProject) {
        return <ProjectsPanel />;
    }

    return (
        <aside className="left-panel" id="left-panel" style={style}>
            <div className="search-box">
                <input
                    type="text"
                    className="search-box__input"
                    placeholder="Filter files..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                />
            </div>
            <FileTree searchQuery={searchQuery} />
        </aside>
    );
};

interface FileTreeProps {
    searchQuery: string;
}

const FileTree: React.FC<FileTreeProps> = ({ searchQuery }) => {
    const { state, dispatch } = useAppState();

    // Get active tab for file tree data
    const activeTab = getActiveTab(state);
    const fileTree = activeTab?.fileTree || null;
    const selectedFile = activeTab?.selectedFile || null;
    const expandedFolders = activeTab?.expandedFolders || new Set<string>();

    // Rename state — shared across all tree nodes
    const [renamingPath, setRenamingPath] = useState<string | null>(null);

    const handleItemClick = useCallback((path: string, isFolder: boolean) => {
        if (isFolder) {
            dispatch({ type: 'TOGGLE_FOLDER', payload: path });
        } else {
            // Update selected file on active tab
            if (activeTab) {
                dispatch({
                    type: 'SET_TAB_SELECTED_FILE',
                    payload: { tabId: activeTab.id, filePath: path }
                });
                dispatch({ type: 'SET_STATE', payload: { currentView: 'preview' } });
            }
        }
    }, [dispatch, activeTab]);

    const handleDeepToggle = useCallback((paths: string[], expand: boolean) => {
        dispatch({ type: 'BULK_SET_FOLDERS', payload: { paths, expand } });
    }, [dispatch]);

    const filteredTree = useMemo(() => {
        if (!fileTree || !searchQuery) return fileTree;
        return filterTreeByQuery(fileTree, searchQuery.toLowerCase());
    }, [fileTree, searchQuery]);

    if (!filteredTree) {
        return (
            <div className="file-tree">
                <div className="file-tree__empty">No project files loaded</div>
            </div>
        );
    }

    return (
        <div className="file-tree">
            <TreeNode
                node={filteredTree}
                depth={0}
                selectedFile={selectedFile}
                expandedFolders={expandedFolders}
                onItemClick={handleItemClick}
                onDeepToggle={handleDeepToggle}
                renamingPath={renamingPath}
                setRenamingPath={setRenamingPath}
            />
        </div>
    );
};

interface TreeNodeProps {
    node: FileTreeNode;
    depth: number;
    selectedFile: string | null;
    expandedFolders: Set<string>;
    onItemClick: (path: string, isFolder: boolean) => void;
    onDeepToggle: (paths: string[], expand: boolean) => void;
    renamingPath: string | null;
    setRenamingPath: (path: string | null) => void;
}

// Compact folders: merge single-child directory chains into one label
function compactNode(node: FileTreeNode): { displayPath: string; effectiveNode: FileTreeNode } {
    let current = node;
    const parts = [current.name];
    while (
        current.isDirectory &&
        current.children?.length === 1 &&
        current.children[0].isDirectory
    ) {
        current = current.children[0];
        parts.push(current.name);
    }
    return { displayPath: parts.join('/'), effectiveNode: current };
}

// Collect all descendant folder paths for deep expand/collapse
function collectAllFolderPaths(node: FileTreeNode): string[] {
    if (!node.isDirectory) return [];
    const result = [node.path];
    for (const child of node.children ?? []) {
        result.push(...collectAllFolderPaths(child));
    }
    return result;
}

// Check if a path is the "content" folder (root layer folder for the project)
function isContentFolder(path: string): boolean {
    const normalized = path.replace(/\\/g, '/');
    return normalized === 'content' || normalized.endsWith('/content');
}

// Get just the filename from a path
function getFileName(path: string): string {
    const parts = path.replace(/\\/g, '/').split('/');
    return parts[parts.length - 1] || path;
}

const TreeNode: React.FC<TreeNodeProps> = React.memo(({
    node,
    depth,
    selectedFile,
    expandedFolders,
    onItemClick,
    onDeepToggle,
    renamingPath,
    setRenamingPath,
}) => {
    const { state, dispatch, openModal, openContextMenu, openConfirmDialog, showToast } = useAppState();
    const renameInputRef = useRef<HTMLInputElement>(null);

    // Apply compact-folder merging
    const { displayPath, effectiveNode } = node.isDirectory ? compactNode(node) : { displayPath: node.name, effectiveNode: node };
    const isExpanded = expandedFolders.has(effectiveNode.path);
    const isSelected = selectedFile === effectiveNode.path;
    const isRenaming = renamingPath === effectiveNode.path;

    const activeTab = getActiveTab(state);
    const projectPath = activeTab?.projectPath || '';

    // Focus rename input when it appears
    useEffect(() => {
        if (isRenaming && renameInputRef.current) {
            const input = renameInputRef.current;
            input.focus();
            // Select filename without extension for files
            const name = getFileName(effectiveNode.path);
            const dotIdx = name.lastIndexOf('.');
            if (!effectiveNode.isDirectory && dotIdx > 0) {
                input.setSelectionRange(0, dotIdx);
            } else {
                input.select();
            }
        }
    }, [isRenaming]);

    const refreshFileTree = async () => {
        if (!activeTab) return;
        const files = await api.listProjectFiles(activeTab.projectPath);
        dispatch({ type: 'SET_FILE_TREE', payload: files });
    };

    const handleClick = (e: React.MouseEvent) => {
        e.stopPropagation();
        if (isRenaming) return;
        if (e.shiftKey && effectiveNode.isDirectory) {
            // Deep expand/collapse
            const allPaths = collectAllFolderPaths(effectiveNode);
            onDeepToggle(allPaths, !isExpanded);
        } else {
            onItemClick(effectiveNode.path, effectiveNode.isDirectory);
        }
    };

    const handleRenameSubmit = async (newName: string) => {
        setRenamingPath(null);
        const currentName = getFileName(effectiveNode.path);
        if (!newName || newName === currentName) return;

        try {
            const result = await api.renameFile(projectPath, effectiveNode.path, newName);
            await refreshFileTree();
            if (result.bin_updates > 0) {
                showToast('success', `Renamed and updated ${result.bin_updates} BIN file${result.bin_updates > 1 ? 's' : ''}`);
            } else {
                showToast('success', 'File renamed');
            }
        } catch (err) {
            const flintError = err as api.FlintError;
            showToast('error', flintError.getUserMessage?.() || 'Failed to rename');
        }
    };

    const handleRenameKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            handleRenameSubmit(e.currentTarget.value.trim());
        } else if (e.key === 'Escape') {
            setRenamingPath(null);
        }
    };

    const handleContextMenu = (e: React.MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();

        const fullPath = projectPath
            ? `${projectPath.replace(/\\/g, '/')}/${effectiveNode.path}`
            : effectiveNode.path;
        const fileName = getFileName(effectiveNode.path);
        const ext = fileName.includes('.') ? fileName.split('.').pop()?.toLowerCase() || '' : '';

        const options: ContextMenuOption[] = [];

        if (effectiveNode.isDirectory) {
            // --- Directory context menu ---

            // Content folder gets "Create New Layer" placeholder
            if (isContentFolder(effectiveNode.path)) {
                options.push({
                    label: 'Create New Layer',
                    icon: getIcon('plus'),
                    onClick: () => showToast('info', 'Layer creation coming soon!'),
                    disabled: true,
                });
                options.push({
                    label: 'Batch Recolor',
                    icon: getIcon('texture'),
                    separator: true,
                    onClick: () => openModal('recolor', { filePath: effectiveNode.path, isFolder: true }),
                });
            } else {
                options.push({
                    label: 'Batch Recolor',
                    icon: getIcon('texture'),
                    onClick: () => openModal('recolor', { filePath: effectiveNode.path, isFolder: true }),
                });
            }

            options.push({
                label: 'New Folder',
                icon: getIcon('folder'),
                separator: true,
                onClick: async () => {
                    const newDir = `${effectiveNode.path}/New Folder`;
                    try {
                        await api.createDirectory(projectPath, newDir);
                        await refreshFileTree();
                    } catch (err) {
                        const flintError = err as api.FlintError;
                        showToast('error', flintError.getUserMessage?.() || 'Failed to create folder');
                    }
                },
            });

            options.push({
                label: 'Rename',
                icon: getIcon('text'),
                onClick: () => setRenamingPath(effectiveNode.path),
            });

            options.push({
                label: 'Copy Path',
                icon: getIcon('code'),
                separator: true,
                onClick: () => navigator.clipboard.writeText(fullPath.replace(/\//g, '\\')),
            });
            options.push({
                label: 'Copy Relative Path',
                onClick: () => navigator.clipboard.writeText(effectiveNode.path),
            });

            options.push({
                label: 'Reveal in Explorer',
                icon: getIcon('folderOpen2'),
                separator: true,
                onClick: () => api.openInExplorer(fullPath.replace(/\//g, '\\')).catch(() => {}),
            });

            options.push({
                label: 'Delete',
                icon: getIcon('trash'),
                danger: true,
                separator: true,
                onClick: () => {
                    openConfirmDialog({
                        title: 'Delete Folder',
                        message: `Are you sure you want to delete "${fileName}" and all its contents? This cannot be undone.`,
                        confirmLabel: 'Delete',
                        danger: true,
                        onConfirm: async () => {
                            try {
                                await api.deleteFile(projectPath, effectiveNode.path);
                                await refreshFileTree();
                                showToast('success', 'Folder deleted');
                            } catch (err) {
                                const flintError = err as api.FlintError;
                                showToast('error', flintError.getUserMessage?.() || 'Failed to delete folder');
                            }
                        },
                    });
                },
            });
        } else {
            // --- File context menu ---

            options.push({
                label: 'Rename',
                icon: getIcon('text'),
                onClick: () => setRenamingPath(effectiveNode.path),
            });

            options.push({
                label: 'Duplicate',
                icon: getIcon('file'),
                onClick: async () => {
                    try {
                        await api.duplicateFile(projectPath, effectiveNode.path);
                        await refreshFileTree();
                        showToast('success', 'File duplicated');
                    } catch (err) {
                        const flintError = err as api.FlintError;
                        showToast('error', flintError.getUserMessage?.() || 'Failed to duplicate');
                    }
                },
            });

            // Recolor option for texture files
            if (ext === 'dds' || ext === 'tex') {
                options.push({
                    label: 'Recolor',
                    icon: getIcon('texture'),
                    separator: true,
                    onClick: () => openModal('recolor', { filePath: effectiveNode.path, isFolder: false }),
                });
            }

            options.push({
                label: 'Copy Path',
                icon: getIcon('code'),
                separator: true,
                onClick: () => navigator.clipboard.writeText(fullPath.replace(/\//g, '\\')),
            });
            options.push({
                label: 'Copy Relative Path',
                onClick: () => navigator.clipboard.writeText(effectiveNode.path),
            });

            options.push({
                label: 'Reveal in Explorer',
                icon: getIcon('folderOpen2'),
                separator: true,
                onClick: () => api.openInExplorer(fullPath.replace(/\//g, '\\')).catch(() => {}),
            });
            options.push({
                label: 'Open with Default App',
                onClick: async () => {
                    try {
                        // Normalize path: ensure consistent backslashes for Windows
                        const normalizedPath = fullPath.replace(/\//g, '\\');
                        await openPath(normalizedPath);
                    } catch (err) {
                        const message = (err as Error).message || String(err);
                        console.error('[FileTree] Failed to open file:', message);
                        showToast('error', `Failed to open file: ${message}`);
                    }
                },
            });

            options.push({
                label: 'Delete',
                icon: getIcon('trash'),
                danger: true,
                separator: true,
                onClick: () => {
                    openConfirmDialog({
                        title: 'Delete File',
                        message: `Are you sure you want to delete "${fileName}"? This cannot be undone.`,
                        confirmLabel: 'Delete',
                        danger: true,
                        onConfirm: async () => {
                            try {
                                await api.deleteFile(projectPath, effectiveNode.path);
                                await refreshFileTree();
                                showToast('success', 'File deleted');
                            } catch (err) {
                                const flintError = err as api.FlintError;
                                showToast('error', flintError.getUserMessage?.() || 'Failed to delete file');
                            }
                        },
                    });
                },
            });
        }

        openContextMenu(e.clientX, e.clientY, options);
    };

    const icon = getFileIcon(effectiveNode.name, effectiveNode.isDirectory, isExpanded);
    const expanderIcon = getExpanderIcon(isExpanded);

    return (
        <div className="file-tree__node">
            <div
                className={`file-tree__item ${isSelected ? 'file-tree__item--selected' : ''}`}
                style={{ paddingLeft: 4 + depth * 12 }}
                onClick={handleClick}
                onContextMenu={handleContextMenu}
            >
                {effectiveNode.isDirectory ? (
                    <span
                        className="file-tree__expander"
                        dangerouslySetInnerHTML={{ __html: expanderIcon }}
                    />
                ) : (
                    <span className="file-tree__expander" style={{ visibility: 'hidden' }} />
                )}
                <span
                    className="file-tree__icon"
                    dangerouslySetInnerHTML={{ __html: icon }}
                />
                {isRenaming ? (
                    <input
                        ref={renameInputRef}
                        className="file-tree__rename-input"
                        defaultValue={getFileName(effectiveNode.path)}
                        onBlur={(e) => handleRenameSubmit(e.currentTarget.value.trim())}
                        onKeyDown={handleRenameKeyDown}
                        onClick={(e) => e.stopPropagation()}
                    />
                ) : (
                    <span className="file-tree__name">
                        {displayPath.includes('/') ? (
                            displayPath.split('/').map((segment, idx, arr) => (
                                <React.Fragment key={idx}>
                                    <span className="file-tree__compact-segment">{segment}</span>
                                    {idx < arr.length - 1 && <span className="file-tree__compact-separator">/</span>}
                                </React.Fragment>
                            ))
                        ) : (
                            displayPath
                        )}
                    </span>
                )}
            </div>
            {effectiveNode.isDirectory && isExpanded && effectiveNode.children && (
                <div className="file-tree__children">
                    {effectiveNode.children.map((child) => (
                        <TreeNode
                            key={child.path}
                            node={child}
                            depth={depth + 1}
                            selectedFile={selectedFile}
                            expandedFolders={expandedFolders}
                            onItemClick={onItemClick}
                            onDeepToggle={onDeepToggle}
                            renamingPath={renamingPath}
                            setRenamingPath={setRenamingPath}
                        />
                    ))}
                </div>
            )}
        </div>
    );
});

const ProjectsPanel: React.FC = () => {
    const { state, dispatch, openModal, setWorking, setReady, setError } = useAppState();

    const handleOpenProject = async (projectPath: string) => {
        try {
            setWorking('Opening project...');

            // Normalize path - strip project file name if present
            // Handles: mod.config.json, flint.json, project.json
            let normalizedPath = projectPath;
            if (normalizedPath.endsWith('.json')) {
                normalizedPath = normalizedPath.replace(/[\\/](mod\.config|flint|project)\.json$/, '');
            }

            const project = await api.openProject(normalizedPath);

            dispatch({ type: 'SET_PROJECT', payload: { project, path: normalizedPath } });

            const files = await api.listProjectFiles(normalizedPath);
            dispatch({ type: 'SET_FILE_TREE', payload: files });
            setReady();
        } catch (error) {
            console.error('Failed to open project:', error);
            const flintError = error as api.FlintError;
            setError(flintError.getUserMessage?.() || 'Failed to open project');
        }
    };

    return (
        <aside className="left-panel projects-panel">
            <div className="projects-panel__header">
                <span className="projects-panel__title">Projects</span>
                <button
                    className="btn btn--ghost btn--small"
                    title="New Project"
                    onClick={() => openModal('newProject')}
                    dangerouslySetInnerHTML={{ __html: getIcon('plus') }}
                />
            </div>
            <div className="projects-panel__list">
                {state.recentProjects.length === 0 ? (
                    <div className="projects-panel__empty">
                        <p>No recent projects</p>
                        <p style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                            Create a new project to get started
                        </p>
                    </div>
                ) : (
                    state.recentProjects.map((project) => (
                        <div
                            key={project.path}
                            className="projects-panel__item"
                            onClick={() => handleOpenProject(project.path)}
                        >
                            <span
                                className="projects-panel__icon"
                                dangerouslySetInnerHTML={{ __html: getIcon('folder') }}
                            />
                            <div className="projects-panel__info">
                                <div className="projects-panel__name">
                                    {project.champion} - {project.name}
                                </div>
                                <div className="projects-panel__meta">Skin {project.skin}</div>
                            </div>
                        </div>
                    ))
                )}
            </div>
        </aside>
    );
};

function filterTreeByQuery(node: FileTreeNode, query: string): FileTreeNode | null {
    if (node.name.toLowerCase().includes(query)) {
        return node;
    }

    if (node.isDirectory && node.children) {
        const filteredChildren = node.children
            .map((child) => filterTreeByQuery(child, query))
            .filter((child): child is FileTreeNode => child !== null);

        if (filteredChildren.length > 0) {
            return { ...node, children: filteredChildren };
        }
    }

    return null;
}

export { FileTree, ProjectsPanel };
