/**
 * Flint UI primitives showcase.
 *
 * Renders every primitive in every variant on a single panel. Mount it
 * from a dev-only route (or temporarily inside SettingsModal's Dev tab)
 * to visually audit changes to the primitives or the underlying CSS.
 *
 * This file deliberately uses no project state — it is fully standalone.
 */

import React, { useState } from 'react';
import { Button } from './Button';
import { Checkbox } from './Checkbox';
import { Dropdown } from './Dropdown';
import {
    Field,
    FormError,
    FormGroup,
    FormHint,
    FormLabel,
    FormRow,
    Input,
    Range,
    SearchInput,
    Select,
    Textarea,
} from './FormField';
import { Icon, type IconName } from './Icon';
import { Modal, ModalBody, ModalFooter, ModalHeader, ModalLoading } from './Modal';
import { Panel, PanelBody, PanelFooter, PanelHeader } from './Panel';
import { ProgressBar } from './ProgressBar';
import { RadioGroup, type RadioOption } from './Radio';
import { Spinner } from './Spinner';

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

const Section: React.FC<{ title: string; subtitle?: string; children: React.ReactNode }> = ({
    title,
    subtitle,
    children,
}) => (
    <section style={{ marginBottom: 32 }}>
        <header style={{ marginBottom: 12 }}>
            <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600, color: 'var(--text-primary)' }}>{title}</h2>
            {subtitle && (
                <p style={{ margin: '4px 0 0', fontSize: 12, color: 'var(--text-muted)' }}>{subtitle}</p>
            )}
        </header>
        <div
            style={{
                padding: 16,
                background: 'var(--bg-tertiary)',
                border: '1px solid var(--border)',
                borderRadius: 8,
            }}
        >
            {children}
        </div>
    </section>
);

const Row: React.FC<{ label?: string; children: React.ReactNode; align?: 'start' | 'center' }> = ({
    label,
    children,
    align = 'center',
}) => (
    <div
        style={{
            display: 'grid',
            gridTemplateColumns: '120px 1fr',
            gap: 12,
            alignItems: align === 'start' ? 'flex-start' : 'center',
            padding: '8px 0',
            borderBottom: '1px dashed var(--border)',
        }}
    >
        <span style={{ fontSize: 11, color: 'var(--text-muted)', fontFamily: 'monospace' }}>
            {label ?? ''}
        </span>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, alignItems: 'center' }}>{children}</div>
    </div>
);

// ---------------------------------------------------------------------------
// Showcase
// ---------------------------------------------------------------------------

const RADIO_OPTIONS: RadioOption<'one' | 'two' | 'three'>[] = [
    { value: 'one', label: 'One' },
    { value: 'two', label: 'Two' },
    { value: 'three', label: 'Three', disabled: true },
];

const ICON_SAMPLE: IconName[] = ['settings', 'folder', 'file', 'success', 'warning', 'error', 'info', 'search', 'refresh', 'download', 'trash', 'wrench'];

export const UIShowcase: React.FC = () => {
    const [text, setText] = useState('Hello world');
    const [search, setSearch] = useState('');
    const [textarea, setTextarea] = useState('Multi\nline\ncontent');
    const [select, setSelect] = useState('a');
    const [range, setRange] = useState(50);
    const [hue, setHue] = useState(180);
    const [check1, setCheck1] = useState(true);
    const [check2, setCheck2] = useState(false);
    const [toggle1, setToggle1] = useState(true);
    const [toggle2, setToggle2] = useState(false);
    const [radio, setRadio] = useState<'one' | 'two' | 'three'>('one');
    const [progress, setProgress] = useState(35);
    const [openModal, setOpenModal] = useState<null | 'default' | 'wide' | 'large' | 'loading'>(null);

    return (
        <div
            style={{
                padding: 24,
                maxWidth: 960,
                margin: '0 auto',
                color: 'var(--text-primary)',
                fontSize: 13,
            }}
        >
            <header style={{ marginBottom: 32 }}>
                <h1 style={{ margin: 0, fontSize: 24, fontWeight: 700 }}>UI Primitives</h1>
                <p style={{ margin: '8px 0 0', color: 'var(--text-secondary)' }}>
                    Every component in every variant. Use this page to audit visual consistency
                    after changes to <code>src/components/ui/</code> or <code>src/styles/index.css</code>.
                </p>
            </header>

            {/* ─── Buttons ────────────────────────────────────────────────── */}
            <Section
                title="Button"
                subtitle="variant × size × icon support. Disabled and active states included."
            >
                <Row label="variant">
                    <Button variant="primary">Primary</Button>
                    <Button variant="secondary">Secondary</Button>
                    <Button variant="ghost">Ghost</Button>
                    <Button variant="danger">Danger</Button>
                </Row>
                <Row label="size">
                    <Button size="sm">Small</Button>
                    <Button size="md">Medium</Button>
                    <Button size="lg" variant="primary">
                        Large
                    </Button>
                </Row>
                <Row label="with icon">
                    <Button icon="download" variant="primary">
                        Download
                    </Button>
                    <Button iconRight="chevronRight">Next</Button>
                    <Button icon="refresh" variant="ghost">
                        Refresh
                    </Button>
                </Row>
                <Row label="iconOnly">
                    <Button iconOnly icon="settings" title="Settings" />
                    <Button iconOnly icon="trash" variant="danger" title="Delete" />
                    <Button iconOnly icon="success" variant="primary" title="Confirm" />
                </Row>
                <Row label="state">
                    <Button>Idle</Button>
                    <Button active>Active</Button>
                    <Button disabled>Disabled</Button>
                    <Button variant="primary" disabled>
                        Primary Disabled
                    </Button>
                </Row>
                <Row label="fullWidth">
                    <div style={{ width: 320 }}>
                        <Button fullWidth variant="primary">
                            Stretch to fill
                        </Button>
                    </div>
                </Row>
            </Section>

            {/* ─── Form fields ─────────────────────────────────────────────── */}
            <Section title="Form fields" subtitle="Input, Select, Textarea, Range, SearchInput, Field wrapper.">
                <Row label="Input" align="start">
                    <div style={{ width: 280 }}>
                        <Input value={text} onChange={(e) => setText(e.target.value)} placeholder="Type..." />
                    </div>
                </Row>
                <Row label="Input + button" align="start">
                    <div style={{ width: 280 }}>
                        <Input
                            value={text}
                            onChange={(e) => setText(e.target.value)}
                            placeholder="C:\Path\To\Folder"
                            buttonLabel="Browse"
                            onButtonClick={() => alert('Browse clicked')}
                        />
                    </div>
                </Row>
                <Row label="Input invalid" align="start">
                    <div style={{ width: 280 }}>
                        <Input invalid placeholder="Has error" />
                    </div>
                </Row>
                <Row label="Input sm" align="start">
                    <div style={{ width: 200 }}>
                        <Input sizeVariant="sm" placeholder="Small input" />
                    </div>
                </Row>
                <Row label="SearchInput" align="start">
                    <div style={{ width: 280 }}>
                        <SearchInput value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search..." />
                    </div>
                </Row>
                <Row label="Select" align="start">
                    <Select value={select} onChange={(e) => setSelect(e.target.value)}>
                        <option value="a">Alpha</option>
                        <option value="b">Bravo</option>
                        <option value="c">Charlie</option>
                    </Select>
                </Row>
                <Row label="Textarea" align="start">
                    <div style={{ width: 320 }}>
                        <Textarea
                            rows={3}
                            value={textarea}
                            onChange={(e) => setTextarea(e.target.value)}
                            placeholder="Long-form text..."
                        />
                    </div>
                </Row>
                <Row label="Range" align="start">
                    <div style={{ width: 240 }}>
                        <Range
                            min={0}
                            max={100}
                            value={range}
                            onChange={(e) => setRange(parseInt(e.target.value))}
                        />
                        <FormHint>Value: {range}</FormHint>
                    </div>
                </Row>
                <Row label="Range hue" align="start">
                    <div style={{ width: 240 }}>
                        <Range
                            hue
                            min={0}
                            max={360}
                            value={hue}
                            onChange={(e) => setHue(parseInt(e.target.value))}
                        />
                        <FormHint>Hue: {hue}°</FormHint>
                    </div>
                </Row>
                <Row label="Field" align="start">
                    <div style={{ width: 320 }}>
                        <Field
                            label="Display name"
                            required
                            placeholder="My Awesome Mod"
                            hint="Shown to users in the launcher."
                        />
                    </div>
                </Row>
                <Row label="Field error" align="start">
                    <div style={{ width: 320 }}>
                        <Field
                            label="Version"
                            placeholder="1.0.0"
                            error="Must follow semver."
                        />
                    </div>
                </Row>
                <Row label="FormRow" align="start">
                    <div style={{ width: 420 }}>
                        <FormRow>
                            <FormGroup half>
                                <FormLabel>First</FormLabel>
                                <Input placeholder="First name" />
                            </FormGroup>
                            <FormGroup half>
                                <FormLabel>Last</FormLabel>
                                <Input placeholder="Last name" />
                            </FormGroup>
                        </FormRow>
                    </div>
                </Row>
                <Row label="hint / error" align="start">
                    <div>
                        <FormHint>Plain hint text below an input.</FormHint>
                        <FormError>An error message in red.</FormError>
                    </div>
                </Row>
            </Section>

            {/* ─── Checkbox / Radio ─────────────────────────────────────────── */}
            <Section title="Checkbox & Radio" subtitle="Plain checkboxes, toggle switches, radio groups.">
                <Row label="Checkbox">
                    <Checkbox checked={check1} onChange={(e) => setCheck1(e.target.checked)} label="Checked" />
                    <Checkbox checked={check2} onChange={(e) => setCheck2(e.target.checked)} label="Unchecked" />
                    <Checkbox disabled label="Disabled" />
                </Row>
                <Row label="Toggle" align="start">
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, width: 360 }}>
                        <Checkbox
                            toggle
                            checked={toggle1}
                            onChange={(e) => setToggle1(e.target.checked)}
                            label="Verbose logging"
                            description="Show detailed debug output in the log panel."
                        />
                        <Checkbox
                            toggle
                            checked={toggle2}
                            onChange={(e) => setToggle2(e.target.checked)}
                            label="Auto-sync to launcher"
                            description="Sync project changes to LTK Manager automatically."
                        />
                        <Checkbox toggle disabled label="Disabled toggle" description="Greyed out." />
                    </div>
                </Row>
                <Row label="RadioGroup">
                    <RadioGroup name="row-group" value={radio} onChange={setRadio} options={RADIO_OPTIONS} />
                </Row>
                <Row label="RadioGroup stacked" align="start">
                    <RadioGroup
                        name="stacked-group"
                        value={radio}
                        onChange={setRadio}
                        options={RADIO_OPTIONS}
                        stacked
                    />
                </Row>
            </Section>

            {/* ─── Dropdown ─────────────────────────────────────────────────── */}
            <Section title="Dropdown" subtitle="Click-outside + Escape to close. Items array or custom children.">
                <Row label="items[]">
                    <Dropdown
                        trigger={(open, toggle) => (
                            <Button onClick={toggle} iconRight="chevronDown" active={open}>
                                Actions
                            </Button>
                        )}
                        items={[
                            { label: 'Open', icon: <Icon name="folder" />, onClick: () => alert('open') },
                            { label: 'Refresh', icon: <Icon name="refresh" />, onClick: () => alert('refresh') },
                            { divider: true },
                            { label: 'Delete', icon: <Icon name="trash" />, danger: true, onClick: () => alert('delete') },
                        ]}
                    />
                </Row>
                <Row label="align left">
                    <Dropdown
                        align="left"
                        trigger={(_, toggle) => <Button onClick={toggle}>Left aligned</Button>}
                        items={[
                            { label: 'Option A', onClick: () => {} },
                            { label: 'Option B', onClick: () => {} },
                        ]}
                    />
                </Row>
            </Section>

            {/* ─── Icons ────────────────────────────────────────────────────── */}
            <Section title="Icon" subtitle="Typed by IconName. Sample of frequently used icons.">
                <Row label="default 16px">
                    {ICON_SAMPLE.map((name) => (
                        <span
                            key={name}
                            title={name}
                            style={{
                                display: 'inline-flex',
                                flexDirection: 'column',
                                alignItems: 'center',
                                gap: 4,
                                color: 'var(--text-secondary)',
                            }}
                        >
                            <Icon name={name} />
                            <span style={{ fontSize: 10, fontFamily: 'monospace' }}>{name}</span>
                        </span>
                    ))}
                </Row>
                <Row label="size=24">
                    <Icon name="settings" size={24} />
                    <Icon name="folder" size={24} />
                    <Icon name="success" size={24} />
                </Row>
            </Section>

            {/* ─── Spinner / Progress ───────────────────────────────────────── */}
            <Section title="Spinner & ProgressBar" subtitle="Loading indicators.">
                <Row label="Spinner">
                    <Spinner size="sm" />
                    <Spinner size="md" />
                    <Spinner size="lg" />
                </Row>
                <Row label="ProgressBar" align="start">
                    <div style={{ width: 320 }}>
                        <ProgressBar value={progress} label="Downloading update..." />
                        <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
                            <Button size="sm" onClick={() => setProgress((p) => Math.max(0, p - 10))}>
                                -10
                            </Button>
                            <Button size="sm" onClick={() => setProgress((p) => Math.min(100, p + 10))}>
                                +10
                            </Button>
                        </div>
                    </div>
                </Row>
                <Row label="indeterminate" align="start">
                    <div style={{ width: 320 }}>
                        <ProgressBar value={100} hideHeader />
                        <FormHint>(static 100% — no marquee animation today)</FormHint>
                    </div>
                </Row>
            </Section>

            {/* ─── Panel ────────────────────────────────────────────────────── */}
            <Section title="Panel" subtitle="Generic surface for sidebars / floating panels.">
                <Row label="basic" align="start">
                    <div style={{ width: 320 }}>
                        <Panel padded style={{ background: 'var(--bg-secondary)', border: '1px solid var(--border)', borderRadius: 8 }}>
                            <h3 style={{ margin: 0, fontSize: 14 }}>Plain panel</h3>
                            <p style={{ marginTop: 8, color: 'var(--text-secondary)' }}>
                                Compose with className for surface treatment.
                            </p>
                        </Panel>
                    </div>
                </Row>
                <Row label="Header/Body/Footer" align="start">
                    <div style={{ width: 320, background: 'var(--bg-secondary)', border: '1px solid var(--border)', borderRadius: 8, overflow: 'hidden' }}>
                        <Panel>
                            <PanelHeader>
                                <strong>Section header</strong>
                            </PanelHeader>
                            <PanelBody>
                                <p style={{ margin: 0, color: 'var(--text-secondary)' }}>Body content goes here.</p>
                            </PanelBody>
                            <PanelFooter>
                                <Button size="sm">OK</Button>
                            </PanelFooter>
                        </Panel>
                    </div>
                </Row>
            </Section>

            {/* ─── Modal demos ──────────────────────────────────────────────── */}
            <Section title="Modal" subtitle="Three sizes and a loading-overlay demo.">
                <Row label="open">
                    <Button onClick={() => setOpenModal('default')}>Default (500px)</Button>
                    <Button onClick={() => setOpenModal('wide')}>Wide (800px)</Button>
                    <Button onClick={() => setOpenModal('large')}>Large (1000px)</Button>
                    <Button variant="primary" onClick={() => setOpenModal('loading')}>
                        With loading overlay
                    </Button>
                </Row>
            </Section>

            {/* ─── Modal renderers ──────────────────────────────────────────── */}
            <Modal open={openModal === 'default'} onClose={() => setOpenModal(null)}>
                <ModalHeader title="Default modal" onClose={() => setOpenModal(null)} />
                <ModalBody>
                    <Field label="Email" placeholder="you@example.com" />
                    <FormGroup>
                        <FormLabel>About</FormLabel>
                        <Textarea rows={3} placeholder="Tell us about yourself..." />
                    </FormGroup>
                </ModalBody>
                <ModalFooter>
                    <Button onClick={() => setOpenModal(null)}>Cancel</Button>
                    <Button variant="primary" onClick={() => setOpenModal(null)}>
                        Save
                    </Button>
                </ModalFooter>
            </Modal>

            <Modal open={openModal === 'wide'} onClose={() => setOpenModal(null)} size="wide">
                <ModalHeader title="Wide modal (800px)" onClose={() => setOpenModal(null)} />
                <ModalBody>
                    <p>Use for two-column layouts or larger forms.</p>
                </ModalBody>
                <ModalFooter split>
                    <Button variant="ghost">Help</Button>
                    <div className="modal__footer-actions">
                        <Button onClick={() => setOpenModal(null)}>Cancel</Button>
                        <Button variant="primary" onClick={() => setOpenModal(null)}>
                            Save
                        </Button>
                    </div>
                </ModalFooter>
            </Modal>

            <Modal open={openModal === 'large'} onClose={() => setOpenModal(null)} size="large">
                <ModalHeader title="Large modal (1000px)" onClose={() => setOpenModal(null)} />
                <ModalBody>
                    <p>Use for asset-heavy modals like the recolor preview.</p>
                </ModalBody>
                <ModalFooter>
                    <Button variant="primary" onClick={() => setOpenModal(null)}>
                        Done
                    </Button>
                </ModalFooter>
            </Modal>

            <Modal open={openModal === 'loading'} onClose={() => setOpenModal(null)}>
                <ModalLoading text="Doing the thing..." progress="42 of 128 files" />
                <ModalHeader title="Working..." />
                <ModalBody>
                    <p>The overlay covers the modal while async work runs.</p>
                </ModalBody>
                <ModalFooter>
                    <Button onClick={() => setOpenModal(null)}>Close</Button>
                </ModalFooter>
            </Modal>
        </div>
    );
};
