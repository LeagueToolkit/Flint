/**
 * Flint - BIN Editor Component (Monaco Editor)
 *
 * A full-featured code editor for viewing and editing Ritobin (.bin) files
 * using Monaco Editor directly (no @monaco-editor/react wrapper — that
 * library's internal loader breaks in Tauri production builds).
 *
 * Features:
 * - Custom Ritobin language with semantic tokenization
 * - Matching dark theme
 * - Dirty state tracking and save functionality
 * - Asset preview on hover (textures, meshes)
 */

import React, { useState, useEffect, useCallback, useRef } from 'react';
import * as monaco from 'monaco-editor';
import type { editor } from 'monaco-editor';
import { useAppState, useAppMetadataStore, useConfigStore } from '../../lib/stores';
import * as api from '../../lib/api';
import { getIcon } from '../../lib/fileIcons';
import {
    RITOBIN_LANGUAGE_ID,
    RITOBIN_THEME_ID,
    registerRitobinLanguage,
    registerRitobinTheme
} from '../../lib/ritobinLanguage';
import { AssetPreviewTooltip } from './AssetPreviewTooltip';

// Configure Monaco workers — wrap in try-catch so a broken worker doesn't
// cascade and break the entire editor (Monarch tokenizer runs on main thread anyway)
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';

self.MonacoEnvironment = {
    getWorker(_: unknown, label: string) {
        try {
            if (label === 'json') return new jsonWorker();
            return new editorWorker();
        } catch (e) {
            console.warn('[Monaco] Worker creation failed, falling back to main thread:', e);
            const blob = new Blob(['self.onmessage=function(){}'], { type: 'text/javascript' });
            return new Worker(URL.createObjectURL(blob));
        }
    },
};

// Register language and theme at module level — guaranteed to run before
// any editor.create() call since ES module imports are evaluated first.
registerRitobinLanguage(monaco as any);
registerRitobinTheme(monaco as any);

/** Delay in milliseconds before showing the asset preview tooltip */
const HOVER_DELAY_MS = 3000;

/** Asset file extensions that can be previewed */
const PREVIEWABLE_EXTENSIONS = ['tex', 'dds', 'scb', 'sco', 'skn'];

function isPreviewableAssetPath(value: string): boolean {
    if (!value) return false;
    const ext = value.toLowerCase().split('.').pop() || '';
    return PREVIEWABLE_EXTENSIONS.includes(ext);
}

/**
 * Extract string value from a line at a given column position.
 * Returns the string content if cursor is inside a quoted string.
 */
function extractStringAtPosition(line: string, column: number): string | null {
    const stringPattern = /"([^"\\]*(\\.[^"\\]*)*)"/g;
    let match;
    while ((match = stringPattern.exec(line)) !== null) {
        const startCol = match.index + 1;
        const endCol = match.index + match[0].length;
        if (column >= startCol && column <= endCol) return match[1];
    }
    return null;
}

/** Monaco editor options shared across create calls */
const EDITOR_OPTIONS: editor.IStandaloneEditorConstructionOptions = {
    automaticLayout: true,
    fontFamily: 'var(--font-mono), "Cascadia Code", "Fira Code", Consolas, "Courier New", monospace',
    fontSize: 13,
    lineHeight: 20,
    lineNumbers: 'on',
    lineNumbersMinChars: 5,
    minimap: { enabled: false },
    folding: false,
    bracketPairColorization: { enabled: false },
    matchBrackets: 'never',
    maxTokenizationLineLength: 5000,
    stopRenderingLineAfter: 10000,
    scrollBeyondLastLine: false,
    smoothScrolling: false,
    fastScrollSensitivity: 5,
    cursorBlinking: 'solid',
    cursorSmoothCaretAnimation: 'off',
    cursorStyle: 'line',
    renderWhitespace: 'none',
    renderControlCharacters: false,
    renderLineHighlight: 'none',
    renderValidationDecorations: 'off',
    occurrencesHighlight: 'off',
    selectionHighlight: false,
    guides: {
        indentation: false,
        bracketPairs: false,
        highlightActiveBracketPair: false,
        highlightActiveIndentation: false,
    },
    scrollbar: {
        vertical: 'auto',
        horizontal: 'auto',
        verticalScrollbarSize: 12,
        horizontalScrollbarSize: 12,
        useShadows: false,
    },
    tabSize: 4,
    insertSpaces: true,
    autoIndent: 'none',
    formatOnPaste: false,
    formatOnType: false,
    wordWrap: 'off',
    quickSuggestions: false,
    suggestOnTriggerCharacters: false,
    acceptSuggestionOnEnter: 'off',
    parameterHints: { enabled: false },
    wordBasedSuggestions: 'off',
    hover: { enabled: false },
    links: false,
    colorDecorators: false,
    codeLens: false,
    inlineSuggest: { enabled: false },
    contextmenu: false,
    accessibilitySupport: 'off',
};

interface BinEditorProps {
    filePath: string;
}

export const BinEditor: React.FC<BinEditorProps> = ({ filePath }) => {
    const { showToast, setWorking, setReady } = useAppState();
    const binConverterEngine = useConfigStore((state) => state.binConverterEngine);
    const jadePath = useConfigStore((state) => state.jadePath);
    const quartzPath = useConfigStore((state) => state.quartzPath);

    const [content, setContent] = useState<string>('');
    const [originalContent, setOriginalContent] = useState<string>('');
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [lineCount, setLineCount] = useState(0);

    // Subscribe to file version changes for hot reload
    const fileVersion = useAppMetadataStore((state) => state.fileVersions[filePath] || 0);

    const useJade = binConverterEngine === 'jade';

    const editorContainerRef = useRef<HTMLDivElement>(null);
    const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);

    // Asset preview tooltip state
    const [previewAsset, setPreviewAsset] = useState<string | null>(null);
    const [previewPosition, setPreviewPosition] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
    const [showPreview, setShowPreview] = useState(false);
    const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const lastHoveredAssetRef = useRef<string | null>(null);
    const containerRef = useRef<HTMLDivElement | null>(null);

    const isDirty = content !== originalContent;
    const basePath = filePath.split(/[/\\]/).slice(0, -1).join('\\');

    // Load BIN file
    useEffect(() => {
        const loadBin = async () => {
            setLoading(true);
            setError(null);
            try {
                const text = await api.readOrConvertBin(filePath, useJade);
                setContent(text);
                setOriginalContent(text);
                setLineCount(text.split('\n').length);
            } catch (err) {
                console.error('[BinEditor] Error:', err);
                setError((err as Error).message || 'Failed to load BIN file');
            } finally {
                setLoading(false);
            }
        };
        loadBin();
    }, [filePath, fileVersion, useJade]); // Re-run when file version changes (hot reload) or engine changes

    // Create Monaco editor directly once content is loaded.
    // Disposes and recreates when file changes (loading cycles false→true→false).
    useEffect(() => {
        if (loading || error || !editorContainerRef.current) return;

        const ed = monaco.editor.create(editorContainerRef.current, {
            ...EDITOR_OPTIONS,
            value: content,
            language: RITOBIN_LANGUAGE_ID,
            theme: RITOBIN_THEME_ID,
        });

        editorRef.current = ed;

        const model = ed.getModel();
        if (model) {
            setLineCount(model.getLineCount());
            model.onDidChangeContent(() => {
                setContent(ed.getValue());
                setLineCount(model.getLineCount());
            });
        }

        return () => {
            ed.dispose();
            editorRef.current = null;
        };
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [loading, error]);

    const handleSave = useCallback(async () => {
        try {
            setWorking('Saving BIN file...');
            await api.saveRitobinToBin(filePath, content, useJade);
            setOriginalContent(content);
            setReady('Saved');
            showToast('success', 'BIN file saved successfully');
        } catch (err) {
            console.error('[BinEditor] Save error:', err);
            const flintError = err as api.FlintError;
            showToast('error', flintError.getUserMessage?.() || 'Failed to save');
        }
    }, [filePath, content, useJade, setWorking, setReady, showToast]);

    const handleOpenWithJade = useCallback(async () => {
        if (!jadePath) return;
        try {
            // Normalize path: ensure consistent backslashes for Windows
            const normalizedPath = filePath.replace(/\//g, '\\');
            await api.launchJade(normalizedPath, jadePath);
        } catch (err) {
            const message = (err as Error).message || String(err);
            console.error('[BinEditor] Failed to launch Jade:', message);
            showToast('error', `Failed to launch Jade: ${message}`);
        }
    }, [filePath, jadePath, showToast]);

    const handleOpenWithQuartz = useCallback(async () => {
        if (!quartzPath) return;
        try {
            // Normalize path: ensure consistent backslashes for Windows
            const normalizedPath = filePath.replace(/\//g, '\\');
            await api.launchQuartz(normalizedPath, quartzPath);
        } catch (err) {
            const message = (err as Error).message || String(err);
            console.error('[BinEditor] Failed to launch Quartz:', message);
            showToast('error', `Failed to launch Quartz: ${message}`);
        }
    }, [filePath, quartzPath, showToast]);

    const handleOpenDefault = useCallback(async () => {
        try {
            // Normalize path: ensure consistent backslashes for Windows
            const normalizedPath = filePath.replace(/\//g, '\\');
            await api.openWithDefaultApp(normalizedPath);
        } catch (err) {
            const message = (err as Error).message || String(err);
            console.error('[BinEditor] Failed to open file:', message);
            showToast('error', `Failed to open file: ${message}`);
        }
    }, [filePath, showToast]);

    useEffect(() => {
        return () => {
            if (hoverTimerRef.current) clearTimeout(hoverTimerRef.current);
        };
    }, []);

    const handleMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
        if (!editorRef.current) return;
        const editorInst = editorRef.current;
        const target = e.target as HTMLElement;
        if (!target.closest('.monaco-editor')) return;
        if (!editorInst.getDomNode()) return;

        const pos = editorInst.getTargetAtClientPoint(e.clientX, e.clientY);
        if (!pos?.position) {
            if (hoverTimerRef.current) { clearTimeout(hoverTimerRef.current); hoverTimerRef.current = null; }
            return;
        }

        const model = editorInst.getModel();
        if (!model) return;

        const lineContent = model.getLineContent(pos.position.lineNumber);
        const stringValue = extractStringAtPosition(lineContent, pos.position.column);

        setPreviewPosition({ x: e.clientX, y: e.clientY });

        if (stringValue && isPreviewableAssetPath(stringValue)) {
            if (stringValue !== lastHoveredAssetRef.current) {
                lastHoveredAssetRef.current = stringValue;
                setShowPreview(false);
                if (hoverTimerRef.current) clearTimeout(hoverTimerRef.current);
                hoverTimerRef.current = setTimeout(() => {
                    setPreviewAsset(stringValue);
                    setShowPreview(true);
                }, HOVER_DELAY_MS);
            }
        } else {
            if (hoverTimerRef.current) { clearTimeout(hoverTimerRef.current); hoverTimerRef.current = null; }
            lastHoveredAssetRef.current = null;
            setShowPreview(false);
        }
    }, []);

    const handleMouseLeave = useCallback(() => {
        if (hoverTimerRef.current) { clearTimeout(hoverTimerRef.current); hoverTimerRef.current = null; }
        lastHoveredAssetRef.current = null;
        setShowPreview(false);
    }, []);

    const handleClick = useCallback(() => {
        if (hoverTimerRef.current) { clearTimeout(hoverTimerRef.current); hoverTimerRef.current = null; }
        setShowPreview(false);
    }, []);

    const fileName = filePath.split('\\').pop() || filePath.split('/').pop() || 'file.bin';

    if (loading) {
        return (
            <div className="bin-editor__loading">
                <div className="spinner spinner--lg" />
                <span>Loading BIN file...</span>
            </div>
        );
    }

    if (error) {
        return (
            <div className="bin-editor__error">
                <span dangerouslySetInnerHTML={{ __html: getIcon('warning') }} />
                <span>{error}</span>
            </div>
        );
    }

    return (
        <div className="bin-editor">
            <div className="bin-editor__toolbar">
                <span className="bin-editor__filename">
                    {fileName}{isDirty ? ' •' : ''}
                    <span className="bin-editor__stats">
                        {lineCount.toLocaleString()} lines
                    </span>
                </span>
                <div className="bin-editor__toolbar-actions">
                    {quartzPath && (
                        <button
                            className="btn btn--secondary btn--sm"
                            onClick={handleOpenWithQuartz}
                            title="Open with Quartz VFX Editor"
                        >
                            Open in Quartz
                        </button>
                    )}
                    {jadePath ? (
                        <button
                            className="btn btn--secondary btn--sm"
                            onClick={handleOpenWithJade}
                            title="Open with Jade League Bin Editor"
                        >
                            Open with Jade
                        </button>
                    ) : (
                        <button
                            className="btn btn--secondary btn--sm"
                            onClick={handleOpenDefault}
                            title="Open with default application"
                        >
                            Open
                        </button>
                    )}
                    <button
                        className="btn btn--primary btn--sm"
                        onClick={handleSave}
                        disabled={!isDirty}
                    >
                        Save
                    </button>
                </div>
            </div>

            <div
                className="bin-editor__content"
                ref={containerRef}
                onMouseMove={handleMouseMove}
                onMouseLeave={handleMouseLeave}
                onClick={handleClick}
            >
                <div ref={editorContainerRef} style={{ width: '100%', height: '100%' }} />
            </div>

            {previewAsset && (
                <AssetPreviewTooltip
                    assetPath={previewAsset}
                    basePath={basePath}
                    position={previewPosition}
                    visible={showPreview}
                />
            )}
        </div>
    );
};
