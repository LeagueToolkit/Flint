/**
 * Flint - Folder Grid View
 *
 * The "custom file explorer". When a folder is selected in the project
 * tree (left panel), this is what renders in the right-hand preview
 * panel: a grid of cards for the immediate children of that folder.
 *
 * - Texture children (.dds / .tex / .png / .jpg) get a small thumbnail
 *   decoded once and cached in the existing image cache.
 * - Other files show their type icon.
 * - Single click on a child = select it (drives the rest of the preview
 *   pipeline). Folders navigate "into" by updating the active tab's
 *   selectedFile to the child's relative path.
 * - Double click on a texture opens the full-resolution image modal.
 */

import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useProjectTabStore, useAppMetadataStore, useModalStore } from '../../lib/stores';
import * as api from '../../lib/api';
import { getFileIcon } from '../../lib/fileIcons';
import { getCachedImage, cacheImage } from '../../lib/imageCache';

interface FolderGridViewProps {
    /** Absolute path of the folder being shown. */
    folderAbsPath: string;
    /** Project root (absolute) used to compute relative paths for the VFS. */
    projectPath: string;
    /** Project-relative form of `folderAbsPath` — what's stored in the
     *  active tab's `selectedFile` field. Used to compute the parent
     *  folder for the "up" button. */
    folderRelPath: string;
}

const TEXTURE_EXTS = new Set(['dds', 'tex', 'png', 'jpg', 'jpeg', 'webp']);

const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

export const FolderGridView: React.FC<FolderGridViewProps> = ({
    folderAbsPath,
    projectPath,
    folderRelPath,
}) => {
    const [entries, setEntries] = useState<api.FolderEntry[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const activeTabId = useProjectTabStore((s) => s.activeTabId);
    const setSelectedFile = useProjectTabStore((s) => s.setSelectedFile);
    const fileTreeVersion = useAppMetadataStore((s) => s.fileTreeVersion);
    const openModal = useModalStore((s) => s.openModal);

    // Refetch when the folder changes or the watcher signals a tree-level
    // change (file added / removed under this directory).
    useEffect(() => {
        let cancelled = false;
        setLoading(true);
        setError(null);
        api.listFolderContents(projectPath, folderAbsPath)
            .then((res) => {
                if (cancelled) return;
                setEntries(res);
            })
            .catch((e) => {
                if (cancelled) return;
                setError((e as { message?: string })?.message ?? String(e));
            })
            .finally(() => {
                if (!cancelled) setLoading(false);
            });
        return () => {
            cancelled = true;
        };
    }, [folderAbsPath, projectPath, fileTreeVersion]);

    const goTo = (relPath: string) => {
        if (!activeTabId) return;
        setSelectedFile(activeTabId, relPath);
    };

    // Parent folder for the "up" button. Empty string = no parent.
    const parentRel = useMemo(() => {
        if (!folderRelPath) return null;
        const idx = folderRelPath.replace(/\\/g, '/').lastIndexOf('/');
        if (idx <= 0) return null;
        return folderRelPath.slice(0, idx);
    }, [folderRelPath]);

    return (
        <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
            {/* Breadcrumb / up nav */}
            <div
                style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '8px',
                    padding: '8px 12px',
                    borderBottom: '1px solid var(--border)',
                    flexShrink: 0,
                    fontSize: '12px',
                }}
            >
                {parentRel !== null ? (
                    <button
                        type="button"
                        className="btn btn--ghost btn--small"
                        style={{ padding: '4px 10px' }}
                        onClick={() => goTo(parentRel)}
                        title="Go to parent folder"
                    >
                        ↑ Up
                    </button>
                ) : (
                    <span style={{ color: 'var(--text-muted)', padding: '4px 0' }}>📁 Project root</span>
                )}
                <span
                    style={{
                        flex: 1,
                        color: 'var(--text-muted)',
                        fontFamily: 'monospace',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                    }}
                    title={folderRelPath}
                >
                    {folderRelPath || '(root)'}
                </span>
                <span style={{ color: 'var(--text-muted)', flexShrink: 0 }}>
                    {loading ? 'Loading…' : `${entries.length} item${entries.length === 1 ? '' : 's'}`}
                </span>
            </div>

            {/* Grid */}
            <div style={{ flex: 1, overflow: 'auto', padding: '12px' }}>
                {error && (
                    <div style={{ color: 'var(--accent-danger)', padding: '12px' }}>
                        Error: {error}
                    </div>
                )}
                {!loading && !error && entries.length === 0 && (
                    <div style={{ color: 'var(--text-muted)', padding: '24px', textAlign: 'center' }}>
                        Empty folder
                    </div>
                )}

                <div
                    style={{
                        display: 'grid',
                        gridTemplateColumns: 'repeat(auto-fill, minmax(120px, 1fr))',
                        gap: '12px',
                    }}
                >
                    {entries.map((entry) => {
                        const isTexture = !entry.is_directory && TEXTURE_EXTS.has(entry.extension);
                        return (
                            <FolderGridCard
                                key={entry.absolute_path}
                                entry={entry}
                                onClick={() => goTo(entry.relative_path)}
                                onDoubleClick={
                                    isTexture
                                        ? () => openModal('fullResImage', {
                                            absPath: entry.absolute_path,
                                            fileName: entry.name,
                                        })
                                        : undefined
                                }
                            />
                        );
                    })}
                </div>
            </div>
        </div>
    );
};

interface FolderGridCardProps {
    entry: api.FolderEntry;
    onClick: () => void;
    onDoubleClick?: () => void;
}

const FolderGridCard: React.FC<FolderGridCardProps> = ({ entry, onClick, onDoubleClick }) => {
    const isTexture = !entry.is_directory && TEXTURE_EXTS.has(entry.extension);
    const [thumbnail, setThumbnail] = useState<string | null>(null);
    const cardRef = useRef<HTMLButtonElement>(null);
    const [inView, setInView] = useState(false);

    // Cache hit short-circuits before any observer setup — visible or not,
    // if the data is already there we render it immediately.
    useEffect(() => {
        if (!isTexture) return;
        const cached = getCachedImage(entry.absolute_path);
        if (cached) setThumbnail(cached as string);
    }, [isTexture, entry.absolute_path]);

    // IntersectionObserver — only fire decode IPC for textures actually
    // scrolled into view. Folders with hundreds of textures used to kick
    // off every decode at once on mount; this caps work to what the user
    // is looking at, plus a 200px rootMargin so scrolling stays smooth.
    useEffect(() => {
        if (!isTexture || thumbnail) return;
        const el = cardRef.current;
        if (!el) return;
        const obs = new IntersectionObserver(
            (entries) => {
                for (const e of entries) {
                    if (e.isIntersecting) {
                        setInView(true);
                        obs.disconnect();
                        break;
                    }
                }
            },
            { rootMargin: '200px' },
        );
        obs.observe(el);
        return () => obs.disconnect();
    }, [isTexture, thumbnail]);

    // Decode once the card has been seen. Cache the result so revisits +
    // adjacent components share work.
    useEffect(() => {
        if (!isTexture || !inView || thumbnail) return;
        let cancelled = false;
        (async () => {
            try {
                let url: string;
                if (entry.extension === 'dds' || entry.extension === 'tex') {
                    const decoded = await api.decodeDdsToPng(entry.absolute_path);
                    url = `data:image/png;base64,${decoded.data}`;
                } else {
                    const bytes = await api.readFileBytes(entry.absolute_path);
                    const blob = new Blob([bytes as BlobPart]);
                    url = URL.createObjectURL(blob);
                }
                if (cancelled) return;
                cacheImage(entry.absolute_path, url);
                setThumbnail(url);
            } catch {
                // Decode failure → falls back to icon. No toast — would be
                // noisy in folders with many bad/unsupported textures.
            }
        })();
        return () => { cancelled = true; };
    }, [isTexture, inView, thumbnail, entry.absolute_path, entry.extension]);

    return (
        <button
            ref={cardRef}
            type="button"
            onClick={onClick}
            onDoubleClick={onDoubleClick}
            title={entry.name}
            style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: '6px',
                padding: '8px',
                background: 'var(--bg-secondary)',
                border: '1px solid var(--border)',
                borderRadius: '6px',
                cursor: 'pointer',
                color: 'var(--text-primary)',
                textAlign: 'center',
                overflow: 'hidden',
            }}
        >
            <div
                style={{
                    width: '100%',
                    aspectRatio: '1 / 1',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    background: 'var(--bg-tertiary, rgba(0,0,0,0.2))',
                    borderRadius: '4px',
                    overflow: 'hidden',
                }}
            >
                {thumbnail ? (
                    <img
                        src={thumbnail}
                        alt={entry.name}
                        style={{
                            width: '100%',
                            height: '100%',
                            objectFit: 'contain',
                            imageRendering: 'pixelated',
                        }}
                    />
                ) : (
                    <span
                        style={{ width: 40, height: 40 }}
                        dangerouslySetInnerHTML={{
                            __html: getFileIcon(entry.name, entry.is_directory, false),
                        }}
                    />
                )}
            </div>
            <span
                style={{
                    fontSize: '11px',
                    width: '100%',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                }}
            >
                {entry.name}
            </span>
            <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>
                {entry.is_directory ? 'folder' : formatBytes(entry.size)}
            </span>
        </button>
    );
};
