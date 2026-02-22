import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
    plugins: [react()],
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
    },
    envPrefix: ['VITE_', 'TAURI_'],
    build: {
        target: ['es2021', 'chrome100', 'safari13'],
        minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
        sourcemap: !!process.env.TAURI_DEBUG,

        // Code splitting for better caching and faster initial load
        rollupOptions: {
            output: {
                manualChunks: (id) => {
                    // Monaco editor workers MUST stay in main bundle for blob: URL worker initialization
                    if (id.includes('monaco-editor') && id.includes('worker')) {
                        return undefined; // Keep in main bundle
                    }
                    // React vendor chunk
                    if (id.includes('node_modules/react') || id.includes('node_modules/react-dom')) {
                        return 'react-vendor';
                    }
                    // Monaco Editor UI (not workers) - lazy loaded
                    if (id.includes('@monaco-editor/react') || id.includes('monaco-editor')) {
                        return 'monaco';
                    }
                    // Three.js 3D rendering - lazy loaded
                    if (id.includes('three') || id.includes('@react-three')) {
                        return 'three';
                    }
                    // Tauri APIs
                    if (id.includes('@tauri-apps')) {
                        return 'tauri-apis';
                    }
                },
            },
        },

        // Increase chunk size warning limit after optimization
        chunkSizeWarningLimit: 600,
    },
    resolve: {
        alias: {
            '@': path.resolve(__dirname, './src'),
        },
    },

    // Optimize dependency pre-bundling
    optimizeDeps: {
        include: ['react', 'react-dom', 'monaco-editor'],
    },
});
