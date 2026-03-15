/**
 * Flint - Logging Service
 * Captures console.log/warn/error and stores them for the log panel
 * This is initialized BEFORE React mounts to capture early logs
 */

import type { LogEntry } from './types';

// Direct import of the store - no dispatcher pattern needed
let addLogToStore: ((level: LogEntry['level'], message: string) => void) | null = null;

/**
 * Set the store's addLog function (called once when store is ready)
 */
export function setLogStore(addLog: (level: LogEntry['level'], message: string) => void) {
    addLogToStore = addLog;
}

/**
 * Add a log entry directly to the store
 */
function addLogEntry(level: LogEntry['level'], message: string) {
    if (addLogToStore) {
        addLogToStore(level, message);
    }
    // If store not ready yet, just drop the log (rare, only very early logs)
}

// Store original console methods
const originalConsole = {
    log: console.log.bind(console),
    warn: console.warn.bind(console),
    error: console.error.bind(console),
};

/**
 * Format arguments to a string message
 */
function formatArgs(args: unknown[]): string {
    return args.map(arg => {
        if (typeof arg === 'object' && arg !== null) {
            try {
                return JSON.stringify(arg, null, 2);
            } catch {
                return String(arg);
            }
        }
        return String(arg);
    }).join(' ');
}

/**
 * Check if a log message should be filtered out (noisy logs)
 */
function shouldFilter(message: string): boolean {
    const filters = [
        '[HMR]',
        '[vite]',
        'Download the React DevTools',
    ];
    return filters.some(f => message.includes(f));
}

/**
 * Initialize console interception
 * Call this BEFORE React mounts
 */
export function initializeLogger() {
    // Override console.log
    console.log = (...args: unknown[]) => {
        originalConsole.log(...args);
        const message = formatArgs(args);
        if (!shouldFilter(message)) {
            addLogEntry('info', message);
        }
    };

    // Override console.warn
    console.warn = (...args: unknown[]) => {
        originalConsole.warn(...args);
        const message = formatArgs(args);
        addLogEntry('warning', message);
    };

    // Override console.error
    console.error = (...args: unknown[]) => {
        originalConsole.error(...args);
        const message = formatArgs(args);
        addLogEntry('error', message);
    };

    // Log initialization
    addLogEntry('info', '🔥 Flint frontend logger initialized');
}

/**
 * Restore original console methods
 */
export function restoreConsole() {
    console.log = originalConsole.log;
    console.warn = originalConsole.warn;
    console.error = originalConsole.error;
}

/**
 * Initialize Tauri event listener for backend logs
 * Call this after the app is ready
 */
export async function initBackendLogListener() {
    try {
        const { listen } = await import('@tauri-apps/api/event');

        await listen<{ timestamp: number; level: string; target: string; message: string }>(
            'log-event',
            (event) => {
                const { level, message } = event.payload;

                // Map Rust log levels to our levels
                let logLevel: 'info' | 'warning' | 'error' = 'info';
                const levelLower = level.toLowerCase();
                if (levelLower === 'warn' || levelLower === 'warning') {
                    logLevel = 'warning';
                } else if (levelLower === 'error') {
                    logLevel = 'error';
                } else if (levelLower === 'debug') {
                    // Map debug to info for frontend display
                    logLevel = 'info';
                }

                // Format message - include [rust] prefix to distinguish from frontend logs
                const formattedMessage = `[rust] ${message}`;
                addLogEntry(logLevel, formattedMessage);
            }
        );

        originalConsole.log('✓ Backend log listener initialized');
        addLogEntry('info', '✓ Backend log listener connected');
    } catch (error) {
        originalConsole.error('✗ Failed to initialize backend log listener:', error);
        addLogEntry('error', '✗ Failed to initialize backend log listener');
    }
}
