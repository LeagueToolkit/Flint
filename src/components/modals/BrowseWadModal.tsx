/**
 * Flint - Browse WAD Modal
 *
 * Single full-width drop zone (also clickable to open the file picker) for
 * choosing a .wad / .wad.client. After the file is loaded we read the chunk
 * list, count chunks with no resolved path (unknown hashes), and — if more
 * than 3 — surface a callout offering to scan the WAD's BIN/SKN chunks and
 * write `hashes.extracted.txt` / `hashes.binhashes.extracted.txt` into the
 * user's hash directory. Algorithm ported from Quartz's `bin_hashes.rs`.
 */

import React, { useEffect, useRef, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useAppState } from '../../lib/stores';
import * as api from '../../lib/api';
import { Button, Icon, Modal, ModalBody, ModalFooter, ModalHeader } from '../ui';

const UNKNOWN_THRESHOLD = 3;

type Phase = 'pick' | 'loading' | 'loaded' | 'error';

export const BrowseWadModal: React.FC = () => {
    const { state, dispatch, closeModal, showToast } = useAppState();
    const isVisible = state.activeModal === 'browseWad';

    const [phase, setPhase] = useState<Phase>('pick');
    const [wadPath, setWadPath] = useState<string | null>(null);
    const [chunks, setChunks] = useState<Array<{ hash: string; path: string | null; size: number }> | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [extracting, setExtracting] = useState(false);
    const [extractResult, setExtractResult] = useState<api.ExtractHashesResult | null>(null);
    const [dragOver, setDragOver] = useState(false);
    const dragDepth = useRef(0);

    useEffect(() => {
        if (!isVisible) return;
        setPhase('pick');
        setWadPath(null);
        setChunks(null);
        setError(null);
        setExtracting(false);
        setExtractResult(null);
        setDragOver(false);
        dragDepth.current = 0;
    }, [isVisible]);

    const loadWad = async (path: string) => {
        if (!/\.(wad|wad\.client)$/i.test(path) && !path.toLowerCase().endsWith('.client')) {
            setError(`Not a WAD file:\n${path}`);
            setPhase('error');
            return;
        }
        setWadPath(path);
        setPhase('loading');
        setError(null);
        try {
            const list = await api.getWadChunks(path);
            setChunks(list);
            setPhase('loaded');
        } catch (err) {
            const msg = err instanceof api.FlintError ? err.getUserMessage() : String(err);
            setError(msg);
            setPhase('error');
        }
    };

    const handleBrowse = async () => {
        try {
            const selected = await open({
                title: 'Open WAD File',
                filters: [{ name: 'WAD Archive', extensions: ['wad', 'client'] }],
                multiple: false,
            });
            if (!selected) return;
            await loadWad(selected as string);
        } catch (err) {
            console.error('[BrowseWad] picker failed', err);
        }
    };

    const onDragEnter = (e: React.DragEvent) => {
        e.preventDefault();
        dragDepth.current += 1;
        setDragOver(true);
    };
    const onDragLeave = (e: React.DragEvent) => {
        e.preventDefault();
        dragDepth.current = Math.max(0, dragDepth.current - 1);
        if (dragDepth.current === 0) setDragOver(false);
    };
    const onDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = 'copy';
    };
    const onDrop = async (e: React.DragEvent) => {
        e.preventDefault();
        dragDepth.current = 0;
        setDragOver(false);

        const dropped = Array.from(e.dataTransfer.files);
        if (dropped.length === 0) {
            showToast('error', 'No file in the drop');
            return;
        }
        const f = dropped[0] as File & { path?: string };
        const path = f.path ?? f.name;
        if (!path || path === f.name) {
            showToast(
                'error',
                'Drag-and-drop needs absolute paths. Use Browse instead, or drop from File Explorer (not the browser).',
            );
            return;
        }
        await loadWad(path);
    };

    const totalChunks = chunks?.length ?? 0;
    const unknownChunks = chunks?.filter((c) => !c.path).length ?? 0;
    const tooManyUnknown = unknownChunks > UNKNOWN_THRESHOLD;

    const handleExtractHashes = async () => {
        if (!wadPath) return;
        setExtracting(true);
        setExtractResult(null);
        try {
            const result = await api.extractHashesFromWad(wadPath);
            setExtractResult(result);
            const total = result.game_hashes_added + result.bin_hashes_added;
            showToast(
                total > 0 ? 'success' : 'info',
                total > 0
                    ? `Extracted ${total} new hash${total === 1 ? '' : 'es'} from ${result.scanned} files`
                    : `Scanned ${result.scanned} BIN/SKN files — no new hashes found`,
            );
        } catch (err) {
            const msg = err instanceof api.FlintError ? err.getUserMessage() : String(err);
            showToast('error', `Extract failed: ${msg}`);
        } finally {
            setExtracting(false);
        }
    };

    const handleOpenInExplorer = () => {
        if (!wadPath || !chunks) return;
        const sessionId = `extract-${Date.now()}`;
        dispatch({ type: 'OPEN_EXTRACT_SESSION', payload: { id: sessionId, wadPath } });
        dispatch({ type: 'SET_EXTRACT_CHUNKS', payload: { sessionId, chunks } });
        closeModal();
    };

    if (!isVisible) return null;

    /* Picker phase has its own headless layout (heading + subtitle inside body
       to match the reference); other phases keep a compact ModalHeader. */
    const showHeader = phase !== 'pick';

    return (
        <Modal open={isVisible} onClose={closeModal} size="wide" modifier="modal--browse-wad">
            {showHeader && (
                <ModalHeader
                    title={
                        <span className="bw-title">
                            <span className="bw-title__icon"><Icon name="wad" /></span>
                            <span className="bw-title__text">
                                <span className="bw-title__name">Open WAD File</span>
                                <span className="bw-title__sub">
                                    {phase === 'loading' && 'Reading chunks…'}
                                    {phase === 'loaded' && `${totalChunks.toLocaleString()} chunks · ${unknownChunks.toLocaleString()} unresolved`}
                                    {phase === 'error' && 'Could not open this WAD'}
                                </span>
                            </span>
                        </span>
                    }
                    onClose={closeModal}
                />
            )}

            <ModalBody className="bw-body">
                {phase === 'pick' && (
                    <div className="bw-pick">
                        {/* Inline X close (since we hid the header for this phase) */}
                        <button
                            type="button"
                            className="modal__close bw-pick__close"
                            onClick={closeModal}
                            aria-label="Close"
                        >
                            <Icon name="close" />
                        </button>

                        <h2 className="bw-pick__title">Open Your WAD File</h2>
                        <p className="bw-pick__sub">
                            Load a League of Legends WAD archive (<code>.wad</code> or
                            {' '}<code>.wad.client</code>) to scan its contents and detect
                            missing path hashes.
                        </p>

                        <div
                            role="button"
                            tabIndex={0}
                            className={`bw-drop ${dragOver ? 'bw-drop--over' : ''}`}
                            onDragEnter={onDragEnter}
                            onDragLeave={onDragLeave}
                            onDragOver={onDragOver}
                            onDrop={onDrop}
                            onClick={handleBrowse}
                            onKeyDown={(e) => {
                                if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleBrowse(); }
                            }}
                            aria-label="Drop a WAD file here, or click to browse"
                        >
                            <div className="bw-drop__inner">
                                <span className="bw-drop__icon" aria-hidden="true">
                                    <svg
                                        width="36"
                                        height="36"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        strokeWidth="2"
                                        strokeLinecap="round"
                                        strokeLinejoin="round"
                                    >
                                        <path d="M12 4v12" />
                                        <path d="M6 10l6-6 6 6" />
                                        <path d="M5 19h14" />
                                    </svg>
                                </span>
                                <strong className="bw-drop__title">
                                    {dragOver ? 'Release to load' : 'Drag & drop your WAD file'}
                                </strong>
                                <span className="bw-drop__desc">
                                    or{' '}
                                    <span className="bw-drop__link">browse to upload</span>
                                </span>
                            </div>
                        </div>

                        <button
                            type="button"
                            className="bw-manual-btn"
                            onClick={handleBrowse}
                        >
                            <Icon name="folder" />
                            <span>Select manually from disk</span>
                        </button>
                    </div>
                )}

                {phase === 'loading' && (
                    <div className="bw-state">
                        <div className="bw-state__spinner" />
                        <strong className="bw-state__title">Reading chunks…</strong>
                        <span className="bw-state__sub">{wadPath}</span>
                    </div>
                )}

                {phase === 'error' && (
                    <div className="bw-state bw-state--error">
                        <span className="bw-state__icon"><Icon name="error" /></span>
                        <strong className="bw-state__title">Failed to open WAD</strong>
                        <span className="bw-state__sub">{error}</span>
                        <Button onClick={() => setPhase('pick')}>Try another file</Button>
                    </div>
                )}

                {phase === 'loaded' && wadPath && chunks && (
                    <div className="bw-summary">
                        <div className="bw-file">
                            <span className="bw-file__icon"><Icon name="wad" /></span>
                            <span className="bw-file__body">
                                <span className="bw-file__name">{wadPath.split(/[\\/]/).pop()}</span>
                                <span className="bw-file__path">{wadPath}</span>
                            </span>
                            <Button variant="ghost" size="sm" onClick={() => setPhase('pick')} icon="refresh">
                                Change
                            </Button>
                        </div>

                        <div className="bw-stats">
                            <div className="bw-stat">
                                <span className="bw-stat__label">Total chunks</span>
                                <span className="bw-stat__value">{totalChunks.toLocaleString()}</span>
                            </div>
                            <div className="bw-stat bw-stat--ok">
                                <span className="bw-stat__label">Resolved</span>
                                <span className="bw-stat__value">{(totalChunks - unknownChunks).toLocaleString()}</span>
                            </div>
                            <div className={`bw-stat ${unknownChunks > 0 ? 'bw-stat--warn' : ''}`}>
                                <span className="bw-stat__label">Unknown</span>
                                <span className="bw-stat__value">{unknownChunks.toLocaleString()}</span>
                            </div>
                        </div>

                        {tooManyUnknown && !extractResult && (
                            <div className="bw-callout">
                                <span className="bw-callout__icon"><Icon name="warning" /></span>
                                <div className="bw-callout__body">
                                    <strong>Unknown hashes found</strong>
                                    <p>
                                        {unknownChunks.toLocaleString()} chunks couldn't be resolved.
                                        Want to scan this WAD's BIN/SKN files for asset paths and
                                        write a <code>hashes.extracted.txt</code> you can keep?
                                    </p>
                                </div>
                                <Button
                                    variant="primary"
                                    icon="download"
                                    onClick={handleExtractHashes}
                                    disabled={extracting}
                                >
                                    {extracting ? 'Extracting…' : 'Extract hashes'}
                                </Button>
                            </div>
                        )}

                        {extractResult && (
                            <div className="bw-callout bw-callout--ok">
                                <span className="bw-callout__icon"><Icon name="success" /></span>
                                <div className="bw-callout__body">
                                    <strong>
                                        {extractResult.game_hashes_added + extractResult.bin_hashes_added > 0
                                            ? `Extracted ${extractResult.game_hashes_added + extractResult.bin_hashes_added} new hashes`
                                            : 'No new hashes found'}
                                    </strong>
                                    <p>
                                        Scanned <strong>{extractResult.scanned.toLocaleString()}</strong> BIN/SKN files.
                                        {extractResult.game_hashes_added > 0 && <> {extractResult.game_hashes_added} game </>}
                                        {extractResult.game_hashes_added > 0 && extractResult.bin_hashes_added > 0 && '· '}
                                        {extractResult.bin_hashes_added > 0 && <>{extractResult.bin_hashes_added} bin</>}
                                    </p>
                                    {extractResult.output_files.length > 0 && (
                                        <ul className="bw-callout__files">
                                            {extractResult.output_files.map((f) => (
                                                <li key={f}><code>{f}</code></li>
                                            ))}
                                        </ul>
                                    )}
                                </div>
                            </div>
                        )}
                    </div>
                )}
            </ModalBody>

            <ModalFooter>
                <Button variant="ghost" onClick={closeModal}>Cancel</Button>
                <Button
                    variant="success"
                    icon="success"
                    onClick={handleOpenInExplorer}
                    disabled={phase !== 'loaded'}
                >
                    Open in WAD Explorer
                </Button>
            </ModalFooter>
        </Modal>
    );
};
