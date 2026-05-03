import React, { useState } from 'react';
import { useAppState } from '../../lib/stores';
import * as updater from '../../lib/updater';
import type { UpdateInfo } from '../../lib/types';
import {
    Button,
    FormGroup,
    FormLabel,
    Icon,
    Modal,
    ModalBody,
    ModalFooter,
    ModalHeader,
    ProgressBar,
} from '../ui';

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
            .replace(
                /\[([^\]]+)\]\((https?:[^)\s]+)\)/g,
                '<a href="$2" target="_blank" rel="noreferrer noopener">$1</a>',
            );

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
            const level = Math.min(6, heading[1].length + 2);
            out.push(`<h${level}>${inline(heading[2])}</h${level}>`);
        } else if (bullet) {
            if (!inList) {
                out.push('<ul>');
                inList = true;
            }
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

const VersionPill: React.FC<{ label: string; version: string; highlight?: boolean }> = ({
    label,
    version,
    highlight,
}) => (
    <div style={{ textAlign: 'center' }}>
        <div
            style={{
                color: highlight ? 'var(--accent-primary)' : 'var(--text-secondary)',
                fontSize: 12,
            }}
        >
            {label}
        </div>
        <div
            style={{
                fontSize: 18,
                fontWeight: 600,
                color: highlight ? 'var(--accent-primary)' : undefined,
            }}
        >
            v{version}
        </div>
    </div>
);

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
                    setDownloadProgress(Math.round((downloaded / total) * 100));
                }
            });
            // The app relaunches automatically after a successful update.
        } catch (err) {
            setIsDownloading(false);
            setDownloadProgress(0);
            const message = err instanceof Error ? err.message : 'Download failed';
            showToast('error', `Update failed: ${message}`);
        }
    };

    const handleSkip = () => {
        if (updateInfo?.latest_version) {
            dispatch({
                type: 'SET_STATE',
                payload: { skippedUpdateVersion: updateInfo.latest_version },
            });
        }
        closeModal();
    };

    const handleRemindLater = () => closeModal();

    if (!updateInfo) return null;

    const publishedDate = updateInfo.published_at
        ? new Date(updateInfo.published_at).toLocaleDateString('en-US', {
              year: 'numeric',
              month: 'long',
              day: 'numeric',
          })
        : '';

    return (
        <Modal open={isVisible} onClose={handleRemindLater}>
            <ModalHeader
                title={
                    <>
                        <Icon name="info" /> Update Available
                    </>
                }
                onClose={handleRemindLater}
            />

            <ModalBody>
                <div
                    className="update-modal__versions"
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        gap: 16,
                        marginBottom: 20,
                        padding: 16,
                        background: 'var(--bg-tertiary)',
                        borderRadius: 8,
                    }}
                >
                    <VersionPill label="Current" version={updateInfo.current_version} />
                    <Icon name="chevronRight" style={{ opacity: 0.5 }} />
                    <VersionPill label="Latest" version={updateInfo.latest_version} highlight />
                </div>

                {publishedDate && (
                    <p style={{ color: 'var(--text-secondary)', fontSize: 13, marginBottom: 12 }}>
                        Released on {publishedDate}
                    </p>
                )}

                {updateInfo.release_notes && (
                    <FormGroup>
                        <FormLabel>What's New</FormLabel>
                        <div
                            className="update-modal__release-notes"
                            style={{
                                maxHeight: 200,
                                overflowY: 'auto',
                                padding: 12,
                                background: 'var(--bg-primary)',
                                borderRadius: 6,
                                border: '1px solid var(--border-color)',
                                fontSize: 13,
                                lineHeight: 1.6,
                            }}
                            dangerouslySetInnerHTML={{ __html: renderReleaseNotes(updateInfo.release_notes) }}
                        />
                    </FormGroup>
                )}

                {isDownloading && (
                    <div style={{ marginTop: 16 }}>
                        <ProgressBar value={downloadProgress} label="Downloading update..." />
                    </div>
                )}
            </ModalBody>

            <ModalFooter>
                <Button variant="ghost" onClick={handleSkip} disabled={isDownloading}>
                    Skip This Version
                </Button>
                <Button variant="secondary" onClick={handleRemindLater} disabled={isDownloading}>
                    Remind Me Later
                </Button>
                <Button variant="primary" onClick={handleUpdateNow} disabled={isDownloading}>
                    {isDownloading ? 'Downloading...' : 'Update Now'}
                </Button>
            </ModalFooter>
        </Modal>
    );
};
