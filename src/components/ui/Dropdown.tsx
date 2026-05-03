import React, { useEffect, useRef, useState } from 'react';

export type DropdownAlign = 'left' | 'right';

export interface DropdownItem {
    label?: React.ReactNode;
    icon?: React.ReactNode;
    onClick?: () => void;
    disabled?: boolean;
    danger?: boolean;
    /** Render a divider instead of an item. */
    divider?: boolean;
}

export interface DropdownProps {
    /** The trigger element. Receives onClick and aria-expanded; usually a Button. */
    trigger: (open: boolean, toggle: () => void) => React.ReactNode;
    items?: DropdownItem[];
    /** Custom menu content, used instead of `items`. */
    children?: React.ReactNode;
    align?: DropdownAlign;
    /** Width applied to the menu (number = px). */
    menuWidth?: number | string;
    className?: string;
    onOpenChange?: (open: boolean) => void;
}

export const Dropdown: React.FC<DropdownProps> = ({
    trigger,
    items,
    children,
    align = 'right',
    menuWidth,
    className = '',
    onOpenChange,
}) => {
    const [open, setOpen] = useState(false);
    const ref = useRef<HTMLDivElement>(null);

    const toggle = () => setOpen((v) => !v);
    const close = () => setOpen(false);

    useEffect(() => {
        onOpenChange?.(open);
    }, [open, onOpenChange]);

    useEffect(() => {
        if (!open) return;
        const onDocClick = (e: MouseEvent) => {
            if (ref.current && !ref.current.contains(e.target as Node)) close();
        };
        const onEsc = (e: KeyboardEvent) => {
            if (e.key === 'Escape') close();
        };
        document.addEventListener('mousedown', onDocClick);
        document.addEventListener('keydown', onEsc);
        return () => {
            document.removeEventListener('mousedown', onDocClick);
            document.removeEventListener('keydown', onEsc);
        };
    }, [open]);

    const menuStyle: React.CSSProperties = {
        ...(align === 'left' ? { left: 0, right: 'auto' } : {}),
        ...(menuWidth != null ? { minWidth: menuWidth, width: menuWidth } : {}),
    };

    return (
        <div ref={ref} className={`dropdown ${open ? 'dropdown--open' : ''} ${className}`.trim()}>
            {trigger(open, toggle)}
            <div className="dropdown__menu" style={menuStyle} role="menu">
                {children ??
                    items?.map((item, i) =>
                        item.divider ? (
                            <div key={`d-${i}`} className="dropdown__divider" />
                        ) : (
                            <button
                                key={i}
                                className="dropdown__item"
                                disabled={item.disabled}
                                role="menuitem"
                                style={item.danger ? { color: 'var(--accent-danger, #f85149)' } : undefined}
                                onClick={() => {
                                    item.onClick?.();
                                    close();
                                }}
                            >
                                {item.icon}
                                <span>{item.label}</span>
                            </button>
                        ),
                    )}
            </div>
        </div>
    );
};
