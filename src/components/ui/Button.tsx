import React from 'react';
import { Icon, type IconName } from './Icon';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';
export type ButtonSize = 'sm' | 'md' | 'lg';

export interface ButtonProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'children'> {
    variant?: ButtonVariant;
    size?: ButtonSize;
    icon?: IconName;
    iconRight?: IconName;
    iconOnly?: boolean;
    active?: boolean;
    fullWidth?: boolean;
    children?: React.ReactNode;
}

const variantClass: Record<ButtonVariant, string> = {
    primary: 'btn--primary',
    secondary: 'btn--secondary',
    ghost: 'btn--ghost',
    danger: 'btn--danger',
};

const sizeClass: Record<ButtonSize, string> = {
    sm: 'btn--sm',
    md: '',
    lg: 'btn--large',
};

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
    (
        {
            variant = 'secondary',
            size = 'md',
            icon,
            iconRight,
            iconOnly = false,
            active = false,
            fullWidth = false,
            className = '',
            style,
            children,
            type = 'button',
            ...rest
        },
        ref,
    ) => {
        const classes = [
            'btn',
            variantClass[variant],
            sizeClass[size],
            iconOnly ? 'btn--icon' : '',
            active ? 'btn--active' : '',
            className,
        ]
            .filter(Boolean)
            .join(' ');

        const mergedStyle = fullWidth ? { width: '100%', ...style } : style;

        return (
            <button ref={ref} type={type} className={classes} style={mergedStyle} {...rest}>
                {icon && <Icon name={icon} />}
                {children != null && <span>{children}</span>}
                {iconRight && <Icon name={iconRight} />}
            </button>
        );
    },
);
Button.displayName = 'Button';
