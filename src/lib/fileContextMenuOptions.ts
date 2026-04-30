/**
 * Shared right-click menu builder used by both FileTree and FolderGridView.
 * Keeps the two surfaces in sync — anything you wire up here shows up in
 * both places.
 */

import * as api from './api';
import { getIcon } from './fileIcons';
import type { ContextMenuOption, ModalType } from './types';

interface BuildOptionsArgs {
    /** The node being right-clicked. */
    node: { path: string; name: string; isDirectory: boolean };
    /** Project root absolute path. */
    projectPath: string;
    /** Tree depth — used to show root-only options like "Set Thumbnail". */
    depth: number;
    /** Refresh the project file tree after a mutation. */
    refreshFileTree: () => Promise<void>;
    /** Open the named modal with the given options. */
    openModal: (modal: ModalType, options?: Record<string, unknown>) => void;
    /** Open the confirmation dialog (Delete, Organize, etc). */
    openConfirmDialog: (dialog: {
        title: string;
        message: string;
        confirmLabel?: string;
        danger?: boolean;
        onConfirm: () => void;
    }) => void;
    /** Toast notifications. */
    showToast: (type: 'info' | 'success' | 'warning' | 'error', message: string) => void;
    /** Optional rename trigger — when present, the menu shows "Rename".
     *  FileTree passes its inline-rename setter; grid views can omit it. */
    onRename?: (path: string) => void;
}

function isContentFolder(path: string): boolean {
    const normalized = path.replace(/\\/g, '/');
    return normalized === 'content' || normalized.endsWith('/content');
}

export function buildFileContextMenuOptions(args: BuildOptionsArgs): ContextMenuOption[] {
    const { node, projectPath, depth, refreshFileTree, openModal, openConfirmDialog, showToast, onRename } = args;
    const options: ContextMenuOption[] = [];

    const fullPath = projectPath
        ? `${projectPath.replace(/\\/g, '/')}/${node.path}`
        : node.path;
    const fileName = node.name;
    const ext = fileName.includes('.') ? fileName.split('.').pop()?.toLowerCase() ?? '' : '';

    if (node.isDirectory) {
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

        if (isContentFolder(node.path)) {
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
                onClick: () => openModal('recolor', { filePath: node.path, isFolder: true }),
            });
        } else {
            options.push({
                label: 'Batch Recolor',
                icon: getIcon('texture'),
                onClick: () => openModal('recolor', { filePath: node.path, isFolder: true }),
            });
        }

        options.push({
            label: 'New Folder',
            icon: getIcon('folder'),
            separator: true,
            onClick: async () => {
                const newDir = `${node.path}/New Folder`;
                try {
                    await api.createDirectory(projectPath, newDir);
                    await refreshFileTree();
                } catch (err) {
                    const flintError = err as api.FlintError;
                    showToast('error', flintError.getUserMessage?.() || 'Failed to create folder');
                }
            },
        });

        if (onRename) {
            options.push({
                label: 'Rename',
                icon: getIcon('text'),
                onClick: () => onRename(node.path),
            });
        }

        if (fileName.toLowerCase() === 'data') {
            options.push({
                label: 'Split BINs by Class…',
                icon: getIcon('code'),
                separator: true,
                onClick: () => openModal('binSplit', {
                    mode: 'folder',
                    folderPath: fullPath.replace(/\//g, '\\'),
                    defaultOutputName: 'VFX.bin',
                }),
            });

            options.push({
                label: 'Organize VFX (auto-consolidate)…',
                icon: getIcon('texture'),
                onClick: async () => {
                    const folderAbs = fullPath.replace(/\//g, '\\');
                    try {
                        const preview = await api.previewOrganizeVfx(folderAbs);
                        const ownerRel = preview.suggested_owner
                            ? preview.suggested_owner.split(/[\\/]/).slice(-3).join('/')
                            : '(none — cannot run)';
                        const deletedEstimate = preview.sources.length > 1
                            ? `up to ${preview.sources.length - 1} non-owner BIN${preview.sources.length - 1 === 1 ? '' : 's'} may be removed`
                            : 'no other BINs to merge';

                        openConfirmDialog({
                            title: 'Organize VFX',
                            message:
                                `Pull ${preview.vfx_objects_estimate} VFX object${preview.vfx_objects_estimate === 1 ? '' : 's'} into ` +
                                `data/${preview.vfx_filename} and merge ${preview.main_objects_estimate} non-VFX object${preview.main_objects_estimate === 1 ? '' : 's'} ` +
                                `into the main BIN (${ownerRel}). ${deletedEstimate}. Continue?`,
                            confirmLabel: 'Organize',
                            onConfirm: async () => {
                                if (!preview.suggested_owner) {
                                    showToast('error', 'No main skin BIN found in this folder — cannot organize');
                                    return;
                                }
                                try {
                                    const result = await api.organizeBinsVfx(
                                        folderAbs,
                                        preview.suggested_owner,
                                        preview.vfx_filename,
                                    );
                                    const msg =
                                        `${result.vfx_objects_moved} VFX → ${preview.vfx_filename}, ` +
                                        `${result.main_objects_merged} merged into main, ` +
                                        `${result.sources_deleted.length} BIN${result.sources_deleted.length === 1 ? '' : 's'} removed`;
                                    showToast('success', msg);
                                    await refreshFileTree();
                                } catch (e) {
                                    const m = (e as { message?: string })?.message ?? String(e);
                                    showToast('error', `Organize failed: ${m}`);
                                }
                            },
                        });
                    } catch (e) {
                        const m = (e as { message?: string })?.message ?? String(e);
                        showToast('error', `Preview failed: ${m}`);
                    }
                },
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
            onClick: () => navigator.clipboard.writeText(node.path),
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
                            await api.deleteFile(projectPath, node.path);
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
        return options;
    }

    // ── File ──────────────────────────────────────────────────────────────

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
                } catch {
                    showToast('error', 'Failed to add contributor');
                }
            },
            separator: true,
        });
    }

    if (onRename) {
        options.push({
            label: 'Rename',
            icon: getIcon('text'),
            onClick: () => onRename(node.path),
        });
    }

    options.push({
        label: 'Duplicate',
        icon: getIcon('file'),
        onClick: async () => {
            try {
                await api.duplicateFile(projectPath, node.path);
                await refreshFileTree();
                showToast('success', 'File duplicated');
            } catch (err) {
                const flintError = err as api.FlintError;
                showToast('error', flintError.getUserMessage?.() || 'Failed to duplicate');
            }
        },
    });

    if (ext === 'dds' || ext === 'tex') {
        options.push({
            label: 'Recolor',
            icon: getIcon('texture'),
            separator: true,
            onClick: () => openModal('recolor', { filePath: node.path, isFolder: false }),
        });
    }

    if (ext === 'bin' && !fileName.toLowerCase().includes('__concat')) {
        const stem = fileName.slice(0, -4);
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
        onClick: () => navigator.clipboard.writeText(node.path),
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
                const normalizedPath = fullPath.replace(/\//g, '\\');
                await api.openWithDefaultApp(normalizedPath);
            } catch (err) {
                const message = (err as Error).message || String(err);
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
                        await api.deleteFile(projectPath, node.path);
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

    return options;
}
