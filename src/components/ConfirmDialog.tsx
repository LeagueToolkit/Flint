/**
 * Flint - Confirm Dialog Component
 * Custom in-app confirmation dialog (replaces window.confirm)
 */

import React, { useEffect, useRef } from 'react';
import { useAppState } from '../lib/stores';

export const ConfirmDialog: React.FC = () => {
    const { state, closeConfirmDialog } = useAppState();
    const dialogRef = useRef<HTMLDivElement>(null);
    const confirmBtnRef = useRef<HTMLButtonElement>(null);

    const dialog = state.confirmDialog;

    // Focus the confirm button when dialog opens, handle Escape key
    useEffect(() => {
        if (!dialog) return;

        // Small delay to ensure animation has started
        const timer = setTimeout(() => confirmBtnRef.current?.focus(), 50);

        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                closeConfirmDialog();
            }
        };
        document.addEventListener('keydown', handleKeyDown);
        return () => {
            clearTimeout(timer);
            document.removeEventListener('keydown', handleKeyDown);
        };
    }, [dialog, closeConfirmDialog]);

    if (!dialog) return null;

    const handleConfirm = () => {
        dialog.onConfirm();
        closeConfirmDialog();
    };

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === e.currentTarget) {
            closeConfirmDialog();
        }
    };

    return (
        <div className="confirm-overlay" onClick={handleOverlayClick}>
            <div className="confirm-dialog" ref={dialogRef}>
                <div className="confirm-dialog__icon">
                    {dialog.danger ? (
                        <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                            <path d="M12 9v4m0 4h.01" stroke="#F85149" strokeWidth="2" strokeLinecap="round"/>
                            <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" stroke="#F85149" strokeWidth="1.5" fill="none"/>
                        </svg>
                    ) : (
                        <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                            <circle cx="12" cy="12" r="9" stroke="var(--accent-secondary)" strokeWidth="1.5"/>
                            <path d="M12 8v5m0 3h.01" stroke="var(--accent-secondary)" strokeWidth="2" strokeLinecap="round"/>
                        </svg>
                    )}
                </div>
                <div className="confirm-dialog__content">
                    <h3 className="confirm-dialog__title">{dialog.title}</h3>
                    <p className="confirm-dialog__message">{dialog.message}</p>
                </div>
                <div className="confirm-dialog__actions">
                    <button
                        className="btn btn--secondary"
                        onClick={closeConfirmDialog}
                    >
                        {dialog.cancelLabel || 'Cancel'}
                    </button>
                    <button
                        ref={confirmBtnRef}
                        className={`btn ${dialog.danger ? 'btn--danger' : 'btn--primary'}`}
                        onClick={handleConfirm}
                    >
                        {dialog.confirmLabel || 'Confirm'}
                    </button>
                </div>
            </div>
        </div>
    );
};
