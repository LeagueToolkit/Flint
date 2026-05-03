/**
 * Flint - League of Legends Modding IDE
 * React Entry Point
 */

import React from 'react';
import { createRoot } from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { initializeLogger, initBackendLogListener } from './lib/logger';
import { installButtonGlow } from './lib/buttonGlow';
import { AppProvider } from './lib/stores';
import { App } from './components/App';
import { DesignLab } from './components/ui/DesignLab';

// Import styles
import './styles/index.css';
// Modernized component layer — must load AFTER index.css to override
import './styles/ui-primitives.css';
// Settings polish + Picker styles — load after primitives
import './styles/settings-polish.css';
// Fixer modal polish — load after settings
import './styles/fixer-polish.css';
// Project list modal polish
import './styles/project-list-polish.css';
// New project modal polish
import './styles/new-project-polish.css';
// Logger / status-bar polish
import './styles/logger-polish.css';
// Import default theme (can be swapped via custom theme import)
import './themes/default.css';

// Hash bypass: opening with #design-lab loads the new-design showcase
// without booting any app state. Live-reloads via Vite the same as the app.
const isDesignLab =
    typeof window !== 'undefined' &&
    (window.location.hash === '#design-lab' || window.location.search.includes('lab'));

// Initialize logger BEFORE React mounts to capture early logs
initializeLogger();
// Cursor-following glow on .btn — delegated, zero per-button overhead
installButtonGlow();

// Initialize app
const container = document.getElementById('app');
if (!container) {
    throw new Error('[Flint] Could not find #app element');
}

// Remove loading screen
const loadingScreen = document.getElementById('loading-screen');
if (loadingScreen) {
    loadingScreen.remove();
}

const root = createRoot(container);
root.render(
    isDesignLab
        ? React.createElement(React.StrictMode, null, React.createElement(DesignLab))
        : React.createElement(
              React.StrictMode,
              null,
              React.createElement(AppProvider, null, React.createElement(App))
          )
);

// Show window after React has mounted and painted
// Use requestAnimationFrame to ensure the DOM is ready
requestAnimationFrame(() => {
    requestAnimationFrame(() => {
        getCurrentWindow()
            .show()
            .then(() => {
                console.log(isDesignLab ? '[Flint] Design Lab mounted' : '[Flint] Window shown successfully');
                // Initialize backend log listener after window is ready (skip in lab mode)
                if (!isDesignLab) initBackendLogListener();
            })
            .catch((err) => {
                console.error('[Flint] Failed to show window:', err);
            });
    });
});
