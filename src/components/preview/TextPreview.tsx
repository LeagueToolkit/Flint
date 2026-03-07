/**
 * Flint - Text Preview Component with Monaco Editor
 */

import React, { useState, useEffect, useRef } from 'react';
import Editor from '@monaco-editor/react';
import * as api from '../../lib/api';
import { getIcon } from '../../lib/fileIcons';
import { useAppMetadataStore } from '../../lib/stores';

interface TextPreviewProps {
    filePath: string;
}

export const TextPreview: React.FC<TextPreviewProps> = ({ filePath }) => {
    const [content, setContent] = useState<string>('');
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [hasChanges, setHasChanges] = useState(false);
    const [saving, setSaving] = useState(false);
    const originalContentRef = useRef<string>('');

    // Subscribe to file version changes for hot reload
    const fileVersion = useAppMetadataStore((state) => state.fileVersions[filePath] || 0);

    const ext = filePath.split('.').pop()?.toLowerCase() || 'txt';

    // Map file extensions to Monaco language IDs
    const getLanguage = (extension: string): string => {
        const languageMap: Record<string, string> = {
            'json': 'json',
            'js': 'javascript',
            'jsx': 'javascript',
            'ts': 'typescript',
            'tsx': 'typescript',
            'py': 'python',
            'css': 'css',
            'scss': 'scss',
            'html': 'html',
            'xml': 'xml',
            'md': 'markdown',
            'yaml': 'yaml',
            'yml': 'yaml',
            'sh': 'shell',
            'bat': 'bat',
            'txt': 'plaintext',
        };
        return languageMap[extension] || 'plaintext';
    };

    useEffect(() => {
        const loadText = async () => {
            setLoading(true);
            setError(null);
            setHasChanges(false);

            try {
                const text = await api.readTextFile(filePath);
                setContent(text);
                originalContentRef.current = text;
            } catch (err) {
                console.error('[TextPreview] Error:', err);
                setError((err as Error).message || 'Failed to load text');
            } finally {
                setLoading(false);
            }
        };

        loadText();
    }, [filePath, fileVersion]); // Re-run when file version changes (hot reload)

    const handleSave = async () => {
        setSaving(true);
        try {
            await api.writeTextFile(filePath, content);
            originalContentRef.current = content;
            setHasChanges(false);
        } catch (err) {
            console.error('[TextPreview] Save error:', err);
            setError('Failed to save file');
        } finally {
            setSaving(false);
        }
    };

    const handleEditorChange = (value: string | undefined) => {
        if (value !== undefined) {
            setContent(value);
            setHasChanges(value !== originalContentRef.current);
        }
    };

    const handleRevert = () => {
        setContent(originalContentRef.current);
        setHasChanges(false);
    };

    if (loading) {
        return (
            <div className="text-preview__loading">
                <div className="spinner" />
                <span>Loading text...</span>
            </div>
        );
    }

    if (error) {
        return (
            <div className="text-preview__error">
                <span dangerouslySetInnerHTML={{ __html: getIcon('warning') }} />
                <span>{error}</span>
            </div>
        );
    }

    const language = getLanguage(ext);
    const lineCount = content.split('\n').length;

    return (
        <div className="text-preview text-preview--monaco">
            <div className="text-preview__toolbar">
                <div className="text-preview__toolbar-left">
                    <span className="text-preview__lang">{ext.toUpperCase()}</span>
                    <span>{lineCount} lines</span>
                    {hasChanges && <span className="text-preview__modified">● Modified</span>}
                </div>
                <div className="text-preview__toolbar-right">
                    {hasChanges && (
                        <>
                            <button
                                className="btn btn--sm btn--ghost"
                                onClick={handleRevert}
                                disabled={saving}
                            >
                                Revert
                            </button>
                            <button
                                className="btn btn--sm btn--primary"
                                onClick={handleSave}
                                disabled={saving}
                            >
                                {saving ? 'Saving...' : 'Save'}
                            </button>
                        </>
                    )}
                </div>
            </div>
            <div className="text-preview__monaco-container">
                <Editor
                    height="100%"
                    language={language}
                    value={content}
                    onChange={handleEditorChange}
                    theme="vs-dark"
                    options={{
                        minimap: { enabled: false },
                        fontSize: 13,
                        lineHeight: 20,
                        fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
                        scrollBeyondLastLine: false,
                        wordWrap: 'off',
                        automaticLayout: true,
                        tabSize: 2,
                        insertSpaces: true,
                        formatOnPaste: true,
                        formatOnType: true,
                        renderWhitespace: 'selection',
                        bracketPairColorization: {
                            enabled: true,
                        },
                    }}
                />
            </div>
        </div>
    );
};
