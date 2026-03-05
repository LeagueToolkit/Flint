/**
 * Flint - New Project Modal Component
 *
 * Uses DataDragon/CommunityDragon API for champion/skin selection.
 * Supports Skin Projects and Animated Loading Screen projects.
 */

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { useAppState } from '../../lib/state';
import * as api from '../../lib/api';
import * as datadragon from '../../lib/datadragon';
import { appDataDir } from '@tauri-apps/api/path';
import type { DDragonChampion, DDragonSkin } from '../../lib/datadragon';
import type { Project } from '../../lib/types';
import {
    calculateBudget,
    getVideoMetadata,
    generateSpritesheet,
    type VideoMeta,
    type BudgetResult,
} from '../../lib/spritesheet';

type ProjectType = 'skin' | 'loading-screen';

const SCALE_OPTIONS = [
    { label: '100%', value: 1.0 },
    { label: '75%', value: 0.75 },
    { label: '50%', value: 0.5 },
    { label: '25%', value: 0.25 },
];

const FPS_OPTIONS = [15, 24, 30, 60];

export const NewProjectModal: React.FC = () => {
    const { state, dispatch, closeModal, showToast, setWorking, setReady } = useAppState();

    // ─── Shared state ────────────────────────────────────────────────────
    const [projectType, setProjectType] = useState<ProjectType>('skin');
    const [projectName, setProjectName] = useState('');
    const [projectPath, setProjectPath] = useState('');
    const [isCreating, setIsCreating] = useState(false);
    const [progress, setProgress] = useState('');

    // ─── Skin project state ─────────────────────────────────────────────
    const [selectedChampion, setSelectedChampion] = useState<DDragonChampion | null>(null);
    const [selectedSkin, setSelectedSkin] = useState<DDragonSkin | null>(null);
    const [champions, setChampions] = useState<DDragonChampion[]>([]);
    const [skins, setSkins] = useState<DDragonSkin[]>([]);
    const [championSearch, setChampionSearch] = useState('');

    // ─── Loading screen state ────────────────────────────────────────────
    const [videoFile, setVideoFile] = useState<File | null>(null);
    const [videoMeta, setVideoMeta] = useState<VideoMeta | null>(null);
    const [trimStart, setTrimStart] = useState(0);
    const [trimEnd, setTrimEnd] = useState(0);
    const [scaleFactor, setScaleFactor] = useState(0.5);
    const [customFps, setCustomFps] = useState(30);
    const [budget, setBudget] = useState<BudgetResult | null>(null);
    const [previewUrl, setPreviewUrl] = useState<string | null>(null);
    const [isGeneratingPreview, setIsGeneratingPreview] = useState(false);
    const videoPreviewRef = useRef<HTMLVideoElement>(null);
    const videoInputRef = useRef<HTMLInputElement>(null);
    const previewVideoRef = useRef<HTMLVideoElement>(null);

    const isVisible = state.activeModal === 'newProject';

    // ─── Effects ─────────────────────────────────────────────────────────

    useEffect(() => {
        if (isVisible && !projectPath) {
            setDefaultProjectPath();
        }
    }, [isVisible]);

    useEffect(() => {
        if (isVisible) {
            loadChampions();
        }
    }, [isVisible]);

    useEffect(() => {
        if (selectedChampion) {
            loadSkins(selectedChampion.id);
        } else {
            setSkins([]);
            setSelectedSkin(null);
        }
    }, [selectedChampion]);

    // Recalculate budget whenever video params change
    useEffect(() => {
        if (!videoMeta) {
            setBudget(null);
            return;
        }
        const result = calculateBudget({
            videoWidth: videoMeta.width,
            videoHeight: videoMeta.height,
            scaleFactor,
            fps: customFps,
            trimStart,
            trimEnd,
        });
        setBudget(result);
    }, [videoMeta, scaleFactor, customFps, trimStart, trimEnd]);

    // ─── Helpers ─────────────────────────────────────────────────────────

    const setDefaultProjectPath = async () => {
        try {
            const dir = await appDataDir();
            const parts = dir.replace(/\\/g, '/').split('/');
            parts.pop();
            const appData = parts.join('/');
            setProjectPath(`${appData}/RitoShark/Flint/Projects`);
        } catch {
            setProjectPath('C:/Users/Projects/Flint');
        }
    };

    const loadChampions = async () => {
        try {
            setWorking('Loading champions...');
            const result = await datadragon.fetchChampions();
            setChampions(result);
            setReady();
        } catch {
            showToast('error', 'Failed to load champions from DataDragon');
            setReady();
        }
    };

    const loadSkins = async (championId: number) => {
        try {
            setWorking('Loading skins...');
            const result = await datadragon.fetchChampionSkins(championId);
            setSkins(result);
            const baseSkin = result.find(s => s.isBase) || result[0];
            setSelectedSkin(baseSkin);
            setReady();
        } catch {
            setSkins([{ id: 0, name: 'Base', num: 0, isBase: true }]);
            setSelectedSkin({ id: 0, name: 'Base', num: 0, isBase: true });
            setReady();
        }
    };

    const handleBrowsePath = async () => {
        try {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const selected = await open({ title: 'Select Project Location', directory: true });
            if (selected) setProjectPath(selected as string);
        } catch { /* ignore */ }
    };

    // ─── Video file handling ─────────────────────────────────────────────

    const handleVideoSelect = () => {
        videoInputRef.current?.click();
    };

    const onVideoInputChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (!file) return;
        // Reset input so re-selecting the same file triggers onChange again
        e.target.value = '';
        try {
            await loadVideoFile(file);
        } catch (err) {
            showToast('error', 'Failed to load video file');
        }
    };

    const loadVideoFile = async (file: File) => {
        try {
            const meta = await getVideoMetadata(file);
            setVideoFile(file);
            setVideoMeta(meta);
            setTrimStart(0);
            setTrimEnd(meta.duration);
            setCustomFps(Math.min(30, Math.round(meta.fps)));

            // Clean up old preview
            if (previewUrl) { URL.revokeObjectURL(previewUrl); setPreviewUrl(null); }

            // Set video thumbnail
            if (videoPreviewRef.current) {
                videoPreviewRef.current.src = URL.createObjectURL(file);
            }
        } catch (err) {
            showToast('error', 'Failed to read video metadata. Ensure it is a valid MP4 or WebM file.');
        }
    };

    const generatePreview = async () => {
        if (!videoFile || !videoMeta) return;
        setIsGeneratingPreview(true);
        if (previewUrl) { URL.revokeObjectURL(previewUrl); setPreviewUrl(null); }

        try {
            const outW = Math.floor(videoMeta.width * scaleFactor);
            const outH = Math.floor(videoMeta.height * scaleFactor);
            const duration = trimEnd - trimStart;
            const totalFrames = Math.ceil(duration * customFps);

            // Load video
            const video = document.createElement('video');
            video.preload = 'auto';
            video.muted = true;
            const srcUrl = URL.createObjectURL(videoFile);
            await new Promise<void>((resolve, reject) => {
                video.oncanplaythrough = () => resolve();
                video.onerror = () => reject(new Error('Failed to load video'));
                video.src = srcUrl;
            });

            // Canvas for frame capture
            const canvas = document.createElement('canvas');
            canvas.width = outW;
            canvas.height = outH;
            const ctx = canvas.getContext('2d')!;

            // MediaRecorder for WebM output
            const stream = canvas.captureStream(0);
            const recorder = new MediaRecorder(stream, {
                mimeType: 'video/webm;codecs=vp9',
                videoBitsPerSecond: 2_000_000,
            });
            const chunks: Blob[] = [];
            recorder.ondataavailable = (e) => { if (e.data.size > 0) chunks.push(e.data); };

            const done = new Promise<Blob>((resolve) => {
                recorder.onstop = () => resolve(new Blob(chunks, { type: 'video/webm' }));
            });
            recorder.start();

            // Draw frames at target FPS
            const frameInterval = 1 / customFps;
            for (let i = 0; i < totalFrames; i++) {
                const time = Math.min(trimStart + i * frameInterval, video.duration - 0.001);

                await new Promise<void>((resolve, reject) => {
                    const onSeeked = () => { video.removeEventListener('seeked', onSeeked); resolve(); };
                    video.addEventListener('seeked', onSeeked);
                    video.addEventListener('error', () => reject(), { once: true });
                    video.currentTime = time;
                });

                ctx.drawImage(video, 0, 0, outW, outH);
                // Request a frame from the capture stream
                (stream.getVideoTracks()[0] as any).requestFrame?.();

                // Wait one frame interval so MediaRecorder captures at correct timing
                await new Promise(r => setTimeout(r, frameInterval * 1000));
            }

            recorder.stop();
            const blob = await done;
            URL.revokeObjectURL(srcUrl);

            const url = URL.createObjectURL(blob);
            setPreviewUrl(url);

            // Auto-play in the preview element
            if (previewVideoRef.current) {
                previewVideoRef.current.src = url;
                previewVideoRef.current.play().catch(() => {});
            }
        } catch (err) {
            showToast('error', 'Failed to generate preview');
        } finally {
            setIsGeneratingPreview(false);
        }
    };

    const handleVideoDrop = useCallback(async (e: React.DragEvent) => {
        e.preventDefault();
        const file = e.dataTransfer.files[0];
        if (file && file.type.startsWith('video/')) {
            await loadVideoFile(file);
        }
    }, []);

    // ─── Create handlers ─────────────────────────────────────────────────

    const handleCreateSkin = async () => {
        if (!projectName || !projectPath || !selectedChampion || !selectedSkin || !state.leaguePath) {
            showToast('error', 'Please fill in all required fields');
            return;
        }

        setIsCreating(true);
        setProgress('Creating project...');

        try {
            const project = await api.createProject({
                name: projectName,
                champion: selectedChampion.alias,
                skin: selectedSkin.num,
                projectPath,
                leaguePath: state.leaguePath,
                creatorName: state.creatorName || undefined,
            });

            await finishProjectCreation(project, selectedChampion.name, selectedSkin.num);
        } catch (err) {
            const flintError = err as api.FlintError;
            showToast('error', flintError.getUserMessage?.() || 'Failed to create project');
        } finally {
            setIsCreating(false);
            setProgress('');
        }
    };

    const handleCreateLoadingScreen = async () => {
        if (!projectName || !projectPath || !videoFile || !budget?.fits || !budget.grid || !state.leaguePath) {
            showToast('error', 'Please fill in all required fields and ensure spritesheet fits within 16k limit');
            return;
        }

        setIsCreating(true);

        try {
            // Phase 1: Generate spritesheet (browser-side)
            setProgress('Extracting video frames...');
            const blob = await generateSpritesheet({
                file: videoFile,
                trimStart,
                trimEnd,
                scaleFactor,
                fps: customFps,
                grid: budget.grid,
                frameW: budget.frameW,
                frameH: budget.frameH,
                onProgress: (cur, total) => setProgress(`Extracting frame ${cur}/${total}...`),
            });

            // Phase 2: Call Rust backend with PNG bytes
            setProgress('Encoding spritesheet & injecting config...');
            const arrayBuf = await blob.arrayBuffer();
            const pngBytes = Array.from(new Uint8Array(arrayBuf));
            const project = await api.createLoadingScreenProject({
                name: projectName,
                projectPath,
                leaguePath: state.leaguePath,
                creatorName: state.creatorName || 'SirDexal',
                spritesheetPngData: pngBytes,
                frameWidth: budget.frameW,
                frameHeight: budget.frameH,
                sheetWidth: budget.grid.sheetWidth,
                sheetHeight: budget.grid.sheetHeight,
                fps: customFps,
                totalFrames: budget.totalFrames,
                cols: budget.grid.cols,
                rows: budget.grid.rows,
            });

            await finishProjectCreation(project, 'Loading Screen', 0);
        } catch (err) {
            const flintError = err as api.FlintError;
            showToast('error', flintError.getUserMessage?.() || 'Failed to create loading screen project');
        } finally {
            setIsCreating(false);
            setProgress('');
        }
    };

    const handleCreate = () => {
        if (projectType === 'skin') return handleCreateSkin();
        return handleCreateLoadingScreen();
    };

    const finishProjectCreation = async (project: Project, championName: string, skinNum: number) => {
        setProgress('Opening project...');

        const projectDir = project.project_path || projectPath;
        dispatch({ type: 'SET_PROJECT', payload: { project, path: projectDir } });

        const files = await api.listProjectFiles(projectDir);
        dispatch({ type: 'SET_FILE_TREE', payload: files });

        const recent = state.recentProjects.filter(p => p.path !== projectDir);
        recent.unshift({
            name: project.display_name || project.name,
            champion: championName,
            skin: skinNum,
            path: projectDir,
            lastOpened: new Date().toISOString(),
        });
        dispatch({ type: 'SET_RECENT_PROJECTS', payload: recent.slice(0, 10) });

        closeModal();
        showToast('success', 'Project created successfully!');

        api.createCheckpoint(projectDir, 'Initial Project State').catch(() => {});
    };

    // ─── Computed values ─────────────────────────────────────────────────

    const filteredChampions = championSearch
        ? champions.filter(c => c.name.toLowerCase().includes(championSearch.toLowerCase()))
        : champions;

    const canCreateSkin = projectType === 'skin'
        && !!projectName && !!projectPath && !!selectedChampion && !!selectedSkin && !isCreating;

    const canCreateLoadingScreen = projectType === 'loading-screen'
        && !!projectName && !!projectPath && !!videoFile && !!budget?.fits && !isCreating;

    const canCreate = canCreateSkin || canCreateLoadingScreen;

    // ─── Budget display helper ───────────────────────────────────────────

    const budgetMaxDim = budget ? Math.max(budget.grid?.sheetWidth ?? 0, budget.grid?.sheetHeight ?? 0) : 0;
    const budgetPercent = Math.min(100, (budgetMaxDim / 16384) * 100);

    if (!isVisible) return null;

    return (
        <div className={`modal-overlay ${isVisible ? 'modal-overlay--visible' : ''}`}>
            <div className="modal modal--wide">
                {isCreating && (
                    <div className="modal__loading-overlay">
                        <div className="modal__loading-content">
                            <div className="spinner spinner--lg" />
                            <div className="modal__loading-text">Creating Project</div>
                            <div className="modal__loading-progress">{progress}</div>
                        </div>
                    </div>
                )}

                <div className="modal__header">
                    <h2 className="modal__title">Create New Project</h2>
                    <button className="modal__close" onClick={closeModal}>&times;</button>
                </div>

                <div className="modal__body">
                    {/* Project Type Selector */}
                    <div className="form-group">
                        <label className="form-label">Project Type</label>
                        <div className="project-type-selector">
                            <button
                                className={`project-type-card${projectType === 'skin' ? ' project-type-card--selected' : ''}`}
                                onClick={() => setProjectType('skin')}
                            >
                                <div className="project-type-card__icon">
                                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                                        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z" stroke="currentColor" strokeWidth="1.5" fill="none"/>
                                        <path d="M7 12.5a1.5 1.5 0 110-3 1.5 1.5 0 010 3zM10 8.5a1.5 1.5 0 110-3 1.5 1.5 0 010 3zM14 8.5a1.5 1.5 0 110-3 1.5 1.5 0 010 3zM17 12.5a1.5 1.5 0 110-3 1.5 1.5 0 010 3z" fill="currentColor"/>
                                        <path d="M16.36 14.64a3 3 0 01-2.83 2.36c-.55 0-1-.45-1-1v-1a1 1 0 00-1-1h-1a1 1 0 00-1 1v1c0 .55-.45 1-1 1a3 3 0 01-2.83-2.36" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" fill="none"/>
                                    </svg>
                                </div>
                                <div className="project-type-card__text">
                                    <span className="project-type-card__title">Skin Project</span>
                                    <span className="project-type-card__desc">Modify champion skins, textures, and models</span>
                                </div>
                            </button>

                            <button
                                className={`project-type-card${projectType === 'loading-screen' ? ' project-type-card--selected' : ''}`}
                                onClick={() => setProjectType('loading-screen')}
                            >
                                <div className="project-type-card__icon">
                                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                                        <rect x="3" y="5" width="18" height="14" rx="2" stroke="currentColor" strokeWidth="1.5" fill="none"/>
                                        <polygon points="10,9 10,15 15,12" fill="currentColor"/>
                                    </svg>
                                </div>
                                <div className="project-type-card__text">
                                    <span className="project-type-card__title">Animated Loading Screen</span>
                                    <span className="project-type-card__desc">Create custom animated loading screens</span>
                                </div>
                            </button>
                        </div>
                    </div>

                    {/* ════════════ Skin Project Form ════════════ */}
                    <div className={`project-type-form${projectType === 'skin' ? ' project-type-form--active' : ''}`}>
                        <div className="form-group">
                            <label className="form-label">Project Name</label>
                            <input
                                type="text"
                                className="form-input"
                                placeholder="e.g., Ahri Base Rework"
                                value={projectName}
                                onChange={(e) => setProjectName(e.target.value)}
                            />
                        </div>

                        <div className="form-group">
                            <label className="form-label">Project Location</label>
                            <div className="form-input--with-button">
                                <input
                                    type="text"
                                    className="form-input"
                                    placeholder="Select folder..."
                                    value={projectPath}
                                    onChange={(e) => setProjectPath(e.target.value)}
                                />
                                <button className="btn btn--secondary" onClick={handleBrowsePath}>Browse</button>
                            </div>
                        </div>

                        <div className="form-group">
                            <label className="form-label">Champion</label>
                            <input
                                type="text"
                                className="form-input form-input--search"
                                placeholder="Search champions..."
                                value={championSearch}
                                onChange={(e) => setChampionSearch(e.target.value)}
                            />
                            <div className="champion-grid">
                                {filteredChampions.map((champ) => (
                                    <div
                                        key={champ.id}
                                        className={`champion-card ${selectedChampion?.id === champ.id ? 'champion-card--selected' : ''}`}
                                        onClick={() => { setSelectedChampion(champ); setChampionSearch(''); }}
                                        title={champ.name}
                                    >
                                        <img
                                            src={datadragon.getChampionIconUrl(champ.id)}
                                            alt={champ.name}
                                            className="champion-card__icon"
                                            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
                                        />
                                        <span className="champion-card__name">{champ.name}</span>
                                    </div>
                                ))}
                            </div>
                            {selectedChampion && (
                                <div className="form-hint">Selected: {selectedChampion.name}</div>
                            )}
                        </div>

                        {selectedChampion && (
                            <div className="form-group">
                                <label className="form-label">Skin</label>
                                <div className="skin-grid">
                                    {skins.map((skin) => (
                                        <div
                                            key={skin.id}
                                            className={`skin-card ${selectedSkin?.id === skin.id ? 'skin-card--selected' : ''}`}
                                            onClick={() => setSelectedSkin(skin)}
                                        >
                                            <img
                                                src={datadragon.getSkinSplashUrl(selectedChampion.alias, skin.num)}
                                                alt={skin.name}
                                                className="skin-card__splash"
                                                onError={(e) => {
                                                    (e.target as HTMLImageElement).src =
                                                        datadragon.getSkinSplashCDragonUrl(selectedChampion.id, skin.id);
                                                }}
                                            />
                                            <span className="skin-card__name">{skin.name}</span>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        )}
                    </div>

                    {/* ════════════ Loading Screen Form ════════════ */}
                    <div className={`project-type-form${projectType === 'loading-screen' ? ' project-type-form--active' : ''}`}>
                        {/* Project Name & Location */}
                        <div className="form-group">
                            <label className="form-label">Project Name</label>
                            <input
                                type="text"
                                className="form-input"
                                placeholder="e.g., My Animated Loadscreen"
                                value={projectName}
                                onChange={(e) => setProjectName(e.target.value)}
                            />
                        </div>

                        <div className="form-group">
                            <label className="form-label">Project Location</label>
                            <div className="form-input--with-button">
                                <input
                                    type="text"
                                    className="form-input"
                                    placeholder="Select folder..."
                                    value={projectPath}
                                    onChange={(e) => setProjectPath(e.target.value)}
                                />
                                <button className="btn btn--secondary" onClick={handleBrowsePath}>Browse</button>
                            </div>
                        </div>

                        {/* Video Selection */}
                        <input
                            ref={videoInputRef}
                            type="file"
                            accept="video/mp4,video/webm,video/quicktime,.mp4,.webm,.mov,.avi,.mkv"
                            style={{ display: 'none' }}
                            onChange={onVideoInputChange}
                        />
                        <div className="form-group">
                            <label className="form-label">Video File</label>
                            {!videoFile ? (
                                <div
                                    className="video-picker"
                                    onClick={handleVideoSelect}
                                    onDragOver={(e) => e.preventDefault()}
                                    onDrop={handleVideoDrop}
                                >
                                    <svg width="32" height="32" viewBox="0 0 24 24" fill="none">
                                        <rect x="3" y="5" width="18" height="14" rx="2" stroke="currentColor" strokeWidth="1.5" fill="none"/>
                                        <polygon points="10,9 10,15 15,12" fill="currentColor"/>
                                    </svg>
                                    <span className="video-picker__text">Click to select or drag & drop a video file</span>
                                    <span className="video-picker__hint">Supports MP4, WebM, MOV</span>
                                </div>
                            ) : (
                                <div className="video-info">
                                    <div className="video-info__preview">
                                        <video
                                            ref={videoPreviewRef}
                                            muted
                                            playsInline
                                            className="video-info__video"
                                        />
                                    </div>
                                    <div className="video-info__meta">
                                        <div className="video-info__name">{videoFile.name}</div>
                                        <div className="video-info__details">
                                            {videoMeta && (
                                                <>
                                                    <span>{videoMeta.width}&times;{videoMeta.height}</span>
                                                    <span>{videoMeta.duration.toFixed(1)}s</span>
                                                </>
                                            )}
                                        </div>
                                        {/* Inline trim controls */}
                                        {videoMeta && (
                                            <div className="trim-controls--inline">
                                                <div className="trim-controls__inputs">
                                                    <label className="trim-controls__label">
                                                        Start
                                                        <input
                                                            type="number"
                                                            className="form-input form-input--sm"
                                                            min={0}
                                                            max={trimEnd - 0.1}
                                                            step={0.1}
                                                            value={trimStart.toFixed(1)}
                                                            onChange={(e) => setTrimStart(Math.max(0, parseFloat(e.target.value) || 0))}
                                                        />
                                                        <span className="trim-controls__unit">s</span>
                                                    </label>
                                                    <label className="trim-controls__label">
                                                        End
                                                        <input
                                                            type="number"
                                                            className="form-input form-input--sm"
                                                            min={trimStart + 0.1}
                                                            max={videoMeta.duration}
                                                            step={0.1}
                                                            value={trimEnd.toFixed(1)}
                                                            onChange={(e) => setTrimEnd(Math.min(videoMeta.duration, parseFloat(e.target.value) || 0))}
                                                        />
                                                        <span className="trim-controls__unit">s</span>
                                                    </label>
                                                    <span className="trim-controls__duration">
                                                        = {(trimEnd - trimStart).toFixed(1)}s
                                                    </span>
                                                </div>
                                            </div>
                                        )}
                                        <div className="video-info__actions">
                                            <button
                                                className="btn btn--secondary btn--sm"
                                                onClick={() => {
                                                    if (previewUrl) { URL.revokeObjectURL(previewUrl); setPreviewUrl(null); }
                                                    setVideoFile(null); setVideoMeta(null); setBudget(null);
                                                }}
                                            >
                                                Change Video
                                            </button>
                                            <button
                                                className="btn btn--primary btn--sm"
                                                onClick={generatePreview}
                                                disabled={isGeneratingPreview || !budget?.fits}
                                                title="Generate a preview with current settings"
                                            >
                                                {isGeneratingPreview ? 'Generating...' : 'Preview'}
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            )}
                            {/* Preview player */}
                            {previewUrl && (
                                <div className="video-preview-player">
                                    <video
                                        ref={previewVideoRef}
                                        src={previewUrl}
                                        muted
                                        loop
                                        autoPlay
                                        playsInline
                                        controls
                                        className="video-preview-player__video"
                                    />
                                </div>
                            )}
                        </div>

                        {/* Editing Controls (only shown when video is loaded) */}
                        {videoMeta && (
                            <>
                                <div className="form-row">
                                    <div className="form-group form-group--half">
                                        <label className="form-label">Resolution</label>
                                        <select
                                            className="form-input"
                                            value={scaleFactor}
                                            onChange={(e) => setScaleFactor(parseFloat(e.target.value))}
                                        >
                                            {SCALE_OPTIONS.map(opt => (
                                                <option key={opt.value} value={opt.value}>
                                                    {opt.label} ({Math.floor(videoMeta.width * opt.value)}&times;{Math.floor(videoMeta.height * opt.value)})
                                                </option>
                                            ))}
                                        </select>
                                    </div>

                                    <div className="form-group form-group--half">
                                        <label className="form-label">FPS</label>
                                        <select
                                            className="form-input"
                                            value={customFps}
                                            onChange={(e) => setCustomFps(parseInt(e.target.value, 10))}
                                        >
                                            {FPS_OPTIONS.map(fps => (
                                                <option key={fps} value={fps}>{fps} fps</option>
                                            ))}
                                        </select>
                                    </div>
                                </div>

                                {/* Budget Indicator */}
                                <div className="form-group">
                                    <label className="form-label">Spritesheet Budget</label>
                                    <div className={`budget-indicator ${budget?.fits ? 'budget-indicator--ok' : 'budget-indicator--exceeded'}`}>
                                        {budget && (
                                            <>
                                                <div className="budget-indicator__summary">
                                                    <span>{budget.totalFrames} frames</span>
                                                    {budget.grid && (
                                                        <>
                                                            <span>{budget.grid.cols}&times;{budget.grid.rows} grid</span>
                                                            <span>{budget.grid.sheetWidth}&times;{budget.grid.sheetHeight} px</span>
                                                        </>
                                                    )}
                                                </div>
                                                <div className="budget-indicator__bar-container">
                                                    <div className="budget-indicator__bar">
                                                        <div
                                                            className="budget-indicator__fill"
                                                            style={{ width: `${Math.min(100, budgetPercent)}%` }}
                                                        />
                                                    </div>
                                                    <span className="budget-indicator__label">
                                                        {budgetMaxDim.toLocaleString()} / 16,384
                                                    </span>
                                                </div>
                                                {!budget.fits && (
                                                    <div className="budget-indicator__warning">
                                                        Exceeds 16,384 pixel limit.
                                                        {budget.suggestedFrameCounts.length > 0 && (
                                                            <> Try reducing resolution or duration (suggested frame counts: {budget.suggestedFrameCounts.slice(0, 3).join(', ')})</>
                                                        )}
                                                    </div>
                                                )}
                                                {budget.fits && (
                                                    <div className="budget-indicator__ok">
                                                        Fits within texture limit
                                                    </div>
                                                )}
                                            </>
                                        )}
                                    </div>
                                </div>
                            </>
                        )}
                    </div>
                </div>

                <div className="modal__footer modal__footer--split">
                    <button
                        className="btn btn--secondary"
                        onClick={() => showToast('info', 'Launcher sync coming soon!')}
                        title="Sync project with launcher"
                    >
                        <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M13.65 2.35A7 7 0 1014.25 8h-1.5a5.5 5.5 0 11-1.1-3.4l1.1 1.1.9-2.35z" fill="currentColor"/></svg>
                        Sync with Launcher
                    </button>
                    <div className="modal__footer-actions">
                        <button className="btn btn--secondary" onClick={closeModal}>Cancel</button>
                        <button
                            className="btn btn--primary"
                            onClick={handleCreate}
                            disabled={!canCreate}
                        >
                            Create Project
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
};
