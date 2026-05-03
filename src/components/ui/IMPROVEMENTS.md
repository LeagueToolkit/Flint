# Flint UI — Improvements Backlog

A focused critique of the design system after the 2026-05 primitives migration. Findings are grouped by theme; each item lists what's wrong, why it hurts UX, and the smallest concrete change that fixes it.

> **Method**: I walked every modal during migration. Items below are things I had to *work around* — not theoretical nits.

---

## 1. Token system is leaky

The CSS uses three different naming conventions for the same concept:

| Concept | Names found | Files |
|---|---|---|
| Danger color | `--accent-danger`, `var(--error)`, raw `#f87171`, raw `#F85149`, raw `#ef4444` | ConfirmDialog, FixerModal, ModConfigEditor, FullResImageModal |
| Warning color | `--color-warning`, `--accent-warning`, raw `#f97316` | SettingsModal, FixerModal |
| Surface bg | `--bg-secondary`, `--bg-tertiary`, `--bg-elevated`, `--color-surface-2` | SettingsModal mixes the last two |
| Border | `--border`, `--border-color`, `--color-border` | UpdateModal uses `--border-color`; SettingsModal Dev tab uses `--color-border` |

**Why it hurts**: themes can't ship cleanly — a custom red theme picks up `var(--accent-danger)` everywhere except the four places hard-coded to `#f87171`. The "Create Custom Theme" button (SettingsModal) silently lies about which knobs work.

**Fix** — promote one canonical set in `src/themes/`:

```css
:root {
  --color-danger: #f85149;
  --color-warning: #f97316;
  --color-success: #2ea043;
  --color-info:    var(--accent-secondary);
  --surface-1: var(--bg-primary);
  --surface-2: var(--bg-secondary);
  --surface-3: var(--bg-tertiary);
  --surface-elevated: var(--bg-elevated);
  --border-default: var(--border);
}
```

Then sweep the codebase: replace every raw hex and every legacy alias. Lint with a regex pre-commit check (`grep -E "#[0-9a-fA-F]{6}" src/**/*.tsx`).

---

## 2. Button variants overlap meaningfully

We have `primary | secondary | ghost | danger`. In practice:

- **`secondary`** and **`ghost`** are visually so similar in the current theme that ghost is mostly used as "tertiary text-link with padding." Migration found at least 6 places where the choice between them was arbitrary (FixerModal close button, SettingsModal auto-detect, BinSplitModal All/None).
- There's no **destructive secondary** — danger is always filled. Deleting a checkpoint should arguably be a danger-outline, not a full red bar.
- There's no **subtle/text** variant for inline actions inside cards (CheckpointModal "Restore" + "Delete"). I had to fake it with `variant="ghost" size="sm" icon="..."`.

**Fix** — collapse + clarify:

| New | Use case | Visual |
|---|---|---|
| `primary` | The single confirming action of a flow | Filled accent |
| `secondary` | All other clickable buttons | Outlined neutral |
| `text` | Inline links / "More options" | No border, accent text on hover |
| `danger` | Destructive primary (delete project) | Filled red |
| `danger-outline` | Destructive secondary (remove tag) | Outlined red |

Drop `ghost` (rename existing `ghost` usage to `text`).

---

## 3. Size scale has duplicates

`src/styles/index.css` defines both `.btn--sm` (line 3004 and 4542) **and** `.btn--small` (line 4093). Some buttons reference `btn--xs` which doesn't exist. The `<Button>` primitive only emits `--sm | --large`, but raw markup in WadCheatSheetModal and FullResImageModal still hits `--small`.

**Fix** — pick one (`--sm | --md | --lg`), delete the other rule, grep for stragglers. Codify the scale in tokens:

```css
:root {
  --btn-h-sm: 24px;
  --btn-h-md: 32px;
  --btn-h-lg: 40px;
  --btn-px-sm: 8px;
  --btn-px-md: 12px;
  --btn-px-lg: 16px;
}
```

---

## 4. Focus indicators are inconsistent

`.btn--danger:focus-visible` has a ring (line 6838), but `.btn--primary` / `.btn--secondary` / `.btn--ghost` do not. Inputs have a `:focus` border swap but no offset ring. Checkboxes (the toggle variant especially) have **no** focus indicator — keyboard users cannot see where focus is on the SettingsModal toggles.

**Fix** — add a single global rule:

```css
*:focus-visible {
  outline: 2px solid var(--accent-primary);
  outline-offset: 2px;
  border-radius: inherit;
}
```

Then remove per-variant focus rules. Verify by tabbing through SettingsModal end-to-end.

---

## 5. Modal sizing is hardcoded and arbitrary

- Default `500px`, wide `800px`, large `1000px` — but real modals override with inline `width: 480px` (ModConfigEditor was), `width: 720px` (BinSplit), `width: 820px` (ThumbnailCrop), `92vw` (FullResImage), `width: 92%` (CheatSheet). Five different sizes for "large-ish."
- The defaults don't account for content. `ConfirmDialog` (one paragraph) and `ExportModal` (full form) currently look the same width.

**Fix** — promote the most-used sizes to first-class:

```ts
size: 'compact' | 'default' | 'wide' | 'large' | 'fullscreen'
// 380 / 520 / 720 / 960 / 92vw
```

…and stop accepting inline `style={{ width: ... }}`. If a modal needs custom size it gets a CSS modifier.

---

## 6. `ConfirmDialog` is a parallel design system

`ConfirmDialog.tsx` uses its own `.confirm-overlay` / `.confirm-dialog__icon` / `.confirm-dialog__title` classes — a complete duplicate of the modal layer. The icon, title and message slot have a tighter, more polished look than the generic Modal — but inconsistent with everything else.

**Fix** — either:
- (a) Migrate to `<Modal modifier="modal--confirm">` and copy the `.confirm-*` styles into `.modal--confirm` modifiers. One overlay system.
- (b) Promote ConfirmDialog's *visual treatment* (icon-leading, two-button footer) to be the standard for compact modals, and rebuild Modal around that. This is the more interesting redesign.

I'd vote (b) — the confirm dialog is the prettiest modal we have.

---

## 7. Toggle (settings-toggle) is the most overloaded primitive

The toggle variant of `Checkbox` actually wraps three separate concerns:
1. The control itself (track + thumb)
2. A two-line label/description block
3. A whole-row hover background that's specific to settings rows

Outside SettingsModal, the description and row-background look out of place. RecolorModal uses `Checkbox` (plain) with a `<FormHint>` underneath — completely different visual rhythm.

**Fix** — split:

- `<Switch checked label />` — pure control + inline label, used anywhere
- `<SettingsRow icon label description action />` — the row layout (label left, control right, hover surface). SettingsModal uses this; nobody else needs to.

This kills ~50 lines of duplicated row markup in SettingsModal.

---

## 8. Icon + label spacing is hand-rolled everywhere

`Button` puts icon and label inside two `<span>`s with no consistent gap rule. Some buttons use `getIcon()` SVG followed by raw text (4px implicit gap from whitespace). Others wrap label in `<span>`. Visual gap varies between 4px–10px depending on which path.

**Fix** — give `.btn` a flex gap:

```css
.btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
.btn--sm { gap: 6px; }
```

Then drop manual `marginRight: 4` hacks (NewProjectModal, ProjectListModal still have these from inline SVGs).

---

## 9. Dropdown is built but barely used

`<Dropdown>` exists, click-outside works, items array is typed — but `TitleBar.tsx` still has a hand-rolled export menu with inline `onMouseEnter={(e) => e.currentTarget.style.background = 'var(--bg-hover)'}` (line 468). Same in WadExplorer for column actions.

**Fix** — migrate TitleBar's export menu to `<Dropdown>` as proof, then sweep. Add a `<Menu>` alias for semantic clarity (Dropdown = trigger + menu).

---

## 10. Disabled states are visually identical to enabled-but-quiet

`.btn:disabled` only does `opacity: 0.5; cursor: not-allowed;`. On dark theme with already-low contrast secondary buttons, that's hard to tell apart from the active state. Worse, hover styles still apply on disabled buttons in some cases.

**Fix**:
```css
.btn:disabled,
.btn[aria-disabled="true"] {
  opacity: 0.4;
  filter: saturate(0.5);
  pointer-events: none; /* kills hover entirely */
}
```

---

## 11. Modal close (`×`) is a typographic character

Every modal close uses literal `×` (multiplication sign). It looks fine at default zoom but is fuzzy on hi-DPI and inconsistent in stroke weight vs the SVG icons used elsewhere. ModalHeader hardcodes it.

**Fix** — replace with `<Icon name="close" />` (already in `fileIcons.tsx`). One-line change in `Modal.tsx`.

---

## 12. Form layout is inflexible

`FormGroup` is `margin-bottom: 16px` (or whatever the CSS says). When you need tighter spacing — e.g., a stack of toggles in SettingsModal — you can't override without inline style. The `FormRow` half-half pattern works but doesn't extend to thirds or fourths.

**Fix** — accept a `density: 'compact' | 'comfortable' | 'spacious'` prop on `FormGroup` (controls margin), and a `cols: 1|2|3|4` prop on `FormRow`. CSS:

```css
.form-row { display: grid; gap: 12px; }
.form-row[data-cols="2"] { grid-template-columns: 1fr 1fr; }
.form-row[data-cols="3"] { grid-template-columns: 1fr 1fr 1fr; }
```

Drop `form-group--half`.

---

## 13. Input-with-button has bad hit targets

The combined `<Input buttonLabel="Browse">` puts a default-size button next to a default-size input. The button extends past the input's border-radius and creates a fiddly visual seam. Most "Browse" buttons in SettingsModal would be better as a leading icon button *inside* the input border (like macOS).

**Fix** — redesign as:
```
┌─────────────────────────────┬──────┐
│  C:\path\to\folder          │ 📁   │
└─────────────────────────────┴──────┘
```
One unified border, the trailing slot is icon-only and 32×32. Browse opens a picker; if a user wants to clear, they can backspace.

---

## 14. Animation/motion is missing

Modals fade in but everything else snaps:
- Toggles flip with no transition on the thumb (CSS has `transition: all` but the `::before` pseudo is animated, the parent isn't)
- Checkbox check appears instantly
- Button hover has `transition: background-color` but not transform — would benefit from a subtle 1px lift on press
- ProgressBar is the only component with motion (`width 0.2s ease`)

**Fix** — adopt a motion scale:
```css
:root {
  --motion-fast: 120ms cubic-bezier(0.4, 0, 0.2, 1);
  --motion-base: 200ms cubic-bezier(0.4, 0, 0.2, 1);
  --motion-slow: 320ms cubic-bezier(0.4, 0, 0.2, 1);
}
```
Apply `--motion-fast` to all hovers, `--motion-base` to state changes (toggle flip, checkbox tick), `--motion-slow` to modal enter/exit. Respect `prefers-reduced-motion`.

---

## 15. No empty-state component

ProjectListModal has a custom empty state. CheckpointModal has a different custom empty state. FixerModal has nothing — just a blank panel. Each is hand-rolled with different padding, icon size, and color treatment.

**Fix** — `<EmptyState icon title description action />`:
```tsx
<EmptyState
  icon="folder"
  title="No projects yet"
  description="Create a new project or import a mod to get started."
  action={<Button variant="primary">New Project</Button>}
/>
```
Replaces ~60 lines of duplicated markup across 5+ places.

---

## 16. No tooltip component

We use `title=""` everywhere, which gives the OS native tooltip — wrong font, wrong theme, 1.5s delay. Critical hover-info buttons (close, action icons) all have native tooltips.

**Fix** — small `<Tooltip>` wrapper using a single shared portal. Default delay 400ms, dark surface, themed.

---

## 17. Status colors don't compose

A "success" toast, a "success" status badge in CheckpointModal, and a "success" icon in SettingsModal all use different green shades. The Toast system has its own palette in CSS, separate from any token.

**Fix** — three-tone status palette per state, applied via `data-status="success"`:
```css
[data-status="success"] {
  --status-bg: color-mix(in oklab, var(--color-success) 12%, var(--surface-2));
  --status-fg: var(--color-success);
  --status-border: color-mix(in oklab, var(--color-success) 40%, transparent);
}
```
One pattern, used by Toast/Badge/Status/empty-state-icon.

---

## Suggested execution order

If we tackle this incrementally, the highest-leverage order is:

1. **Token sweep** (#1, #3) — unblocks everything theme-related, pure refactor.
2. **Focus + disabled + close icon** (#4, #10, #11) — accessibility + polish, near-zero risk.
3. **Button variants** (#2) — needs migration grep but clarifies intent.
4. **Empty state + tooltip + status colors** (#15, #16, #17) — net-new components, additive.
5. **Toggle / SettingsRow split** (#7) — touches SettingsModal, needs care.
6. **ConfirmDialog → Modal redesign** (#6) — biggest visual change, do last.

Items #5, #8, #9, #12, #13, #14 are smaller and can land opportunistically alongside any of the above.
