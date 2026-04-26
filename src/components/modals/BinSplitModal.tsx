/**
 * Flint - BIN Split Modal
 *
 * Right-click → "Split VFX to separate BIN" on a Skin{N}.bin opens this.
 * Shows the BIN's class-grouped object list with checkboxes; default-checks
 * the groups our classifier flagged as VFX. On confirm, calls the Rust
 * `split_bin_entries` command which writes a new sibling BIN, removes the
 * moved objects from the parent, and appends a link to the parent's
 * dependency list.
 *
 * The wire format we emit is byte-compatible with Quartz's combine action,
 * which is the inverse operation. See `flint-ltk/src/bin/split.rs` for the
 * details.
 */

import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { useAppState } from '../../lib/stores';
import * as api from '../../lib/api';

interface BinSplitOptions {
    /** Absolute path to the parent BIN file. */
    binPath: string;
    /**
     * Suggested filename for the new sibling BIN. Computed by the caller
     * (FileTree context menu) from the parent's stem, e.g.
     * `Skin19.bin` → `Skin19_VFX.bin`.
     */
    defaultOutputName: string;
}

export const BinSplitModal: React.FC = () => {
    const { state, closeModal, showToast } = useAppState();
    const isVisible = state.activeModal === 'binSplit';
    const options = state.modalOptions as BinSplitOptions | null;

    const [analysis, setAnalysis] = useState<api.BinSplitAnalysis | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [outputName, setOutputName] = useState('');
    /** Set of class hashes whose group is checked for moving. */
    const [checkedClasses, setCheckedClasses] = useState<Set<string>>(new Set());
    const [busy, setBusy] = useState(false);

    // Load analysis when modal opens
    useEffect(() => {
        if (!isVisible || !options?.binPath) {
            setAnalysis(null);
            setError(null);
            setCheckedClasses(new Set());
            return;
        }

        setOutputName(options.defaultOutputName);
        setLoading(true);
        setError(null);
        setAnalysis(null);

        let cancelled = false;
        api.analyzeBinForSplit(options.binPath)
            .then((res) => {
                if (cancelled) return;
                setAnalysis(res);
                // Default-check every group that our Rust classifier flagged
                // as VFX. The user can toggle from there.
                const initial = new Set<string>();
                for (const g of res.groups) {
                    if (g.is_vfx_default) initial.add(g.class_hash);
                }
                setCheckedClasses(initial);
            })
            .catch((e) => {
                if (cancelled) return;
                setError((e as Error).message ?? String(e));
            })
            .finally(() => {
                if (!cancelled) setLoading(false);
            });

        return () => {
            cancelled = true;
        };
    }, [isVisible, options?.binPath, options?.defaultOutputName]);

    const moveCount = useMemo(() => {
        if (!analysis) return 0;
        let n = 0;
        for (const g of analysis.groups) {
            if (checkedClasses.has(g.class_hash)) n += g.path_hashes.length;
        }
        return n;
    }, [analysis, checkedClasses]);

    const toggleClass = useCallback((classHash: string) => {
        setCheckedClasses((prev) => {
            const next = new Set(prev);
            if (next.has(classHash)) next.delete(classHash);
            else next.add(classHash);
            return next;
        });
    }, []);

    const selectAll = useCallback(() => {
        if (!analysis) return;
        setCheckedClasses(new Set(analysis.groups.map((g) => g.class_hash)));
    }, [analysis]);

    const selectNone = useCallback(() => {
        setCheckedClasses(new Set());
    }, []);

    const selectVfxDefault = useCallback(() => {
        if (!analysis) return;
        setCheckedClasses(
            new Set(analysis.groups.filter((g) => g.is_vfx_default).map((g) => g.class_hash)),
        );
    }, [analysis]);

    const handleSplit = useCallback(async () => {
        if (!analysis || !options?.binPath || moveCount === 0 || busy) return;

        const trimmed = outputName.trim();
        if (!trimmed) {
            showToast('error', 'Output filename is required');
            return;
        }
        if (!trimmed.toLowerCase().endsWith('.bin')) {
            showToast('error', 'Output filename must end with .bin');
            return;
        }
        if (trimmed.includes('/') || trimmed.includes('\\')) {
            showToast('error', 'Output filename cannot contain path separators');
            return;
        }

        // Collect every path_hash whose class group is checked.
        const hashes: string[] = [];
        for (const g of analysis.groups) {
            if (checkedClasses.has(g.class_hash)) hashes.push(...g.path_hashes);
        }

        setBusy(true);
        try {
            const result = await api.splitBinEntries(options.binPath, trimmed, hashes);
            showToast(
                'success',
                `Moved ${result.moved} object${result.moved === 1 ? '' : 's'} to ${trimmed}`,
            );
            closeModal();
        } catch (e) {
            const msg = (e as { message?: string })?.message ?? String(e);
            showToast('error', `Split failed: ${msg}`);
        } finally {
            setBusy(false);
        }
    }, [analysis, options, outputName, checkedClasses, moveCount, busy, showToast, closeModal]);

    if (!isVisible) return null;

    const totalRemainingInParent =
        (analysis?.total_objects ?? 0) - moveCount;

    return (
        <div className="modal-overlay" onClick={busy ? undefined : closeModal}>
            <div
                className="modal modal--large"
                onClick={(e) => e.stopPropagation()}
                style={{ width: '720px', maxWidth: '95vw', maxHeight: '85vh', display: 'flex', flexDirection: 'column' }}
            >
                <div className="modal__header">
                    <h2 className="modal__title">Split BIN by Class</h2>
                    <button className="modal__close" onClick={closeModal} disabled={busy}>×</button>
                </div>

                <div className="modal__body" style={{ overflow: 'auto', flex: 1 }}>
                    {loading && <div className="modal__empty">Reading BIN…</div>}
                    {error && <div className="modal__empty" style={{ color: 'var(--accent-danger)' }}>Error: {error}</div>}

                    {analysis && !loading && !error && (
                        <>
                            <div style={{ marginBottom: '16px', fontSize: '13px', color: 'var(--text-muted)' }}>
                                {analysis.total_objects} objects total · {moveCount} selected to move ·{' '}
                                {totalRemainingInParent} will stay in parent
                            </div>

                            <div style={{ display: 'flex', gap: '8px', marginBottom: '12px' }}>
                                <button className="btn btn--secondary btn--small" onClick={selectVfxDefault}>VFX preset</button>
                                <button className="btn btn--ghost btn--small" onClick={selectAll}>All</button>
                                <button className="btn btn--ghost btn--small" onClick={selectNone}>None</button>
                            </div>

                            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                                {analysis.groups.map((g) => {
                                    const checked = checkedClasses.has(g.class_hash);
                                    const label = g.class_name ?? `0x${g.class_hash}`;
                                    return (
                                        <label
                                            key={g.class_hash}
                                            style={{
                                                display: 'flex',
                                                alignItems: 'center',
                                                gap: '10px',
                                                padding: '6px 10px',
                                                borderRadius: '4px',
                                                cursor: 'pointer',
                                                background: checked ? 'rgba(79,195,247,0.08)' : 'transparent',
                                            }}
                                        >
                                            <input
                                                type="checkbox"
                                                checked={checked}
                                                onChange={() => toggleClass(g.class_hash)}
                                                disabled={busy}
                                            />
                                            <span style={{ fontSize: '13px', flex: 1, fontFamily: g.class_name ? 'inherit' : 'monospace' }}>
                                                {label}
                                                {g.is_vfx_default && (
                                                    <span style={{ marginLeft: '8px', fontSize: '10px', color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.06em' }}>
                                                        VFX
                                                    </span>
                                                )}
                                            </span>
                                            <span style={{ fontSize: '11px', color: 'var(--text-muted)', minWidth: '48px', textAlign: 'right' }}>
                                                ×{g.path_hashes.length}
                                            </span>
                                        </label>
                                    );
                                })}
                            </div>
                        </>
                    )}
                </div>

                <div className="modal__footer" style={{ flexDirection: 'column', alignItems: 'stretch', gap: '12px' }}>
                    <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                        <label style={{ fontSize: '12px', color: 'var(--text-muted)', flexShrink: 0 }}>
                            Output file:
                        </label>
                        <input
                            type="text"
                            value={outputName}
                            onChange={(e) => setOutputName(e.target.value)}
                            disabled={busy}
                            style={{ flex: 1, padding: '6px 10px', fontSize: '13px', fontFamily: 'monospace' }}
                        />
                    </div>
                    <div style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end' }}>
                        <button className="btn btn--ghost" onClick={closeModal} disabled={busy}>
                            Cancel
                        </button>
                        <button
                            className="btn btn--primary"
                            onClick={handleSplit}
                            disabled={busy || moveCount === 0 || !analysis}
                        >
                            {busy ? 'Splitting…' : `Split ${moveCount} object${moveCount === 1 ? '' : 's'}`}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
};
