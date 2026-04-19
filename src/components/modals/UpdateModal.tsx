import React, { useState } from 'react';
import { useAppState } from '../../lib/stores';
import * as updater from '../../lib/updater';
import type { UpdateInfo } from '../../lib/types';
import { getIcon } from '../../lib/fileIcons';

const escapeHtml = (s: string): string =>
    s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

// Minimal GitHub-flavored markdown → HTML renderer for release notes.
// Handles headings, bold/italic, inline code, links, and simple bullet lists.
const renderReleaseNotes = (md: string): string => {
    const lines = escapeHtml(md).replace(/\r\n/g, '\n').split('\n');
    const out: string[] = [];
    let inList = false;

    const inline = (s: string): string =>
        s
            .replace(/`([^`]+)`/g, '<code>$1</code>')
            .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
            .replace(/(^|[\s(])\*([^*\n]+)\*(?=[\s).,!?]|$)/g, '$1<em>$2</em>')
            .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g,
                '<a href="$2" target="_blank" rel="noreferrer noopener">$1</a>');

    const closeList = () => {
        if (inList) {
            out.push('</ul>');
            inList = false;
        }
    };

    for (const raw of lines) {
        const line = raw.trimEnd();
        const heading = /^(#{1,4})\s+(.*)$/.exec(line);
        const bullet = /^\s*[-*]\s+(.*)$/.exec(line);

        if (heading) {
            closeList();
            const level = Math.min(6, heading[1].length + 2); // ## → h4 etc.
            out.push(`<h${level}>${inline(heading[2])}</h${level}>`);
        } else if (bullet) {
            if (!inList) { out.push('<ul>'); inList = true; }
            out.push(`<li>${inline(bullet[1])}</li>`);
        } else if (line.trim() === '') {
            closeList();
        } else {
            closeList();
            out.push(`<p>${inline(line)}</p>`);
        }
    }
    closeList();
    return out.join('');
};

export const UpdateModal: React.FC = () => {
    const { state, dispatch, closeModal, showToast } = useAppState();
    const [isDownloading, setIsDownloading] = useState(false);
    const [downloadProgress, setDownloadProgress] = useState(0);

    const isVisible = state.activeModal === 'updateAvailable';
    const updateInfo = state.modalOptions as UpdateInfo | null;

    const handleUpdateNow = async () => {
        setIsDownloading(true);
        setDownloadProgress(0);

        try {
            showToast('info', 'Downloading update...', { duration: 0 });

            await updater.downloadAndInstallUpdate((downloaded, total) => {
                if (total > 0) {
                    const progress = Math.round((downloaded / total) * 100);
                    setDownloadProgress(progress);
                }
            });

            // The app will relaunch automatically after successful update
        } catch (err) {
            setIsDownloading(false);
            setDownloadProgress(0);
            const message = err instanceof Error ? err.message : 'Download failed';
            showToast('error', `Update failed: ${message}`);
        }
    };

    const handleSkip = () => {
        // Persist skipped version so we don't ask again
        if (updateInfo?.latest_version) {
            dispatch({
                type: 'SET_STATE',
                payload: { skippedUpdateVersion: updateInfo.latest_version },
            });
        }
        closeModal();
    };

    const handleRemindLater = () => closeModal();

    if (!isVisible || !updateInfo) return null;

    const publishedDate = updateInfo.published_at
        ? new Date(updateInfo.published_at).toLocaleDateString('en-US', {
            year: 'numeric',
            month: 'long',
            day: 'numeric',
        })
        : '';

    return (
        <div className={`modal-overlay ${isVisible ? 'modal-overlay--visible' : ''}`}>
            <div className="modal" style={{ maxWidth: '500px' }}>
                <div className="modal__header">
                    <h2 className="modal__title">
                        <span dangerouslySetInnerHTML={{ __html: getIcon('info') }} />
                        {' '}Update Available
                    </h2>
                    <button className="modal__close" onClick={handleRemindLater}>×</button>
                </div>

                <div className="modal__body">
                    <div className="update-modal__versions" style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        gap: '16px',
                        marginBottom: '20px',
                        padding: '16px',
                        background: 'var(--bg-tertiary)',
                        borderRadius: '8px',
                    }}>
                        <div style={{ textAlign: 'center' }}>
                            <div style={{ color: 'var(--text-secondary)', fontSize: '12px' }}>Current</div>
                            <div style={{ fontSize: '18px', fontWeight: '600' }}>v{updateInfo.current_version}</div>
                        </div>
                        <span dangerouslySetInnerHTML={{ __html: getIcon('chevronRight') }} style={{ opacity: 0.5 }} />
                        <div style={{ textAlign: 'center' }}>
                            <div style={{ color: 'var(--accent-primary)', fontSize: '12px' }}>Latest</div>
                            <div style={{ fontSize: '18px', fontWeight: '600', color: 'var(--accent-primary)' }}>
                                v{updateInfo.latest_version}
                            </div>
                        </div>
                    </div>

                    {publishedDate && (
                        <p style={{ color: 'var(--text-secondary)', fontSize: '13px', marginBottom: '12px' }}>
                            Released on {publishedDate}
                        </p>
                    )}

                    {updateInfo.release_notes && (
                        <div className="form-group">
                            <label className="form-label">What's New</label>
                            <div
                                className="update-modal__release-notes"
                                style={{
                                    maxHeight: '200px',
                                    overflowY: 'auto',
                                    padding: '12px',
                                    background: 'var(--bg-primary)',
                                    borderRadius: '6px',
                                    border: '1px solid var(--border-color)',
                                    fontSize: '13px',
                                    lineHeight: '1.6',
                                }}
                                dangerouslySetInnerHTML={{ __html: renderReleaseNotes(updateInfo.release_notes) }}
                            />
                        </div>
                    )}

                    {isDownloading && (
                        <div style={{ marginTop: '16px' }}>
                            <div style={{
                                display: 'flex',
                                justifyContent: 'space-between',
                                marginBottom: '8px',
                                fontSize: '13px',
                            }}>
                                <span>Downloading update...</span>
                                <span>{downloadProgress}%</span>
                            </div>
                            <div style={{
                                height: '4px',
                                background: 'var(--bg-tertiary)',
                                borderRadius: '2px',
                                overflow: 'hidden',
                            }}>
                                <div style={{
                                    height: '100%',
                                    width: `${downloadProgress}%`,
                                    background: 'var(--accent-primary)',
                                    transition: 'width 0.2s ease',
                                }} />
                            </div>
                        </div>
                    )}
                </div>

                <div className="modal__footer">
                    <button
                        className="btn btn--ghost"
                        onClick={handleSkip}
                        disabled={isDownloading}
                    >
                        Skip This Version
                    </button>
                    <button
                        className="btn btn--secondary"
                        onClick={handleRemindLater}
                        disabled={isDownloading}
                    >
                        Remind Me Later
                    </button>
                    <button
                        className="btn btn--primary"
                        onClick={handleUpdateNow}
                        disabled={isDownloading}
                    >
                        {isDownloading ? 'Downloading...' : 'Update Now'}
                    </button>
                </div>
            </div>
        </div>
    );
};
