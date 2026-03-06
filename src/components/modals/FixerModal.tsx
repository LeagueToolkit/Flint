/**
 * Flint - Fixer Modal (Hematite Integration)
 *
 * Provides "Fix Project" and "Batch Fix" functionality.
 * Scans BIN files for known issues and applies config-driven fixes.
 */

import React, { useState, useCallback, useEffect } from 'react';
import { useAppState } from '../../lib/stores';
import * as api from '../../lib/api';
import { open } from '@tauri-apps/plugin-dialog';
import { appDataDir } from '@tauri-apps/api/path';
import { getIcon } from '../../lib/fileIcons';
import type {
    ProjectAnalysis,
    ProjectFixResult,
    BatchFixResult,
    DetectedIssue,
    RecentProject,
    FixConfig,
} from '../../lib/types';

type FixerTab = 'single' | 'batch';
type FixerPhase = 'idle' | 'loading-config' | 'scanning' | 'results' | 'fixing' | 'done';

/** Get the default Flint projects directory: {AppData}/RitoShark/Flint/Projects */
async function getProjectsBasePath(): Promise<string | undefined> {
    try {
        const dir = await appDataDir();
        const parts = dir.replace(/\\/g, '/').split('/');
        parts.pop();
        return `${parts.join('/')}/RitoShark/Flint/Projects`;
    } catch {
        return undefined;
    }
}

export const FixerModal: React.FC = () => {
    const { state, closeModal, showToast } = useAppState();

    const isVisible = state.activeModal === 'fixer';

    // Current tab
    const [tab, setTab] = useState<FixerTab>('single');

    // Config
    const [config, setConfig] = useState<FixConfig | null>(null);

    // Single project state
    const [projectPath, setProjectPath] = useState('');
    const [analysis, setAnalysis] = useState<ProjectAnalysis | null>(null);
    const [fixResult, setFixResult] = useState<ProjectFixResult | null>(null);
    const [selectedFixes, setSelectedFixes] = useState<Set<string>>(new Set());

    // Batch state
    const [batchPaths, setBatchPaths] = useState<string[]>([]);
    const [batchResult, setBatchResult] = useState<BatchFixResult | null>(null);

    // Phase
    const [phase, setPhase] = useState<FixerPhase>('idle');
    const [statusMessage, setStatusMessage] = useState('');

    // Auto-fill project path from active tab
    useEffect(() => {
        if (isVisible) {
            const activeTab = state.openTabs.find(t => t.id === state.activeTabId);
            if (activeTab?.projectPath) {
                setProjectPath(activeTab.projectPath);
            }
        }
    }, [isVisible, state.activeTabId, state.openTabs]);

    // Reset state when modal closes
    useEffect(() => {
        if (!isVisible) {
            setPhase('idle');
            setAnalysis(null);
            setFixResult(null);
            setBatchResult(null);
            setSelectedFixes(new Set());
            setStatusMessage('');
        }
    }, [isVisible]);

    // =========================================================================
    // Single Project
    // =========================================================================

    const handleBrowseProject = useCallback(async () => {
        const defaultPath = await getProjectsBasePath();
        const selected = await open({ title: 'Select Flint Project Folder', directory: true, defaultPath });
        if (selected) setProjectPath(selected as string);
    }, []);

    const handleScanProject = useCallback(async () => {
        if (!projectPath) return;
        setPhase('scanning');
        setStatusMessage('Scanning project for issues...');
        setAnalysis(null);
        setFixResult(null);

        try {
            // Load config if not cached
            if (!config) {
                setPhase('loading-config');
                setStatusMessage('Fetching fix config...');
                const cfg = await api.getFixerConfig();
                setConfig(cfg);
            }

            setPhase('scanning');
            setStatusMessage('Analyzing BIN files...');
            const result = await api.analyzeProject(projectPath);
            setAnalysis(result);

            // Select all detected fixes by default
            const allFixIds = new Set<string>();
            for (const scan of result.results) {
                for (const issue of scan.detected_issues) {
                    allFixIds.add(issue.fix_id);
                }
            }
            setSelectedFixes(allFixIds);

            setPhase('results');
            setStatusMessage(
                result.issues_found > 0
                    ? `Found ${result.issues_found} issue(s) in ${result.results.length} file(s)`
                    : 'No issues found!'
            );
        } catch (err) {
            const msg = err instanceof api.FlintError ? err.getUserMessage() : String(err);
            showToast('error', msg);
            setPhase('idle');
            setStatusMessage('');
        }
    }, [projectPath, config, showToast]);

    const handleFixProject = useCallback(async () => {
        if (!projectPath || selectedFixes.size === 0) return;
        setPhase('fixing');
        setStatusMessage('Applying fixes...');

        try {
            const result = await api.fixProject(projectPath, Array.from(selectedFixes));
            setFixResult(result);
            setPhase('done');
            setStatusMessage(
                `Applied ${result.total_applied} fix(es)` +
                (result.total_failed > 0 ? `, ${result.total_failed} failed` : '')
            );
            showToast(
                result.total_failed > 0 ? 'warning' : 'success',
                `Fixed project: ${result.total_applied} applied, ${result.total_failed} failed`
            );
        } catch (err) {
            const msg = err instanceof api.FlintError ? err.getUserMessage() : String(err);
            showToast('error', msg);
            setPhase('results');
            setStatusMessage('Fix failed');
        }
    }, [projectPath, selectedFixes, showToast]);

    const toggleFix = useCallback((fixId: string) => {
        setSelectedFixes(prev => {
            const next = new Set(prev);
            if (next.has(fixId)) next.delete(fixId);
            else next.add(fixId);
            return next;
        });
    }, []);

    // =========================================================================
    // Batch Fix
    // =========================================================================

    const handleAddBatchProjects = useCallback(async () => {
        const defaultPath = await getProjectsBasePath();
        const selected = await open({
            title: 'Select Project Folders to Fix',
            directory: true,
            multiple: true,
            defaultPath,
        });
        if (selected) {
            const paths = Array.isArray(selected) ? selected : [selected];
            setBatchPaths(prev => [...new Set([...prev, ...paths])]);
        }
    }, []);

    const handleRemoveBatchPath = useCallback((path: string) => {
        setBatchPaths(prev => prev.filter(p => p !== path));
    }, []);

    const handleBatchFix = useCallback(async () => {
        if (batchPaths.length === 0) return;
        setPhase('fixing');
        setStatusMessage(`Fixing ${batchPaths.length} project(s)...`);
        setBatchResult(null);

        try {
            const result = await api.batchFixProjects(batchPaths);
            setBatchResult(result);
            setPhase('done');
            setStatusMessage(
                `Batch complete: ${result.total_applied} fixes across ${result.total_projects} projects`
            );
            showToast('success', `Batch fixed ${result.total_projects} projects (${result.total_applied} fixes)`);
        } catch (err) {
            const msg = err instanceof api.FlintError ? err.getUserMessage() : String(err);
            showToast('error', msg);
            setPhase('idle');
        }
    }, [batchPaths, showToast]);

    // =========================================================================
    // Render
    // =========================================================================

    if (!isVisible) return null;

    const isWorking = phase === 'scanning' || phase === 'fixing' || phase === 'loading-config';

    return (
        <div className={`modal-overlay ${isVisible ? 'modal-overlay--visible' : ''}`}>
            <div className="modal modal--wide">
                {/* Header */}
                <div className="modal__header">
                    <h2 className="modal__title">
                        <span dangerouslySetInnerHTML={{ __html: getIcon('wrench') }} />
                        {' '}Fixer
                    </h2>
                    <button className="modal__close" onClick={closeModal} disabled={isWorking}>
                        ×
                    </button>
                </div>

                {/* Tab bar */}
                <div style={{
                    display: 'flex',
                    borderBottom: '1px solid var(--border)',
                    padding: '0 var(--space-lg)',
                }}>
                    <button
                        className={`btn btn--ghost ${tab === 'single' ? 'btn--active' : ''}`}
                        onClick={() => { setTab('single'); setPhase('idle'); setBatchResult(null); }}
                        disabled={isWorking}
                        style={{
                            borderBottom: tab === 'single' ? '2px solid var(--accent-primary)' : '2px solid transparent',
                            borderRadius: 0,
                            padding: '8px 16px',
                        }}
                    >
                        Fix Project
                    </button>
                    <button
                        className={`btn btn--ghost ${tab === 'batch' ? 'btn--active' : ''}`}
                        onClick={() => { setTab('batch'); setPhase('idle'); setAnalysis(null); setFixResult(null); }}
                        disabled={isWorking}
                        style={{
                            borderBottom: tab === 'batch' ? '2px solid var(--accent-primary)' : '2px solid transparent',
                            borderRadius: 0,
                            padding: '8px 16px',
                        }}
                    >
                        Batch Fix
                    </button>
                </div>

                {/* Body */}
                <div className="modal__body" style={{ minHeight: '300px' }}>
                    {tab === 'single' ? (
                        <SingleFixTab
                            projectPath={projectPath}
                            setProjectPath={setProjectPath}
                            phase={phase}
                            statusMessage={statusMessage}
                            analysis={analysis}
                            fixResult={fixResult}
                            selectedFixes={selectedFixes}
                            toggleFix={toggleFix}
                            onBrowse={handleBrowseProject}
                            onScan={handleScanProject}
                            onFix={handleFixProject}
                            isWorking={isWorking}
                            recentProjects={state.recentProjects}
                        />
                    ) : (
                        <BatchFixTab
                            batchPaths={batchPaths}
                            phase={phase}
                            statusMessage={statusMessage}
                            batchResult={batchResult}
                            onAdd={handleAddBatchProjects}
                            onRemove={handleRemoveBatchPath}
                            onFix={handleBatchFix}
                            isWorking={isWorking}
                            recentProjects={state.recentProjects}
                            onAddPath={(p: string) => setBatchPaths(prev => [...new Set([...prev, p])])}
                        />
                    )}
                </div>

                {/* Footer */}
                <div className="modal__footer">
                    <button className="btn btn--ghost" onClick={closeModal} disabled={isWorking}>
                        Close
                    </button>
                </div>
            </div>
        </div>
    );
};

// =============================================================================
// Single Fix Tab
// =============================================================================

interface SingleFixTabProps {
    projectPath: string;
    setProjectPath: (p: string) => void;
    phase: FixerPhase;
    statusMessage: string;
    analysis: ProjectAnalysis | null;
    fixResult: ProjectFixResult | null;
    selectedFixes: Set<string>;
    toggleFix: (id: string) => void;
    onBrowse: () => void;
    onScan: () => void;
    onFix: () => void;
    isWorking: boolean;
    recentProjects: RecentProject[];
}

const SingleFixTab: React.FC<SingleFixTabProps> = ({
    projectPath, setProjectPath, phase, statusMessage,
    analysis, fixResult, selectedFixes, toggleFix,
    onBrowse, onScan, onFix, isWorking, recentProjects,
}) => (
    <div>
        {/* Recent projects quick-select */}
        {recentProjects.length > 0 && (
            <div className="form-group">
                <label className="form-label">Recent Projects</label>
                <div style={{
                    maxHeight: '120px',
                    overflowY: 'auto',
                    border: '1px solid var(--border)',
                    borderRadius: '6px',
                }}>
                    {recentProjects.map((p: RecentProject) => {
                        // Strip trailing project.json to get the folder path
                        const folderPath = p.path.replace(/[\\/]project\.json$/, '');
                        const isSelected = projectPath === folderPath;
                        return (
                            <div
                                key={p.path}
                                onClick={() => { if (!isWorking) setProjectPath(folderPath); }}
                                style={{
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: '8px',
                                    padding: '6px 12px',
                                    borderBottom: '1px solid var(--border)',
                                    cursor: isWorking ? 'default' : 'pointer',
                                    fontSize: '13px',
                                    background: isSelected ? 'var(--accent-primary-alpha, rgba(99,102,241,0.12))' : 'transparent',
                                }}
                            >
                                <span dangerouslySetInnerHTML={{ __html: getIcon('folder') }} style={{ flexShrink: 0, opacity: 0.6 }} />
                                <span style={{ flex: 1, fontWeight: isSelected ? 600 : 400 }}>
                                    {p.champion} - {p.name}
                                </span>
                            </div>
                        );
                    })}
                </div>
            </div>
        )}

        {/* Project path input */}
        <div className="form-group">
            <label className="form-label">Project Path</label>
            <div style={{ display: 'flex', gap: '8px' }}>
                <input
                    className="form-input"
                    value={projectPath}
                    onChange={e => setProjectPath(e.target.value)}
                    placeholder="Select a Flint project folder..."
                    disabled={isWorking}
                    style={{ flex: 1 }}
                />
                <button className="btn btn--secondary" onClick={onBrowse} disabled={isWorking}>
                    Browse
                </button>
                <button
                    className="btn btn--primary"
                    onClick={onScan}
                    disabled={!projectPath || isWorking}
                >
                    {phase === 'scanning' ? 'Scanning...' : 'Scan'}
                </button>
            </div>
        </div>

        {/* Status */}
        {statusMessage && (
            <div style={{
                padding: '8px 12px',
                marginTop: '12px',
                borderRadius: '6px',
                background: 'var(--bg-tertiary)',
                fontSize: '13px',
                color: phase === 'done' ? 'var(--accent-primary)' : 'var(--text-secondary)',
            }}>
                {isWorking && <span style={{ marginRight: '8px' }}>...</span>}
                {statusMessage}
            </div>
        )}

        {/* Scan results */}
        {analysis && phase === 'results' && analysis.issues_found > 0 && (
            <div style={{ marginTop: '16px' }}>
                <h3 style={{ fontSize: '14px', fontWeight: 600, marginBottom: '8px' }}>
                    Detected Issues ({analysis.issues_found})
                </h3>
                <div style={{
                    maxHeight: '200px',
                    overflowY: 'auto',
                    border: '1px solid var(--border)',
                    borderRadius: '6px',
                }}>
                    {analysis.results.map((scan) =>
                        scan.detected_issues.map((issue: DetectedIssue) => (
                            <label
                                key={`${scan.file_path}:${issue.fix_id}`}
                                style={{
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: '8px',
                                    padding: '8px 12px',
                                    borderBottom: '1px solid var(--border)',
                                    cursor: 'pointer',
                                    fontSize: '13px',
                                }}
                            >
                                <input
                                    type="checkbox"
                                    checked={selectedFixes.has(issue.fix_id)}
                                    onChange={() => toggleFix(issue.fix_id)}
                                />
                                <SeverityBadge severity={issue.severity} />
                                <span style={{ flex: 1 }}>
                                    <strong>{issue.fix_name}</strong>
                                    <span style={{ color: 'var(--text-muted)', marginLeft: '8px' }}>
                                        {issue.description}
                                    </span>
                                </span>
                            </label>
                        ))
                    )}
                </div>

                <button
                    className="btn btn--primary"
                    onClick={onFix}
                    disabled={selectedFixes.size === 0 || isWorking}
                    style={{ marginTop: '12px' }}
                >
                    {isWorking ? 'Fixing...' : `Apply ${selectedFixes.size} Fix(es)`}
                </button>
            </div>
        )}

        {/* Fix results */}
        {fixResult && phase === 'done' && (
            <div style={{ marginTop: '16px' }}>
                <h3 style={{ fontSize: '14px', fontWeight: 600, marginBottom: '8px' }}>
                    Fix Results
                </h3>
                <div style={{
                    maxHeight: '200px',
                    overflowY: 'auto',
                    border: '1px solid var(--border)',
                    borderRadius: '6px',
                    fontSize: '13px',
                }}>
                    {fixResult.results.map((r) => (
                        <div key={r.file_path} style={{ padding: '8px 12px', borderBottom: '1px solid var(--border)' }}>
                            <div style={{ fontWeight: 500, marginBottom: '4px', wordBreak: 'break-all' }}>
                                {r.file_path.split(/[\\/]/).slice(-2).join('/')}
                            </div>
                            {r.fixes_applied.map((f) => (
                                <div key={f.fix_id} style={{ color: 'var(--accent-primary)', paddingLeft: '12px' }}>
                                    + {f.description} ({f.changes_count} changes)
                                </div>
                            ))}
                            {r.fixes_failed.map((f) => (
                                <div key={f.fix_id} style={{ color: '#f87171', paddingLeft: '12px' }}>
                                    x {f.fix_id}: {f.error}
                                </div>
                            ))}
                        </div>
                    ))}
                </div>
            </div>
        )}
    </div>
);

// =============================================================================
// Batch Fix Tab
// =============================================================================

interface BatchFixTabProps {
    batchPaths: string[];
    phase: FixerPhase;
    statusMessage: string;
    batchResult: BatchFixResult | null;
    onAdd: () => void;
    onRemove: (path: string) => void;
    onFix: () => void;
    isWorking: boolean;
    recentProjects: RecentProject[];
    onAddPath: (path: string) => void;
}

const BatchFixTab: React.FC<BatchFixTabProps> = ({
    batchPaths, phase, statusMessage, batchResult,
    onAdd, onRemove, onFix, isWorking, recentProjects, onAddPath,
}) => {
    const addAllRecent = () => {
        for (const p of recentProjects) {
            const folderPath = p.path.replace(/[\\/]project\.json$/, '');
            onAddPath(folderPath);
        }
    };

    return (
    <div>
        <div className="form-group">
            <label className="form-label">Project Folders</label>
            <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginBottom: '8px' }}>
                Select multiple Flint project directories to fix at once.
            </p>
            <div style={{ display: 'flex', gap: '8px' }}>
                <button className="btn btn--secondary" onClick={onAdd} disabled={isWorking}>
                    Browse
                </button>
                {recentProjects.length > 0 && (
                    <button className="btn btn--secondary" onClick={addAllRecent} disabled={isWorking}>
                        Add All Recent ({recentProjects.length})
                    </button>
                )}
            </div>
        </div>

        {/* Path list */}
        {batchPaths.length > 0 && (
            <div style={{
                maxHeight: '180px',
                overflowY: 'auto',
                border: '1px solid var(--border)',
                borderRadius: '6px',
                marginTop: '8px',
            }}>
                {batchPaths.map((p) => (
                    <div
                        key={p}
                        style={{
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'space-between',
                            padding: '6px 12px',
                            borderBottom: '1px solid var(--border)',
                            fontSize: '13px',
                        }}
                    >
                        <span style={{ wordBreak: 'break-all', flex: 1 }}>{p}</span>
                        <button
                            className="btn btn--ghost"
                            onClick={() => onRemove(p)}
                            disabled={isWorking}
                            style={{ padding: '2px 6px', fontSize: '12px', color: '#f87171' }}
                        >
                            Remove
                        </button>
                    </div>
                ))}
            </div>
        )}

        {/* Status */}
        {statusMessage && (
            <div style={{
                padding: '8px 12px',
                marginTop: '12px',
                borderRadius: '6px',
                background: 'var(--bg-tertiary)',
                fontSize: '13px',
                color: phase === 'done' ? 'var(--accent-primary)' : 'var(--text-secondary)',
            }}>
                {isWorking && <span style={{ marginRight: '8px' }}>...</span>}
                {statusMessage}
            </div>
        )}

        {/* Fix button */}
        {batchPaths.length > 0 && phase !== 'done' && (
            <button
                className="btn btn--primary"
                onClick={onFix}
                disabled={isWorking}
                style={{ marginTop: '12px' }}
            >
                {isWorking ? 'Fixing...' : `Fix ${batchPaths.length} Project(s)`}
            </button>
        )}

        {/* Batch results */}
        {batchResult && phase === 'done' && (
            <div style={{ marginTop: '16px' }}>
                <h3 style={{ fontSize: '14px', fontWeight: 600, marginBottom: '8px' }}>
                    Batch Results
                </h3>
                <div style={{
                    maxHeight: '200px',
                    overflowY: 'auto',
                    border: '1px solid var(--border)',
                    borderRadius: '6px',
                    fontSize: '13px',
                }}>
                    {batchResult.projects.map((proj) => (
                        <div key={proj.project_path} style={{ padding: '8px 12px', borderBottom: '1px solid var(--border)' }}>
                            <div style={{ fontWeight: 500, marginBottom: '2px', wordBreak: 'break-all' }}>
                                {proj.project_path.split(/[\\/]/).pop()}
                            </div>
                            <span style={{ color: 'var(--accent-primary)' }}>
                                {proj.total_applied} fixed
                            </span>
                            {proj.total_failed > 0 && (
                                <span style={{ color: '#f87171', marginLeft: '8px' }}>
                                    {proj.total_failed} failed
                                </span>
                            )}
                            {proj.total_applied === 0 && proj.total_failed === 0 && (
                                <span style={{ color: 'var(--text-muted)' }}>No issues</span>
                            )}
                        </div>
                    ))}
                </div>
                <div style={{
                    marginTop: '8px',
                    padding: '8px 12px',
                    background: 'var(--bg-tertiary)',
                    borderRadius: '6px',
                    fontSize: '13px',
                    fontWeight: 500,
                }}>
                    Total: {batchResult.total_applied} fixes applied, {batchResult.total_failed} failed
                </div>
            </div>
        )}
    </div>
    );
};

// =============================================================================
// Severity Badge
// =============================================================================

const severityColors: Record<string, string> = {
    critical: '#ef4444',
    high: '#f97316',
    medium: '#eab308',
    low: '#3b82f6',
};

const SeverityBadge: React.FC<{ severity: string }> = ({ severity }) => (
    <span
        style={{
            display: 'inline-block',
            padding: '1px 6px',
            borderRadius: '4px',
            fontSize: '11px',
            fontWeight: 600,
            textTransform: 'uppercase',
            background: `${severityColors[severity] || '#6b7280'}22`,
            color: severityColors[severity] || '#6b7280',
            border: `1px solid ${severityColors[severity] || '#6b7280'}44`,
        }}
    >
        {severity}
    </span>
);
