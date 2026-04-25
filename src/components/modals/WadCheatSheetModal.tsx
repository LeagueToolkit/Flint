import React, { useMemo, useState, useCallback } from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import CHEAT_SHEET_MD from '../../assets/wad-cheat-sheet.md?raw';

// ─── Placeholder detection ────────────────────────────────────────────────────

const PLACEHOLDER_TOKENS = ['CHAMPION', 'SKINID', 'MINIONNAME', 'XX', 'YY', 'ZZ'];
const PLACEHOLDER_RE = new RegExp(PLACEHOLDER_TOKENS.join('|'), 'g');

function hasPlaceholder(text: string): boolean {
    return PLACEHOLDER_TOKENS.some(p => text.includes(p));
}

/** Converts a path with ALLCAPS placeholders into a regex string usable in WAD Explorer search. */
function toRegexPattern(path: string): string {
    return path
        .replace(/\./g, '\\.')       // escape literal dots
        .replace(PLACEHOLDER_RE, '[^/]+'); // replace each placeholder with a non-slash wildcard
}

// ─── Inline token types ───────────────────────────────────────────────────────

type Token =
    | { kind: 'text'; text: string }
    | { kind: 'bold'; text: string }
    | { kind: 'italic'; text: string }
    | { kind: 'link'; text: string; url: string }
    | { kind: 'code'; raw: string; wad?: string; filePath?: string; filter?: string };

function parseCodeSpan(raw: string): Token & { kind: 'code' } {
    const spaceIdx = raw.indexOf(' ');
    const firstWord = spaceIdx === -1 ? raw : raw.slice(0, spaceIdx);
    const rest = spaceIdx === -1 ? '' : raw.slice(spaceIdx + 1);

    if (firstWord.endsWith('.wad.client') || firstWord.endsWith('.wad')) {
        const wadIsPlaceholder = hasPlaceholder(firstWord);
        const pathHasPlaceholder = rest ? hasPlaceholder(rest) : false;
        return {
            kind: 'code',
            raw,
            wad: wadIsPlaceholder ? undefined : firstWord,
            filePath: (!wadIsPlaceholder && rest && !pathHasPlaceholder) ? rest : undefined,
            filter: rest ? (pathHasPlaceholder ? toRegexPattern(rest) || undefined : rest) : undefined,
        };
    }

    if (raw.startsWith('assets/') || raw.startsWith('data/')) {
        const filter = toRegexPattern(raw) || undefined;
        return { kind: 'code', raw, filter };
    }

    return { kind: 'code', raw };
}

function parseInline(text: string): Token[] {
    const tokens: Token[] = [];
    let i = 0;
    while (i < text.length) {
        if (text[i] === '*' && text[i + 1] === '*') {
            const end = text.indexOf('**', i + 2);
            if (end !== -1) { tokens.push({ kind: 'bold', text: text.slice(i + 2, end) }); i = end + 2; continue; }
        }
        if (text[i] === '*' && text[i + 1] !== '*') {
            const end = text.indexOf('*', i + 1);
            if (end !== -1) { tokens.push({ kind: 'italic', text: text.slice(i + 1, end) }); i = end + 1; continue; }
        }
        if (text[i] === '`') {
            const end = text.indexOf('`', i + 1);
            if (end !== -1) { tokens.push(parseCodeSpan(text.slice(i + 1, end))); i = end + 1; continue; }
        }
        if (text[i] === '[') {
            const cb = text.indexOf(']', i);
            if (cb !== -1 && text[cb + 1] === '(') {
                const cp = text.indexOf(')', cb + 2);
                if (cp !== -1) {
                    tokens.push({ kind: 'link', text: text.slice(i + 1, cb), url: text.slice(cb + 2, cp) });
                    i = cp + 1;
                    continue;
                }
            }
        }
        const last = tokens[tokens.length - 1];
        if (last?.kind === 'text') last.text += text[i];
        else tokens.push({ kind: 'text', text: text[i] });
        i++;
    }
    return tokens;
}

// ─── Block types ──────────────────────────────────────────────────────────────

type Block =
    | { type: 'h1' | 'h2' | 'h3'; tokens: Token[] }
    | { type: 'hr' }
    | { type: 'blockquote'; tokens: Token[] }
    | { type: 'listitem'; tokens: Token[]; indent: number }
    | { type: 'paragraph'; tokens: Token[] }
    | { type: 'table-head'; cells: Token[][] }
    | { type: 'table-row'; cells: Token[][] };

function parseBlocks(md: string): Block[] {
    const lines = md.split('\n');
    const blocks: Block[] = [];
    let inTable = false;

    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) { inTable = false; continue; }
        if (trimmed === '---') { blocks.push({ type: 'hr' }); inTable = false; continue; }
        if (trimmed.startsWith('### ')) { blocks.push({ type: 'h3', tokens: parseInline(trimmed.slice(4)) }); inTable = false; continue; }
        if (trimmed.startsWith('## ')) { blocks.push({ type: 'h2', tokens: parseInline(trimmed.slice(3)) }); inTable = false; continue; }
        if (trimmed.startsWith('# ')) { blocks.push({ type: 'h1', tokens: parseInline(trimmed.slice(2)) }); inTable = false; continue; }

        if (trimmed.startsWith('> ')) {
            blocks.push({ type: 'blockquote', tokens: parseInline(trimmed.slice(2)) });
            continue;
        }

        if (trimmed.startsWith('|') && trimmed.endsWith('|')) {
            const rawCells = trimmed.slice(1, -1).split('|').map(c => c.trim());
            if (rawCells.every(c => /^:?-+:?$/.test(c))) continue;
            const cells = rawCells.map(c => parseInline(c));
            if (!inTable) { blocks.push({ type: 'table-head', cells }); inTable = true; }
            else { blocks.push({ type: 'table-row', cells }); }
            continue;
        }
        inTable = false;

        const listMatch = line.match(/^(\s*)[*-]\s(.+)$/);
        if (listMatch) {
            blocks.push({ type: 'listitem', tokens: parseInline(listMatch[2]), indent: listMatch[1].length });
            continue;
        }

        blocks.push({ type: 'paragraph', tokens: parseInline(trimmed) });
    }
    return blocks;
}

// ─── Section grouping ─────────────────────────────────────────────────────────

interface Section {
    heading: string;
    blocks: Block[];
}

function groupSections(blocks: Block[]): Section[] {
    const sections: Section[] = [];
    let cur: Section = { heading: '', blocks: [] };
    sections.push(cur);

    for (const block of blocks) {
        if (block.type === 'h3' || block.type === 'h2') {
            const heading = block.tokens.map(t => (t.kind === 'text' ? t.text : '')).join('');
            cur = { heading, blocks: [block] };
            sections.push(cur);
        } else if (block.type !== 'h1') {
            // Skip h1 (document title) in body — it's already shown in the modal header
            cur.blocks.push(block);
        }
    }
    return sections.filter(s => s.blocks.length > 0);
}

// ─── Rendering ───────────────────────────────────────────────────────────────

function tokensText(tokens: Token[]): string {
    return tokens.map(t => ('raw' in t ? t.raw : 'text' in t ? t.text : '')).join('');
}

interface RenderCtx {
    onOpenWad: (wad: string, filePath?: string) => void;
    onFilterPath: (path: string) => void;
}

function RenderTokens({ tokens, ctx }: { tokens: Token[]; ctx: RenderCtx }): React.ReactElement {
    return (
        <>
            {tokens.map((tok, i) => {
                switch (tok.kind) {
                    case 'text': return <React.Fragment key={i}>{tok.text}</React.Fragment>;
                    case 'bold': return <strong key={i}>{tok.text}</strong>;
                    case 'italic': return <em key={i}>{tok.text}</em>;
                    case 'link': return (
                        <a key={i} href="#" className="cs-link"
                            onClick={e => { e.preventDefault(); openUrl(tok.url).catch(() => { }); }}>
                            {tok.text}
                        </a>
                    );
                    case 'code': {
                        const hasWadAction = !!tok.wad;
                        const hasFilterAction = !!tok.filter && !tok.filePath;
                        return (
                            <span key={i} className="cs-code-group">
                                <code className="cs-code">{tok.raw}</code>
                                {hasWadAction && (
                                    <button
                                        className="cs-pill cs-pill--wad"
                                        title={tok.filePath ? `Navigate to file in ${tok.wad}` : `Open ${tok.wad} in WAD Explorer`}
                                        onClick={() => ctx.onOpenWad(tok.wad!, tok.filePath)}
                                    >
                                        {tok.filePath ? 'Show File' : 'Show WAD'}
                                    </button>
                                )}
                                {hasFilterAction && (
                                    <button
                                        className="cs-pill cs-pill--filter"
                                        title={`Search WAD Explorer for: ${tok.filter}`}
                                        onClick={() => ctx.onFilterPath(tok.filter!)}
                                    >
                                        Show Regex
                                    </button>
                                )}
                            </span>
                        );
                    }
                }
            })}
        </>
    );
}

function RenderSectionBlocks({ blocks, ctx }: { blocks: Block[]; ctx: RenderCtx }): React.ReactElement {
    const elements: React.ReactNode[] = [];
    let i = 0;

    while (i < blocks.length) {
        const block = blocks[i];

        if (block.type === 'table-head') {
            const head = block;
            const rows: Token[][][] = [];
            i++;
            while (i < blocks.length && blocks[i].type === 'table-row') {
                rows.push((blocks[i] as { type: 'table-row'; cells: Token[][] }).cells);
                i++;
            }
            elements.push(
                <table key={`t${i}`} className="cs-table">
                    <thead>
                        <tr>{head.cells.map((c, ci) => <th key={ci}><RenderTokens tokens={c} ctx={ctx} /></th>)}</tr>
                    </thead>
                    <tbody>
                        {rows.map((row, ri) => (
                            <tr key={ri}>{row.map((c, ci) => <td key={ci}><RenderTokens tokens={c} ctx={ctx} /></td>)}</tr>
                        ))}
                    </tbody>
                </table>
            );
            continue;
        }

        switch (block.type) {
            case 'h2': elements.push(<h2 key={i} className="cs-h2"><RenderTokens tokens={block.tokens} ctx={ctx} /></h2>); break;
            case 'h3': elements.push(<h3 key={i} className="cs-h3"><RenderTokens tokens={block.tokens} ctx={ctx} /></h3>); break;
            case 'hr': elements.push(<div key={i} className="cs-section-gap" />); break;
            case 'blockquote': elements.push(
                <blockquote key={i} className="cs-blockquote">
                    <RenderTokens tokens={block.tokens} ctx={ctx} />
                </blockquote>
            ); break;
            case 'listitem': elements.push(
                <div key={i} className="cs-listitem" style={{ paddingLeft: `${block.indent + 12}px` }}>
                    <span className="cs-listitem__bullet">•</span>
                    <span><RenderTokens tokens={block.tokens} ctx={ctx} /></span>
                </div>
            ); break;
            case 'paragraph': elements.push(
                <p key={i} className="cs-paragraph"><RenderTokens tokens={block.tokens} ctx={ctx} /></p>
            ); break;
        }
        i++;
    }

    return <>{elements}</>;
}

// ─── Modal component ──────────────────────────────────────────────────────────

export interface WadCheatSheetModalProps {
    onClose: () => void;
    onOpenWad: (wadName: string, filePath?: string) => void;
    onFilterPath: (path: string) => void;
}

export const WadCheatSheetModal: React.FC<WadCheatSheetModalProps> = ({ onClose, onOpenWad, onFilterPath }) => {
    const [search, setSearch] = useState('');

    const sections = useMemo(() => groupSections(parseBlocks(CHEAT_SHEET_MD)), []);

    const filtered = useMemo(() => {
        const q = search.toLowerCase().trim();
        if (!q) return sections;
        return sections.filter(sec => {
            if (sec.heading.toLowerCase().includes(q)) return true;
            return sec.blocks.some(b => {
                if (b.type === 'hr') return false;
                return tokensText((b as { tokens?: Token[] }).tokens ?? []).toLowerCase().includes(q);
            });
        });
    }, [sections, search]);

    const ctx: RenderCtx = useMemo(() => ({
        onOpenWad: (wad, filePath) => { onOpenWad(wad, filePath); onClose(); },
        onFilterPath: (path) => { onFilterPath(path); onClose(); },
    }), [onOpenWad, onFilterPath, onClose]);

    const handleBackdrop = useCallback((e: React.MouseEvent) => {
        if (e.target === e.currentTarget) onClose();
    }, [onClose]);

    return (
        <div className="modal-overlay modal-overlay--visible" onClick={handleBackdrop}>
            <div className="modal modal--large cs-modal" style={{ width: '92%', maxHeight: '88vh', display: 'flex', flexDirection: 'column' }}>

                {/* ── Header ── */}
                <div className="cs-modal__header">
                    <span className="cs-modal__title">📖 Asset Path Cheat Sheet</span>
                    <input
                        type="text"
                        className="file-tree__search-input cs-search"
                        placeholder="Filter sections…"
                        value={search}
                        onChange={e => setSearch(e.target.value)}
                        onKeyDown={e => e.key === 'Escape' && (search ? setSearch('') : onClose())}
                        autoFocus
                    />
                    <button className="btn btn--sm" onClick={onClose} title="Close (Esc)" style={{ padding: '2px 6px', flexShrink: 0 }}>
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
                            <path d="M4.5 4.5l7 7m0-7l-7 7" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                        </svg>
                    </button>
                </div>

                {/* ── Content ── */}
                <div className="cs-modal__body">
                    {filtered.length === 0 ? (
                        <div className="cs-modal__empty">No sections match "{search}"</div>
                    ) : (
                        filtered.map((sec, si) => (
                            <div key={si} className="cs-section">
                                <RenderSectionBlocks blocks={sec.blocks} ctx={ctx} />
                            </div>
                        ))
                    )}
                </div>

                {/* ── Footer ── */}
                <div className="cs-modal__footer">
                    <span>·</span>
                    <span>Original cheat sheet by Aropatnik</span>
                    <span>·</span>
                </div>
            </div>
        </div>
    );
};
