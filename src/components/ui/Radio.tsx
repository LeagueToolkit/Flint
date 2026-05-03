import React from 'react';

export interface RadioOption<T extends string = string> {
    value: T;
    label: React.ReactNode;
    description?: React.ReactNode;
    icon?: React.ReactNode;
    disabled?: boolean;
}

export interface RadioProps<T extends string = string> extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type' | 'value' | 'onChange'> {
    name: string;
    value: T;
    checked: boolean;
    onChange: (value: T) => void;
    label?: React.ReactNode;
    icon?: React.ReactNode;
}

export function Radio<T extends string = string>({
    name,
    value,
    checked,
    onChange,
    label,
    icon,
    className = '',
    ...rest
}: RadioProps<T>) {
    return (
        <label className={`radio-label ${className}`.trim()}>
            <input
                type="radio"
                name={name}
                value={value}
                checked={checked}
                onChange={() => onChange(value)}
                {...rest}
            />
            {icon}
            {label && <span className="radio-text">{label}</span>}
        </label>
    );
}

export interface RadioGroupProps<T extends string = string> {
    name: string;
    value: T;
    onChange: (value: T) => void;
    options: RadioOption<T>[];
    /** stacked column vs row (default row). */
    stacked?: boolean;
    className?: string;
}

export function RadioGroup<T extends string = string>({
    name,
    value,
    onChange,
    options,
    stacked = false,
    className = '',
}: RadioGroupProps<T>) {
    const style: React.CSSProperties = stacked
        ? { display: 'flex', flexDirection: 'column', gap: 8 }
        : {};
    return (
        <div className={`radio-group ${className}`.trim()} style={style}>
            {options.map((opt) => (
                <Radio<T>
                    key={opt.value}
                    name={name}
                    value={opt.value}
                    checked={value === opt.value}
                    onChange={onChange}
                    disabled={opt.disabled}
                    icon={opt.icon}
                    label={opt.label}
                />
            ))}
        </div>
    );
}
