/**
 * Flint - Context Menu Component
 */

import React, { useEffect, useRef } from 'react';
import { useModalStore } from '../lib/stores';

export const ContextMenu: React.FC = () => {
    const menu = useModalStore((s) => s.contextMenu);
    const closeContextMenu = useModalStore((s) => s.closeContextMenu);
    const menuRef = useRef<HTMLDivElement>(null);

    const isVisible = !!menu;

    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
                closeContextMenu();
            }
        };

        if (isVisible) {
            document.addEventListener('mousedown', handleClickOutside);
        }

        return () => {
            document.removeEventListener('mousedown', handleClickOutside);
        };
    }, [isVisible, closeContextMenu]);

    if (!isVisible || !menu) return null;

    // Adjust position to keep menu inside window
    const style: React.CSSProperties = {
        position: 'absolute',
        top: menu.y,
        left: menu.x,
        zIndex: 2000,
    };

    return (
        <div
            className="context-menu"
            ref={menuRef}
            style={style}
            onClick={(e) => e.stopPropagation()}
        >
            {menu.options.map((option, index) => (
                <React.Fragment key={index}>
                    {option.separator && index > 0 && (
                        <div className="context-menu__separator" />
                    )}
                    <div
                        className={`context-menu__item${option.danger ? ' context-menu__item--danger' : ''}${option.disabled ? ' context-menu__item--disabled' : ''}`}
                        onClick={() => {
                            if (option.disabled) return;
                            option.onClick();
                            closeContextMenu();
                        }}
                    >
                        {option.icon && (
                            <span
                                className="context-menu__icon"
                                dangerouslySetInnerHTML={{ __html: option.icon }}
                            />
                        )}
                        <span className="context-menu__label">{option.label}</span>
                    </div>
                </React.Fragment>
            ))}
        </div>
    );
};
