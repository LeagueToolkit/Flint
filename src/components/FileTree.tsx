/**
 * Flint - File Tree Component
 */

import React, { useState, useMemo, useCallback, useRef, useEffect, CSSProperties } from 'react';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { useAppMetadataStore, useProjectTabStore, useModalStore, useNotificationStore, useConfigStore, useNavigationStore } from '../lib/stores';
import { getFileIcon, getExpanderIcon, getIcon } from '../lib/fileIcons';
import * as api from '../lib/api';
import type { FileTreeNode, ProjectTab, ContextMenuOption } from '../lib/types';

// Helper to get active tab from projectTabStore state
function getActiveTab(activeTabId: string | null, openTabs: ProjectTab[]): ProjectTab | null {
    if (!activeTabId) return null;
    return openTabs.find(t => t.id === activeTabId) || null;
}

interface LeftPanelProps {
    style?: CSSProperties;
}

export const LeftPanel: React.FC<LeftPanelProps> = ({ style }) => {
    const activeTabId = useProjectTabStore((s) => s.activeTabId);
    const openTabs = useProjectTabStore((s) => s.openTabs);
    const [searchQuery, setSearchQuery] = useState('');

    const activeTab = getActiveTab(activeTabId, openTabs);
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
    const activeTabId = useProjectTabStore((s) => s.activeTabId);
    const openTabs = useProjectTabStore((s) => s.openTabs);
    const toggleFolder = useProjectTabStore((s) => s.toggleFolder);
    const setFileTree = useProjectTabStore((s) => s.setFileTree);
    const setSelectedFile = useProjectTabStore((s) => s.setSelectedFile);
    const bulkSetFolders = useProjectTabStore((s) => s.bulkSetFolders);
    const showToast = useNotificationStore((s) => s.showToast);

    // Get active tab for file tree data
    const activeTab = getActiveTab(activeTabId, openTabs);
    const fileTree = activeTab?.fileTree || null;
    const selectedFile = activeTab?.selectedFile || null;
    const expandedFolders = activeTab?.expandedFolders || new Set<string>();

    // Subscribe to file tree version changes — auto-refresh when files are created/removed
    const fileTreeVersion = useAppMetadataStore((s) => s.fileTreeVersion);
    useEffect(() => {
        if (!activeTab || fileTreeVersion === 0) return;
        api.listProjectFiles(activeTab.projectPath).then((files) => {
            setFileTree(activeTab.id, files);
        }).catch(() => {});
    }, [fileTreeVersion]); // eslint-disable-line react-hooks/exhaustive-deps

    // Rename state — shared across all tree nodes
    const [renamingPath, setRenamingPath] = useState<string | null>(null);

    // External drag & drop (OS files dropped onto the file tree).
    // WebView2 swallows HTML5 dragover/drop, so we use Tauri's webview-level
    // drag-drop event and hit-test the DOM with `position` from the payload.
    const [dropTargetPath, setDropTargetPath] = useState<string | null>(null);

    // Fresh refs so the listener always sees current state
    const activeTabRef = useRef(activeTab);
    useEffect(() => { activeTabRef.current = activeTab; });
    const setFileTreeRef = useRef(setFileTree);
    useEffect(() => { setFileTreeRef.current = setFileTree; });
    const showToastRef = useRef(showToast);
    useEffect(() => { showToastRef.current = showToast; });
    const bulkSetFoldersRef = useRef(bulkSetFolders);
    useEffect(() => { bulkSetFoldersRef.current = bulkSetFolders; });

    useEffect(() => {
        let unlisten: (() => void) | null = null;
        let expandTimer: number | null = null;
        let lastHover: string | null = null;
        let cancelled = false;

        const hitTest = (x: number, y: number): string | null => {
            const el = document.elementFromPoint(x, y) as HTMLElement | null;
            const folderEl = el?.closest('[data-drop-path]') as HTMLElement | null;
            return folderEl?.getAttribute('data-drop-path') ?? null;
        };

        // The webview drag-drop event reports physical pixel coordinates; the
        // DOM uses CSS pixels. Divide by devicePixelRatio.
        const cssCoords = (pos: { x: number; y: number }) => ({
            x: pos.x / window.devicePixelRatio,
            y: pos.y / window.devicePixelRatio,
        });

        getCurrentWebview()
            .onDragDropEvent((event) => {
                if (cancelled) return;
                const { type } = event.payload as { type: string };

                if (type === 'over') {
                    const { x, y } = cssCoords((event.payload as any).position);
                    const path = hitTest(x, y);
                    if (path !== lastHover) {
                        if (expandTimer !== null) { clearTimeout(expandTimer); expandTimer = null; }
                        lastHover = path;
                        setDropTargetPath(path);
                        if (path) {
                            const target = path;
                            expandTimer = window.setTimeout(() => {
                                const tab = activeTabRef.current;
                                if (tab) bulkSetFoldersRef.current(tab.id, [target], true);
                                expandTimer = null;
                            }, 1200);
                        }
                    }
                } else if (type === 'drop') {
                    const payload = event.payload as { position: { x: number; y: number }; paths: string[] };
                    const { x, y } = cssCoords(payload.position);
                    const target = hitTest(x, y) ?? lastHover;
                    if (expandTimer !== null) { clearTimeout(expandTimer); expandTimer = null; }
                    lastHover = null;
                    setDropTargetPath(null);

                    const tab = activeTabRef.current;
                    if (!tab || !target || !payload.paths?.length) return;

                    (async () => {
                        try {
                            const created = await api.importExternalFiles(tab.projectPath, target, payload.paths);
                            const files = await api.listProjectFiles(tab.projectPath);
                            setFileTreeRef.current(tab.id, files);
                            showToastRef.current('success', `Imported ${created.length} item${created.length === 1 ? '' : 's'}`);
                        } catch (err) {
                            const fe = err as api.FlintError;
                            showToastRef.current('error', fe.getUserMessage?.() || 'Failed to import');
                        }
                    })();
                } else {
                    // 'leave' / 'cancelled'
                    if (expandTimer !== null) { clearTimeout(expandTimer); expandTimer = null; }
                    lastHover = null;
                    setDropTargetPath(null);
                }
            })
            .then((fn) => {
                if (cancelled) fn();
                else unlisten = fn;
            })
            .catch(() => {});

        return () => {
            cancelled = true;
            if (expandTimer !== null) clearTimeout(expandTimer);
            if (unlisten) unlisten();
        };
    }, []);

    const dragProps: DragProps = useMemo(() => ({
        dropTargetPath,
    }), [dropTargetPath]);

    const handleItemClick = useCallback((path: string, isFolder: boolean) => {
        if (isFolder) {
            if (activeTab) toggleFolder(activeTab.id, path);
        } else {
            if (activeTab) {
                setSelectedFile(activeTab.id, path);
                useNavigationStore.getState().setView('preview');
            }
        }
    }, [activeTab, toggleFolder, setSelectedFile]);

    const handleDeepToggle = useCallback((paths: string[], expand: boolean) => {
        if (activeTab) bulkSetFolders(activeTab.id, paths, expand);
    }, [activeTab, bulkSetFolders]);

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
                projectPath={activeTab?.projectPath || ''}
                dragProps={dragProps}
            />
        </div>
    );
};

interface DragProps {
    dropTargetPath: string | null;
}

interface TreeNodeProps {
    node: FileTreeNode;
    depth: number;
    selectedFile: string | null;
    expandedFolders: Set<string>;
    onItemClick: (path: string, isFolder: boolean) => void;
    onDeepToggle: (paths: string[], expand: boolean) => void;
    renamingPath: string | null;
    setRenamingPath: (path: string | null) => void;
    projectPath: string;
    dragProps: DragProps;
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
    projectPath,
    dragProps,
}) => {
    const openModal = useModalStore((s) => s.openModal);
    const openContextMenu = useModalStore((s) => s.openContextMenu);
    const openConfirmDialog = useModalStore((s) => s.openConfirmDialog);
    const showToast = useNotificationStore((s) => s.showToast);
    const renameInputRef = useRef<HTMLInputElement>(null);

    // Apply compact-folder merging
    const { displayPath, effectiveNode } = node.isDirectory ? compactNode(node) : { displayPath: node.name, effectiveNode: node };
    const isExpanded = expandedFolders.has(effectiveNode.path);
    const isSelected = selectedFile === effectiveNode.path;
    const isRenaming = renamingPath === effectiveNode.path;

    // Drop-target highlight when external files are dragged over this folder
    const isDropTarget = dragProps.dropTargetPath === effectiveNode.path;

    // Subscribe to file status for THIS node only (not all statuses)
    const fullPath = projectPath ? `${projectPath}/${effectiveNode.path}`.replaceAll('\\', '/') : '';
    const fileStatus = useAppMetadataStore((s) => s.fileStatuses[fullPath]);

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
        if (!projectPath) return;
        const files = await api.listProjectFiles(projectPath);
        const { activeTabId } = useProjectTabStore.getState();
        if (activeTabId) useProjectTabStore.getState().setFileTree(activeTabId, files);
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

            // Root project folder: Set Thumbnail option
            if (depth === 0 && projectPath) {
                options.push({
                    label: 'Set Thumbnail',
                    icon: getIcon('document'),
                    onClick: () => openModal('thumbnail', { projectPath }),
                });
                options.push({
                    label: 'Edit Project Info',
                    icon: getIcon('code'),
                    onClick: () => {
                        const configPath = `${projectPath.replace(/\\/g, '/')}/mod.config.json`;
                        openModal('modConfig', { filePath: configPath });
                    },
                    separator: true,
                });
            }

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

            // Special options for mod.config.json
            if (fileName === 'mod.config.json') {
                options.push({
                    label: 'Edit Project Info',
                    icon: getIcon('code'),
                    onClick: () => openModal('modConfig', { filePath: fullPath }),
                });
                options.push({
                    label: 'Add Contributor',
                    icon: getIcon('plus'),
                    onClick: async () => {
                        try {
                            const text = await api.readTextFile(fullPath);
                            const config = JSON.parse(text);
                            const name = prompt('Contributor name:');
                            if (!name?.trim()) return;
                            const role = prompt('Role (optional):');
                            const author = role?.trim()
                                ? { NameAndRole: { name: name.trim(), role: role.trim() } }
                                : { Name: name.trim() };
                            config.authors = [...(config.authors || []), author];
                            await api.writeTextFile(fullPath, JSON.stringify(config, null, 2));
                            showToast('success', `Added contributor: ${name.trim()}`);
                        } catch (err) {
                            showToast('error', 'Failed to add contributor');
                        }
                    },
                    separator: true,
                });
            }

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

            // Split BIN option — only on Skin{N}.bin and similar (.bin files
            // not matching one of the auto-generated names). The modal will
            // show its class breakdown so the user can pick what to extract.
            if (ext === 'bin' && !fileName.toLowerCase().includes('__concat')) {
                const stem = fileName.slice(0, -4); // drop ".bin"
                const defaultOutputName = `${stem}_VFX.bin`;
                options.push({
                    label: 'Split BIN by Class…',
                    icon: getIcon('code'),
                    separator: true,
                    onClick: () => openModal('binSplit', {
                        binPath: fullPath.replace(/\//g, '\\'),
                        defaultOutputName,
                    }),
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
                        await api.openWithDefaultApp(normalizedPath);
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

    const statusClass = fileStatus ? `file-tree__item--${fileStatus}` : '';

    return (
        <div
            className={`file-tree__node${isDropTarget ? ' file-tree__node--drop-target' : ''}`}
            data-drop-path={effectiveNode.isDirectory ? effectiveNode.path : undefined}
        >
            <div
                className={`file-tree__item ${isSelected ? 'file-tree__item--selected' : ''} ${statusClass}${isDropTarget ? ' file-tree__item--drop-target' : ''}`}
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
                    <>
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
                        {fileStatus && (
                            <span className={`file-tree__status-badge file-tree__status-badge--${fileStatus}`}>
                                {fileStatus === 'new' ? 'N' : 'M'}
                            </span>
                        )}
                    </>
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
                            projectPath={projectPath}
                            dragProps={dragProps}
                        />
                    ))}
                </div>
            )}
        </div>
    );
});

const ProjectsPanel: React.FC = () => {
    const openModal = useModalStore((s) => s.openModal);
    const recentProjects = useConfigStore((s) => s.recentProjects);
    const { setWorking, setReady, setError } = useAppMetadataStore();

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

            // Add tab + switch to preview
            useProjectTabStore.getState().addTab(project, normalizedPath);
            useNavigationStore.getState().setView('preview');

            // Auto-save to saved projects list
            useConfigStore.getState().addSavedProject({
                id: `proj-${Date.now()}`,
                name: project.display_name || project.name,
                champion: project.champion,
                path: normalizedPath,
                lastOpened: new Date().toISOString(),
            });

            const files = await api.listProjectFiles(normalizedPath);
            const tabId = useProjectTabStore.getState().activeTabId;
            if (tabId) useProjectTabStore.getState().setFileTree(tabId, files);

            // Clear file statuses when opening a new project
            useAppMetadataStore.getState().clearFileStatuses();

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
                {recentProjects.length === 0 ? (
                    <div className="projects-panel__empty">
                        <p>No recent projects</p>
                        <p style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                            Create a new project to get started
                        </p>
                    </div>
                ) : (
                    recentProjects.map((project) => (
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
