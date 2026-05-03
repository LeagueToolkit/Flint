import React from 'react';

export interface PanelProps {
    children: React.ReactNode;
    className?: string;
    padded?: boolean;
    style?: React.CSSProperties;
}

/**
 * Generic panel surface. Use `padded` for default inner padding.
 * Wraps a div with `.panel`. The actual look comes from the global
 * stylesheet — components compose modifiers via `className`.
 */
export const Panel: React.FC<PanelProps> = ({ children, className = '', padded = false, style }) => (
    <div className={`panel ${padded ? 'panel--padded' : ''} ${className}`.trim()} style={style}>
        {children}
    </div>
);

export const PanelHeader: React.FC<{ children: React.ReactNode; className?: string }> = ({ children, className = '' }) => (
    <div className={`panel__header ${className}`.trim()}>{children}</div>
);

export const PanelBody: React.FC<{ children: React.ReactNode; className?: string }> = ({ children, className = '' }) => (
    <div className={`panel__body ${className}`.trim()}>{children}</div>
);

export const PanelFooter: React.FC<{ children: React.ReactNode; className?: string }> = ({ children, className = '' }) => (
    <div className={`panel__footer ${className}`.trim()}>{children}</div>
);
