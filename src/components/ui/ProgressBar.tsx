import React from 'react';

export interface ProgressBarProps {
    /** 0–100. Values outside the range are clamped. */
    value: number;
    /** Optional label shown above the bar (left side). */
    label?: React.ReactNode;
    /** Optional value caption shown above the bar (right side). Defaults to `${value}%`. */
    caption?: React.ReactNode;
    /** Hide the header row even when label/caption are provided. */
    hideHeader?: boolean;
    height?: number;
    className?: string;
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
    value,
    label,
    caption,
    hideHeader = false,
    height = 4,
    className = '',
}) => {
    const pct = Math.max(0, Math.min(100, value));
    const showHeader = !hideHeader && (label != null || caption != null);

    return (
        <div className={`progress-bar ${className}`.trim()}>
            {showHeader && (
                <div
                    style={{
                        display: 'flex',
                        justifyContent: 'space-between',
                        marginBottom: 8,
                        fontSize: 13,
                    }}
                >
                    <span>{label}</span>
                    <span>{caption ?? `${Math.round(pct)}%`}</span>
                </div>
            )}
            <div
                style={{
                    height,
                    background: 'var(--bg-tertiary)',
                    borderRadius: height / 2,
                    overflow: 'hidden',
                }}
            >
                <div
                    style={{
                        height: '100%',
                        width: `${pct}%`,
                        background: 'var(--accent-primary)',
                        transition: 'width 0.2s ease',
                    }}
                />
            </div>
        </div>
    );
};
