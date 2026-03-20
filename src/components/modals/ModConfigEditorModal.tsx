/**
 * Flint - Mod Config Editor Modal
 * Provides a form-based editor for mod.config.json fields
 */

import React, { useState, useEffect, useCallback } from 'react';
import { useAppState } from '../../lib/stores';
import * as api from '../../lib/api';

interface ModConfig {
    name: string;
    display_name: string;
    version: string;
    description: string;
    authors: ModConfigAuthor[];
    license: unknown;
    transformers: unknown[];
    layers: unknown[];
    thumbnail: string | null;
    [key: string]: unknown;
}

interface ModConfigAuthor {
    Name?: string;
    NameAndRole?: { name: string; role: string };
}

function getAuthorName(author: ModConfigAuthor): string {
    if (typeof author === 'string') return author;
    if (author.Name) return author.Name;
    if (author.NameAndRole) return author.NameAndRole.name;
    return '';
}

function getAuthorRole(author: ModConfigAuthor): string {
    if (author.NameAndRole) return author.NameAndRole.role;
    return '';
}

function buildAuthor(name: string, role: string): ModConfigAuthor {
    if (role.trim()) {
        return { NameAndRole: { name, role } };
    }
    return { Name: name };
}

export const ModConfigEditorModal: React.FC = () => {
    const { state, closeModal, showToast } = useAppState();

    const isVisible = state.activeModal === 'modConfig';
    const options = state.modalOptions as { filePath: string } | null;

    const [config, setConfig] = useState<ModConfig | null>(null);
    const [displayName, setDisplayName] = useState('');
    const [version, setVersion] = useState('');
    const [description, setDescription] = useState('');
    const [authors, setAuthors] = useState<{ name: string; role: string }[]>([]);
    const [dirty, setDirty] = useState(false);

    // Load mod.config.json when modal opens
    useEffect(() => {
        if (!isVisible || !options?.filePath) return;

        const loadConfig = async () => {
            try {
                const text = await api.readTextFile(options.filePath);
                const parsed = JSON.parse(text) as ModConfig;
                setConfig(parsed);
                setDisplayName(parsed.display_name || '');
                setVersion(parsed.version || '');
                setDescription(parsed.description || '');
                setAuthors(
                    (parsed.authors || []).map((a: ModConfigAuthor) => ({
                        name: getAuthorName(a),
                        role: getAuthorRole(a),
                    }))
                );
                setDirty(false);
            } catch (err) {
                console.error('Failed to load mod.config.json:', err);
                showToast('error', 'Failed to load mod.config.json');
                closeModal();
            }
        };

        loadConfig();
    }, [isVisible, options?.filePath, showToast, closeModal]);

    const handleSave = useCallback(async () => {
        if (!config || !options?.filePath) return;

        try {
            const updated: ModConfig = {
                ...config,
                display_name: displayName,
                version,
                description,
                authors: authors
                    .filter(a => a.name.trim())
                    .map(a => buildAuthor(a.name.trim(), a.role.trim())),
            };

            const text = JSON.stringify(updated, null, 2);
            await api.writeTextFile(options.filePath, text);
            showToast('success', 'Project config saved');
            setDirty(false);
            closeModal();
        } catch (err) {
            console.error('Failed to save mod.config.json:', err);
            showToast('error', 'Failed to save project config');
        }
    }, [config, options?.filePath, displayName, version, description, authors, showToast, closeModal]);

    const addAuthor = useCallback(() => {
        setAuthors(prev => [...prev, { name: '', role: '' }]);
        setDirty(true);
    }, []);

    const removeAuthor = useCallback((index: number) => {
        setAuthors(prev => prev.filter((_, i) => i !== index));
        setDirty(true);
    }, []);

    const updateAuthor = useCallback((index: number, field: 'name' | 'role', value: string) => {
        setAuthors(prev => prev.map((a, i) => (i === index ? { ...a, [field]: value } : a)));
        setDirty(true);
    }, []);

    if (!isVisible) return null;

    return (
        <div className={`modal-overlay ${isVisible ? 'modal-overlay--visible' : ''}`}>
            <div className="modal" style={{ width: '480px' }}>
                <div className="modal__header">
                    <h2 className="modal__title">Edit Project Info</h2>
                    <button className="modal__close" onClick={closeModal}>&times;</button>
                </div>

                <div className="modal__body" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                    {/* Display Name */}
                    <div className="form-group">
                        <label className="form-label">Display Name</label>
                        <input
                            className="form-input"
                            type="text"
                            value={displayName}
                            onChange={e => { setDisplayName(e.target.value); setDirty(true); }}
                            placeholder="My Awesome Mod"
                        />
                    </div>

                    {/* Version */}
                    <div className="form-group">
                        <label className="form-label">Version</label>
                        <input
                            className="form-input"
                            type="text"
                            value={version}
                            onChange={e => { setVersion(e.target.value); setDirty(true); }}
                            placeholder="1.0.0"
                        />
                    </div>

                    {/* Description */}
                    <div className="form-group">
                        <label className="form-label">Description</label>
                        <textarea
                            className="form-input"
                            value={description}
                            onChange={e => { setDescription(e.target.value); setDirty(true); }}
                            placeholder="A brief description of your mod"
                            rows={3}
                            style={{ resize: 'vertical' }}
                        />
                    </div>

                    {/* Authors */}
                    <div className="form-group">
                        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                            <label className="form-label" style={{ margin: 0 }}>Contributors</label>
                            <button className="btn btn--sm" onClick={addAuthor}>+ Add</button>
                        </div>
                        {authors.length === 0 && (
                            <div style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-sm)', padding: '8px 0' }}>
                                No contributors added yet
                            </div>
                        )}
                        {authors.map((author, index) => (
                            <div key={index} style={{ display: 'flex', gap: '6px', marginBottom: '6px', alignItems: 'center' }}>
                                <input
                                    className="form-input"
                                    type="text"
                                    value={author.name}
                                    onChange={e => updateAuthor(index, 'name', e.target.value)}
                                    placeholder="Name"
                                    style={{ flex: 2 }}
                                />
                                <input
                                    className="form-input"
                                    type="text"
                                    value={author.role}
                                    onChange={e => updateAuthor(index, 'role', e.target.value)}
                                    placeholder="Role (optional)"
                                    style={{ flex: 1 }}
                                />
                                <button
                                    className="btn btn--sm"
                                    onClick={() => removeAuthor(index)}
                                    title="Remove contributor"
                                    style={{ color: 'var(--error)', flexShrink: 0 }}
                                >
                                    &times;
                                </button>
                            </div>
                        ))}
                    </div>
                </div>

                <div className="modal__footer">
                    <button className="btn btn--secondary" onClick={closeModal}>Cancel</button>
                    <button className="btn btn--primary" onClick={handleSave} disabled={!dirty}>Save</button>
                </div>
            </div>
        </div>
    );
};
