/**
 * Flint - First Time Setup Modal Component
 */

import React, { useState } from 'react';
import { useAppState } from '../../lib/stores';
import * as api from '../../lib/api';
import { open } from '@tauri-apps/plugin-dialog';
import { Button, Field, FormGroup, FormHint, FormLabel, Input, Modal, ModalBody, ModalFooter, ModalHeader } from '../ui';

export const FirstTimeSetupModal: React.FC = () => {
    const { state, dispatch, closeModal, showToast } = useAppState();

    const [creatorName, setCreatorName] = useState('');
    const [leaguePath, setLeaguePath] = useState(state.leaguePath || '');
    const [isDetecting, setIsDetecting] = useState(false);

    const isVisible = state.activeModal === 'firstTimeSetup';

    const handleDetectLeague = async () => {
        setIsDetecting(true);
        try {
            const result = await api.detectLeague();
            if (result.path) {
                setLeaguePath(result.path);
                showToast('success', 'League installation detected!');
            }
        } catch {
            showToast('warning', 'Could not auto-detect. Please select manually.');
        } finally {
            setIsDetecting(false);
        }
    };

    const handleBrowseLeague = async () => {
        const selected = await open({
            title: 'Select League of Legends Game Folder',
            directory: true,
        });
        if (selected) setLeaguePath(selected as string);
    };

    const handleComplete = async () => {
        if (!creatorName.trim()) {
            showToast('warning', 'Please enter your creator name');
            return;
        }

        if (leaguePath) {
            try {
                const result = await api.validateLeague(leaguePath);
                if (!result.valid) {
                    showToast('error', 'Invalid League of Legends path');
                    return;
                }
            } catch {
                showToast('error', 'Failed to validate League path');
                return;
            }
        }

        dispatch({
            type: 'SET_STATE',
            payload: {
                creatorName: creatorName.trim(),
                leaguePath: leaguePath || null,
            },
        });

        closeModal();
        showToast('success', 'Setup complete! Welcome to Flint.');
    };

    return (
        <Modal open={isVisible} closeOnOverlay={false} closeOnEscape={false}>
            <ModalHeader title="Welcome to Flint!" />
            <ModalBody>
                <p style={{ marginBottom: 16, color: 'var(--text-secondary)' }}>
                    Let's get you set up. This will only take a moment.
                </p>

                <Field
                    label="Your Creator Name"
                    required
                    placeholder="e.g., SirDexal"
                    value={creatorName}
                    onChange={(e) => setCreatorName(e.target.value)}
                    hint="This will be used in your mods for proper crediting."
                />

                <FormGroup>
                    <FormLabel>League of Legends Path</FormLabel>
                    <Input
                        placeholder="C:\Riot Games\League of Legends"
                        value={leaguePath}
                        onChange={(e) => setLeaguePath(e.target.value)}
                        buttonLabel="Browse"
                        onButtonClick={handleBrowseLeague}
                    />
                    <Button
                        variant="ghost"
                        icon="search"
                        style={{ marginTop: 8 }}
                        onClick={handleDetectLeague}
                        disabled={isDetecting}
                    >
                        Auto-detect
                    </Button>
                    <FormHint>Optional — needed for in-game tooling and validation.</FormHint>
                </FormGroup>
            </ModalBody>
            <ModalFooter>
                <Button variant="primary" onClick={handleComplete}>
                    Get Started
                </Button>
            </ModalFooter>
        </Modal>
    );
};
