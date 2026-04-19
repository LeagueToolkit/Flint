/**
 * Flint — Audio Cutter Modal
 *
 * Real audio cutter with:
 *  - Draggable red bars on the waveform (direct manipulation, no sliders)
 *  - Playback cursor that animates during playback
 *  - Zoom (Ctrl+wheel or +/- buttons) with cursor-anchored zoom
 *  - Horizontal pan (wheel / drag empty space / scrollbar)
 *  - Time ruler
 *  - Play / Pause / Play selection
 *  - Apply trim: replaces the WEM with the selected range as PCM WAV
 */

import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import * as api from '../../lib/api';
import type { AudioEntryInfo } from '../../lib/types';
import {
    audioBufferToWav,
    decodeWemToBuffer,
    sliceAudioBuffer,
    formatTime,
} from './audioUtils';

interface AudioCutterModalProps {
    entry: AudioEntryInfo;
    filePath: string;
    bankBytes: Uint8Array | null;
    onClose: () => void;
    onApply: (newWav: Uint8Array) => Promise<void>;
}

type DragKind = null | 'start' | 'end' | 'selection' | 'new' | 'pan';

interface DragState {
    kind: Exclude<DragKind, null>;
    startClientX: number;
    startTime: number;
    origSel: { start: number; end: number };
    origScrollX: number;
}

const HANDLE_HIT_PX = 8;
const RULER_HEIGHT = 22;
const MIN_ZOOM = 1;
const MAX_ZOOM = 200;

export const AudioCutterModal: React.FC<AudioCutterModalProps> = ({
    entry,
    filePath,
    bankBytes,
    onClose,
    onApply,
}) => {
    // -----------------------------------------------------------------------
    // Audio state
    // -----------------------------------------------------------------------
    const [buffer, setBuffer] = useState<AudioBuffer | null>(null);
    const [loadError, setLoadError] = useState<string | null>(null);
    const [selection, setSelection] = useState<{ start: number; end: number }>({ start: 0, end: 0 });
    const [playhead, setPlayhead] = useState(0);
    const [isPlaying, setIsPlaying] = useState(false);
    const [applying, setApplying] = useState(false);

    // -----------------------------------------------------------------------
    // View state
    // -----------------------------------------------------------------------
    const [zoom, setZoom] = useState(1);         // 1 = full fit
    const [scrollT, setScrollT] = useState(0);   // time at left edge, seconds
    const [canvasSize, setCanvasSize] = useState({ w: 800, h: 180 });
    const [cursorStyle, setCursorStyle] = useState<string>('default');

    // -----------------------------------------------------------------------
    // Refs
    // -----------------------------------------------------------------------
    const containerRef = useRef<HTMLDivElement>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const rulerCanvasRef = useRef<HTMLCanvasElement>(null);
    const audioCtxRef = useRef<AudioContext | null>(null);
    const sourceRef = useRef<AudioBufferSourceNode | null>(null);
    const rafRef = useRef<number | null>(null);
    const playTrackerRef = useRef<{ ctxTime: number; offset: number; endAt: number } | null>(null);
    const dragRef = useRef<DragState | null>(null);
    const peaksRef = useRef<{ min: Float32Array; max: Float32Array; pixels: number } | null>(null);

    const duration = buffer?.duration ?? 0;

    // -----------------------------------------------------------------------
    // Derived layout — how one second maps to pixels, etc.
    // -----------------------------------------------------------------------
    const layout = useMemo(() => {
        const { w } = canvasSize;
        if (duration <= 0) {
            return { viewDuration: 1, pxPerSec: w, scrollT: 0, clampedScrollT: 0 };
        }
        const viewDuration = duration / zoom;
        const pxPerSec = w / viewDuration;
        const maxScroll = Math.max(0, duration - viewDuration);
        const clampedScrollT = Math.min(Math.max(scrollT, 0), maxScroll);
        return { viewDuration, pxPerSec, scrollT, clampedScrollT };
    }, [canvasSize, duration, zoom, scrollT]);

    const timeToX = useCallback(
        (t: number) => (t - layout.clampedScrollT) * layout.pxPerSec,
        [layout],
    );
    const xToTime = useCallback(
        (x: number) => x / layout.pxPerSec + layout.clampedScrollT,
        [layout],
    );

    // -----------------------------------------------------------------------
    // Load + decode
    // -----------------------------------------------------------------------
    useEffect(() => {
        let cancelled = false;
        (async () => {
            try {
                const wemBytes = bankBytes
                    ? new Uint8Array(await api.readAudioEntryBytes(Array.from(bankBytes), entry.id))
                    : new Uint8Array(await api.readAudioEntry(filePath, entry.id));
                const buf = await decodeWemToBuffer(wemBytes);
                if (cancelled) return;
                setBuffer(buf);
                setSelection({ start: 0, end: buf.duration });
            } catch (err) {
                if (!cancelled) setLoadError((err as Error).message || String(err));
            }
        })();
        return () => {
            cancelled = true;
        };
    }, [entry.id, filePath, bankBytes]);

    // Invalidate peak cache when view changes (not currently used — kept for future LOD)
    useEffect(() => {
        peaksRef.current = null;
    }, [buffer, canvasSize.w, zoom, layout.clampedScrollT]);

    // -----------------------------------------------------------------------
    // Observe container size for responsive canvas
    // -----------------------------------------------------------------------
    useEffect(() => {
        const el = containerRef.current;
        if (!el) return;
        const observer = new ResizeObserver(() => {
            const rect = el.getBoundingClientRect();
            setCanvasSize({
                w: Math.max(300, Math.floor(rect.width)),
                h: 180,
            });
        });
        observer.observe(el);
        const rect = el.getBoundingClientRect();
        setCanvasSize({ w: Math.max(300, Math.floor(rect.width)), h: 180 });
        return () => observer.disconnect();
    }, []);

    // -----------------------------------------------------------------------
    // Draw waveform, selection, handles, playhead
    // -----------------------------------------------------------------------
    useEffect(() => {
        if (!buffer) return;
        const canvas = canvasRef.current;
        if (!canvas) return;

        const { w, h } = canvasSize;
        const dpr = window.devicePixelRatio || 1;
        canvas.width = Math.floor(w * dpr);
        canvas.height = Math.floor(h * dpr);
        canvas.style.width = `${w}px`;
        canvas.style.height = `${h}px`;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, w, h);

        // Background
        ctx.fillStyle = 'rgba(255,255,255,0.02)';
        ctx.fillRect(0, 0, w, h);

        // Mid line
        ctx.strokeStyle = 'rgba(255,255,255,0.06)';
        ctx.beginPath();
        ctx.moveTo(0, h / 2);
        ctx.lineTo(w, h / 2);
        ctx.stroke();

        // Accent color from theme
        const accent =
            getComputedStyle(document.documentElement).getPropertyValue('--accent-primary').trim() ||
            '#EF4444';

        // Compute peaks for visible window only
        const visibleStartFrame = Math.max(0, Math.floor(layout.clampedScrollT * buffer.sampleRate));
        const visibleEndFrame = Math.min(
            buffer.length,
            Math.ceil((layout.clampedScrollT + layout.viewDuration) * buffer.sampleRate),
        );
        const visibleFrames = visibleEndFrame - visibleStartFrame;
        if (visibleFrames <= 0) return;

        const ch0 = buffer.getChannelData(0);
        const ch1 = buffer.numberOfChannels > 1 ? buffer.getChannelData(1) : null;
        const samplesPerPixel = Math.max(1, visibleFrames / w);

        ctx.fillStyle = accent;
        ctx.globalAlpha = 0.85;
        for (let x = 0; x < w; x++) {
            const from = visibleStartFrame + Math.floor(x * samplesPerPixel);
            const to = Math.min(
                buffer.length,
                visibleStartFrame + Math.floor((x + 1) * samplesPerPixel),
            );
            let mn = 1;
            let mx = -1;
            for (let i = from; i < to; i++) {
                const v = ch1 ? (ch0[i] + ch1[i]) * 0.5 : ch0[i];
                if (v < mn) mn = v;
                if (v > mx) mx = v;
            }
            const y1 = (0.5 - mx / 2) * h;
            const y2 = (0.5 - mn / 2) * h;
            ctx.fillRect(x, y1, 1, Math.max(1, y2 - y1));
        }
        ctx.globalAlpha = 1;

        // Selection overlay
        const xStart = timeToX(selection.start);
        const xEnd = timeToX(selection.end);
        if (xEnd > 0 && xStart < w) {
            const ox = Math.max(0, xStart);
            const ow = Math.min(w, xEnd) - ox;
            ctx.fillStyle = 'rgba(255,255,255,0.08)';
            ctx.fillRect(ox, 0, ow, h);
        }

        // Edge handles (red bars)
        if (xStart >= -2 && xStart <= w + 2) drawHandle(ctx, xStart, h, accent, 'start');
        if (xEnd >= -2 && xEnd <= w + 2) drawHandle(ctx, xEnd, h, accent, 'end');

        // Playhead
        if (isPlaying) {
            const px = timeToX(playhead);
            if (px >= 0 && px <= w) {
                ctx.strokeStyle = '#ffffff';
                ctx.lineWidth = 1.5;
                ctx.beginPath();
                ctx.moveTo(px + 0.5, 0);
                ctx.lineTo(px + 0.5, h);
                ctx.stroke();
            }
        }
    }, [buffer, selection, playhead, isPlaying, canvasSize, layout, timeToX]);

    // -----------------------------------------------------------------------
    // Draw ruler (time ticks)
    // -----------------------------------------------------------------------
    useEffect(() => {
        const ruler = rulerCanvasRef.current;
        if (!ruler || !buffer) return;
        const { w } = canvasSize;
        const dpr = window.devicePixelRatio || 1;
        ruler.width = Math.floor(w * dpr);
        ruler.height = Math.floor(RULER_HEIGHT * dpr);
        ruler.style.width = `${w}px`;
        ruler.style.height = `${RULER_HEIGHT}px`;

        const ctx = ruler.getContext('2d');
        if (!ctx) return;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, w, RULER_HEIGHT);
        ctx.fillStyle = 'rgba(255,255,255,0.03)';
        ctx.fillRect(0, 0, w, RULER_HEIGHT);

        // Pick a tick interval that gives ~60-120px between ticks
        const targetPx = 80;
        const targetSec = targetPx / layout.pxPerSec;
        const intervals = [0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2, 5, 10, 30, 60];
        const interval = intervals.find((v) => v >= targetSec) ?? 60;

        ctx.fillStyle = 'rgba(255,255,255,0.5)';
        ctx.font = '10px var(--font-mono, monospace)';
        ctx.strokeStyle = 'rgba(255,255,255,0.18)';

        const firstTick = Math.ceil(layout.clampedScrollT / interval) * interval;
        for (let t = firstTick; t <= layout.clampedScrollT + layout.viewDuration + 0.0001; t += interval) {
            const x = timeToX(t);
            if (x < 0 || x > w) continue;
            ctx.beginPath();
            ctx.moveTo(x + 0.5, RULER_HEIGHT - 6);
            ctx.lineTo(x + 0.5, RULER_HEIGHT);
            ctx.stroke();
            ctx.fillText(formatTickLabel(t, interval), x + 3, RULER_HEIGHT - 8);
        }
    }, [buffer, canvasSize, layout, timeToX]);

    // -----------------------------------------------------------------------
    // Hit testing for cursor + drag
    // -----------------------------------------------------------------------
    const hitTest = useCallback(
        (x: number): DragKind => {
            if (!buffer) return null;
            const xStart = timeToX(selection.start);
            const xEnd = timeToX(selection.end);
            if (Math.abs(x - xStart) <= HANDLE_HIT_PX) return 'start';
            if (Math.abs(x - xEnd) <= HANDLE_HIT_PX) return 'end';
            if (x > xStart && x < xEnd) return 'selection';
            return 'new';
        },
        [buffer, selection, timeToX],
    );

    // -----------------------------------------------------------------------
    // Mouse handlers
    // -----------------------------------------------------------------------
    const onCanvasMouseMove = useCallback(
        (e: React.MouseEvent<HTMLCanvasElement>) => {
            const canvas = canvasRef.current;
            if (!canvas || !buffer) return;
            const rect = canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const kind = hitTest(x);
            if (kind === 'start' || kind === 'end') setCursorStyle('ew-resize');
            else if (kind === 'selection') setCursorStyle('grab');
            else setCursorStyle('crosshair');
        },
        [buffer, hitTest],
    );

    const beginDrag = useCallback(
        (e: React.MouseEvent<HTMLCanvasElement>) => {
            const canvas = canvasRef.current;
            if (!canvas || !buffer) return;
            const rect = canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const startT = xToTime(x);

            let kind: Exclude<DragKind, null> = hitTest(x) as Exclude<DragKind, null>;
            if (e.button === 1 || (e.button === 0 && e.altKey)) {
                kind = 'pan';
            }

            let newSel = selection;
            if (kind === 'new') {
                newSel = { start: startT, end: startT };
                setSelection(newSel);
            }

            dragRef.current = {
                kind,
                startClientX: e.clientX,
                startTime: startT,
                origSel: newSel,
                origScrollX: layout.clampedScrollT,
            };
            setCursorStyle(
                kind === 'selection' || kind === 'pan' ? 'grabbing' : kind === 'new' ? 'crosshair' : 'ew-resize',
            );
            e.preventDefault();
        },
        [buffer, selection, hitTest, xToTime, layout.clampedScrollT],
    );

    // Global mousemove/up while dragging
    useEffect(() => {
        const onMove = (e: MouseEvent) => {
            const drag = dragRef.current;
            if (!drag || !buffer) return;
            const canvas = canvasRef.current;
            if (!canvas) return;
            const rect = canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const t = xToTime(x);

            if (drag.kind === 'start') {
                const newStart = Math.min(Math.max(t, 0), drag.origSel.end - 0.005);
                setSelection({ start: newStart, end: drag.origSel.end });
            } else if (drag.kind === 'end') {
                const newEnd = Math.max(Math.min(t, duration), drag.origSel.start + 0.005);
                setSelection({ start: drag.origSel.start, end: newEnd });
            } else if (drag.kind === 'new') {
                const a = Math.min(drag.startTime, t);
                const b = Math.max(drag.startTime, t);
                setSelection({
                    start: Math.max(0, a),
                    end: Math.min(duration, b),
                });
            } else if (drag.kind === 'selection') {
                const delta = t - drag.startTime;
                const len = drag.origSel.end - drag.origSel.start;
                let newStart = drag.origSel.start + delta;
                newStart = Math.min(Math.max(newStart, 0), duration - len);
                setSelection({ start: newStart, end: newStart + len });
            } else if (drag.kind === 'pan') {
                const dtPx = e.clientX - drag.startClientX;
                const dtSec = -dtPx / layout.pxPerSec;
                setScrollT(drag.origScrollX + dtSec);
            }
        };
        const onUp = () => {
            if (!dragRef.current) return;
            dragRef.current = null;
            setCursorStyle('default');
        };
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
        return () => {
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        };
    }, [buffer, duration, xToTime, layout.pxPerSec]);

    // -----------------------------------------------------------------------
    // Wheel: Ctrl = zoom, otherwise pan
    // -----------------------------------------------------------------------
    const onWheel = useCallback(
        (e: React.WheelEvent<HTMLCanvasElement>) => {
            if (!buffer) return;
            e.preventDefault();
            const canvas = canvasRef.current;
            if (!canvas) return;
            const rect = canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;

            if (e.ctrlKey || e.metaKey) {
                const factor = e.deltaY > 0 ? 1 / 1.15 : 1.15;
                const newZoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom * factor));
                // Anchor zoom on cursor: keep t under cursor stable
                const tUnder = xToTime(x);
                setZoom(newZoom);
                // After zoom, solve for scrollT that keeps tUnder at x
                const newViewDuration = duration / newZoom;
                const newPxPerSec = canvasSize.w / newViewDuration;
                const newScroll = tUnder - x / newPxPerSec;
                const maxScroll = Math.max(0, duration - newViewDuration);
                setScrollT(Math.min(Math.max(newScroll, 0), maxScroll));
            } else {
                // Horizontal pan
                const delta = (e.deltaY + e.deltaX) / layout.pxPerSec;
                const maxScroll = Math.max(0, duration - layout.viewDuration);
                setScrollT((prev) => Math.min(Math.max(prev + delta, 0), maxScroll));
            }
        },
        [buffer, zoom, xToTime, canvasSize.w, duration, layout.pxPerSec, layout.viewDuration],
    );

    // -----------------------------------------------------------------------
    // Playback
    // -----------------------------------------------------------------------
    const stopPlayback = useCallback(() => {
        try {
            sourceRef.current?.stop();
        } catch {}
        sourceRef.current?.disconnect();
        sourceRef.current = null;
        audioCtxRef.current?.close().catch(() => {});
        audioCtxRef.current = null;
        if (rafRef.current !== null) {
            cancelAnimationFrame(rafRef.current);
            rafRef.current = null;
        }
        playTrackerRef.current = null;
        setIsPlaying(false);
    }, []);

    const startPlayback = useCallback(
        (from: number, to: number) => {
            if (!buffer) return;
            stopPlayback();
            const start = Math.max(0, Math.min(from, duration - 0.01));
            const end = Math.min(duration, Math.max(to, start + 0.01));

            const ctx = new AudioContext();
            const src = ctx.createBufferSource();
            src.buffer = buffer;
            src.connect(ctx.destination);
            src.onended = () => {
                stopPlayback();
            };
            src.start(0, start, end - start);

            audioCtxRef.current = ctx;
            sourceRef.current = src;
            playTrackerRef.current = {
                ctxTime: ctx.currentTime,
                offset: start,
                endAt: end,
            };
            setIsPlaying(true);
            setPlayhead(start);

            const tick = () => {
                const tracker = playTrackerRef.current;
                const c = audioCtxRef.current;
                if (!tracker || !c) return;
                const elapsed = c.currentTime - tracker.ctxTime;
                const t = tracker.offset + elapsed;
                if (t >= tracker.endAt) {
                    stopPlayback();
                    return;
                }
                setPlayhead(t);
                rafRef.current = requestAnimationFrame(tick);
            };
            rafRef.current = requestAnimationFrame(tick);
        },
        [buffer, duration, stopPlayback],
    );

    // Unmount cleanup
    useEffect(() => () => stopPlayback(), [stopPlayback]);

    // -----------------------------------------------------------------------
    // Keyboard
    // -----------------------------------------------------------------------
    useEffect(() => {
        const onKey = (e: KeyboardEvent) => {
            const tag = (e.target as HTMLElement | null)?.tagName;
            if (tag === 'INPUT' || tag === 'TEXTAREA') return;
            if (e.key === 'Escape') {
                e.preventDefault();
                if (!applying) onClose();
            } else if (e.code === 'Space') {
                e.preventDefault();
                if (isPlaying) stopPlayback();
                else startPlayback(selection.start, selection.end);
            } else if (e.key === '+' || e.key === '=') {
                e.preventDefault();
                setZoom((z) => Math.min(MAX_ZOOM, z * 1.5));
            } else if (e.key === '-' || e.key === '_') {
                e.preventDefault();
                setZoom((z) => Math.max(MIN_ZOOM, z / 1.5));
            } else if (e.key === '0') {
                e.preventDefault();
                setZoom(1);
                setScrollT(0);
            }
        };
        window.addEventListener('keydown', onKey);
        return () => window.removeEventListener('keydown', onKey);
    }, [isPlaying, stopPlayback, startPlayback, selection, applying, onClose]);

    // -----------------------------------------------------------------------
    // Toolbar actions
    // -----------------------------------------------------------------------
    const playAll = () => startPlayback(0, duration);
    const playSelection = () => startPlayback(selection.start, selection.end);
    const zoomIn = () => setZoom((z) => Math.min(MAX_ZOOM, z * 1.5));
    const zoomOut = () => setZoom((z) => Math.max(MIN_ZOOM, z / 1.5));
    const resetZoom = () => {
        setZoom(1);
        setScrollT(0);
    };
    const fitSelection = () => {
        if (!buffer) return;
        const len = selection.end - selection.start;
        if (len <= 0) return;
        const pad = len * 0.1;
        const newView = Math.max(0.05, len + pad * 2);
        const newZoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, duration / newView));
        const center = (selection.start + selection.end) / 2;
        const newScroll = Math.max(0, Math.min(duration - newView, center - newView / 2));
        setZoom(newZoom);
        setScrollT(newScroll);
    };
    const selectAll = () => setSelection({ start: 0, end: duration });

    const handleApply = useCallback(async () => {
        if (!buffer) return;
        setApplying(true);
        try {
            stopPlayback();
            const sliced = sliceAudioBuffer(buffer, selection.start, selection.end);
            const wav = audioBufferToWav(sliced);
            await onApply(wav);
        } finally {
            setApplying(false);
        }
    }, [buffer, selection, onApply, stopPlayback]);

    // -----------------------------------------------------------------------
    // Scrollbar
    // -----------------------------------------------------------------------
    const scrollbar = useMemo(() => {
        if (!buffer || zoom <= 1) return null;
        const thumbW = Math.max(30, (layout.viewDuration / duration) * canvasSize.w);
        const thumbX = (layout.clampedScrollT / duration) * canvasSize.w;
        return { thumbW, thumbX };
    }, [buffer, zoom, layout, duration, canvasSize.w]);

    const onScrollbarDrag = (e: React.MouseEvent<HTMLDivElement>) => {
        if (!buffer || !scrollbar) return;
        const track = e.currentTarget;
        const rect = track.getBoundingClientRect();
        const updateFromX = (clientX: number) => {
            const x = clientX - rect.left - scrollbar.thumbW / 2;
            const ratio = Math.max(0, Math.min(1, x / (canvasSize.w - scrollbar.thumbW)));
            const maxScroll = Math.max(0, duration - layout.viewDuration);
            setScrollT(ratio * maxScroll);
        };
        updateFromX(e.clientX);
        const onMove = (ev: MouseEvent) => updateFromX(ev.clientX);
        const onUp = () => {
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        };
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    };

    // -----------------------------------------------------------------------
    // Render
    // -----------------------------------------------------------------------
    return (
        <div style={styles.overlay} onClick={() => !applying && onClose()}>
            <div style={styles.modal} onClick={(e) => e.stopPropagation()}>
                <div style={styles.header}>
                    <span style={{ fontWeight: 600 }}>Audio cutter</span>
                    <span style={styles.subtle}>
                        WEM {entry.id} · {duration > 0 ? formatTime(duration) : '—'}
                    </span>
                </div>

                {loadError && (
                    <div style={styles.errorBar}>Failed to decode: {loadError}</div>
                )}

                <div style={styles.toolbar}>
                    <button
                        className="btn btn--sm"
                        onClick={isPlaying ? stopPlayback : playAll}
                        disabled={!buffer || applying}
                        title="Space"
                    >
                        {isPlaying ? '■ Stop' : '▶ Play all'}
                    </button>
                    <button
                        className="btn btn--sm"
                        onClick={playSelection}
                        disabled={!buffer || applying || selection.end <= selection.start + 0.001}
                    >
                        ▶ Play selection
                    </button>

                    <span style={styles.toolbarSep} />

                    <button className="btn btn--sm btn--ghost" onClick={zoomOut} disabled={!buffer || zoom <= MIN_ZOOM} title="-">
                        −
                    </button>
                    <span style={styles.zoomLabel}>{zoom.toFixed(1)}×</span>
                    <button className="btn btn--sm btn--ghost" onClick={zoomIn} disabled={!buffer || zoom >= MAX_ZOOM} title="+">
                        +
                    </button>
                    <button className="btn btn--sm btn--ghost" onClick={resetZoom} disabled={!buffer} title="0">
                        Reset
                    </button>
                    <button className="btn btn--sm btn--ghost" onClick={fitSelection} disabled={!buffer}>
                        Fit selection
                    </button>

                    <span style={styles.toolbarSep} />

                    <button className="btn btn--sm btn--ghost" onClick={selectAll} disabled={!buffer}>
                        Select all
                    </button>
                </div>

                <div ref={containerRef} style={styles.canvasWrap}>
                    {!buffer && !loadError && (
                        <div style={styles.loading}>
                            <div className="spinner" />
                            <span>Decoding audio...</span>
                        </div>
                    )}
                    {buffer && (
                        <>
                            <canvas ref={rulerCanvasRef} style={{ display: 'block' }} />
                            <canvas
                                ref={canvasRef}
                                style={{ display: 'block', cursor: cursorStyle, userSelect: 'none' }}
                                onMouseDown={beginDrag}
                                onMouseMove={onCanvasMouseMove}
                                onWheel={onWheel}
                                onDoubleClick={selectAll}
                            />
                            {scrollbar && (
                                <div style={styles.scrollbarTrack} onMouseDown={onScrollbarDrag}>
                                    <div
                                        style={{
                                            ...styles.scrollbarThumb,
                                            width: scrollbar.thumbW,
                                            left: scrollbar.thumbX,
                                        }}
                                    />
                                </div>
                            )}
                        </>
                    )}
                </div>

                <div style={styles.infoRow}>
                    <div style={styles.subtle}>
                        Drag red bars to adjust · Click empty area to start new selection · Ctrl+wheel to zoom · Alt+drag to pan
                    </div>
                </div>

                <div style={styles.timeRow}>
                    <TimeField
                        label="Start"
                        value={selection.start}
                        max={duration}
                        onChange={(v) =>
                            setSelection((s) => ({ start: Math.min(v, s.end - 0.01), end: s.end }))
                        }
                    />
                    <TimeField
                        label="End"
                        value={selection.end}
                        max={duration}
                        onChange={(v) =>
                            setSelection((s) => ({ start: s.start, end: Math.max(v, s.start + 0.01) }))
                        }
                    />
                    <TimeField
                        label="Length"
                        value={selection.end - selection.start}
                        max={duration}
                        readOnly
                    />
                </div>

                <div style={styles.footer}>
                    <button className="btn btn--sm btn--ghost" onClick={onClose} disabled={applying}>
                        Cancel
                    </button>
                    <button
                        className="btn btn--sm btn--primary"
                        onClick={handleApply}
                        disabled={!buffer || applying || selection.end <= selection.start + 0.001}
                        title="Replace this WEM with the selected range"
                    >
                        {applying ? 'Applying...' : 'Apply trim'}
                    </button>
                </div>
            </div>
        </div>
    );
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function drawHandle(
    ctx: CanvasRenderingContext2D,
    x: number,
    h: number,
    accent: string,
    _kind: 'start' | 'end',
) {
    ctx.save();
    ctx.strokeStyle = accent;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(x + 0.5, 0);
    ctx.lineTo(x + 0.5, h);
    ctx.stroke();

    // Grab grip: two small squares at top/bottom
    ctx.fillStyle = accent;
    ctx.fillRect(x - 3, 0, 6, 8);
    ctx.fillRect(x - 3, h - 8, 6, 8);
    ctx.restore();
}

function formatTickLabel(t: number, interval: number): string {
    if (interval < 0.1) return t.toFixed(2) + 's';
    if (interval < 1) return t.toFixed(1) + 's';
    if (t < 60) return t.toFixed(0) + 's';
    return formatTime(t);
}

// ---------------------------------------------------------------------------
// Time input field
// ---------------------------------------------------------------------------

const TimeField: React.FC<{
    label: string;
    value: number;
    max: number;
    readOnly?: boolean;
    onChange?: (v: number) => void;
}> = ({ label, value, max, readOnly, onChange }) => {
    const [text, setText] = useState(value.toFixed(3));
    useEffect(() => {
        setText(value.toFixed(3));
    }, [value]);
    const commit = () => {
        if (readOnly || !onChange) return;
        const n = Number(text);
        if (!isNaN(n)) onChange(Math.max(0, Math.min(max, n)));
        else setText(value.toFixed(3));
    };
    return (
        <label style={styles.timeField}>
            <span style={styles.timeLabel}>{label}</span>
            <input
                type="number"
                step={0.01}
                min={0}
                max={max}
                value={text}
                readOnly={readOnly}
                onChange={(e) => setText(e.target.value)}
                onBlur={commit}
                onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                        e.preventDefault();
                        (e.target as HTMLInputElement).blur();
                    }
                }}
                style={{
                    ...styles.timeInput,
                    opacity: readOnly ? 0.7 : 1,
                }}
            />
            <span style={styles.timeSuffix}>s</span>
        </label>
    );
};

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles: Record<string, React.CSSProperties> = {
    overlay: {
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.6)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1001,
    },
    modal: {
        background: 'var(--bg-secondary)',
        border: '1px solid var(--border)',
        borderRadius: 10,
        width: 860,
        maxWidth: 'calc(100vw - 40px)',
        boxShadow: '0 12px 40px rgba(0,0,0,0.5)',
        display: 'flex',
        flexDirection: 'column',
    },
    header: {
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: '14px 18px',
        borderBottom: '1px solid var(--border)',
    },
    subtle: { color: 'var(--text-muted)', fontSize: 12 },
    errorBar: {
        padding: '8px 16px',
        color: '#f87171',
        fontSize: 12,
        background: 'rgba(239,68,68,0.1)',
        borderBottom: '1px solid var(--border)',
    },
    toolbar: {
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        padding: '10px 16px',
        borderBottom: '1px solid var(--border)',
        flexWrap: 'wrap',
    },
    toolbarSep: {
        width: 1,
        height: 18,
        background: 'var(--border)',
        margin: '0 6px',
    },
    zoomLabel: {
        fontFamily: 'var(--font-mono, monospace)',
        fontSize: 11,
        color: 'var(--text-muted)',
        minWidth: 40,
        textAlign: 'center',
    },
    canvasWrap: {
        position: 'relative',
        padding: '8px 16px',
        background: 'rgba(0,0,0,0.15)',
    },
    loading: {
        height: 200,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 10,
        color: 'var(--text-muted)',
        fontSize: 12,
    },
    scrollbarTrack: {
        position: 'relative',
        height: 10,
        marginTop: 6,
        background: 'rgba(255,255,255,0.04)',
        borderRadius: 3,
        cursor: 'pointer',
    },
    scrollbarThumb: {
        position: 'absolute',
        top: 1,
        bottom: 1,
        background: 'var(--accent-primary)',
        opacity: 0.6,
        borderRadius: 3,
    },
    infoRow: {
        padding: '8px 16px',
        borderTop: '1px solid var(--border)',
    },
    timeRow: {
        display: 'flex',
        gap: 14,
        padding: '10px 16px',
        borderTop: '1px solid var(--border)',
        flexWrap: 'wrap',
    },
    timeField: {
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        fontSize: 12,
    },
    timeLabel: {
        color: 'var(--text-muted)',
        minWidth: 46,
    },
    timeInput: {
        width: 100,
        padding: '4px 8px',
        fontSize: 12,
        fontFamily: 'var(--font-mono, monospace)',
        background: 'var(--bg-primary)',
        color: 'var(--text-primary)',
        border: '1px solid var(--border)',
        borderRadius: 4,
        outline: 'none',
    },
    timeSuffix: {
        color: 'var(--text-muted)',
        fontSize: 11,
    },
    footer: {
        display: 'flex',
        justifyContent: 'flex-end',
        gap: 8,
        padding: '12px 16px',
        borderTop: '1px solid var(--border)',
    },
};
