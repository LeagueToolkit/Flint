/**
 * Cursor-following glow for `.btn`. Single delegated mousemove listener on
 * document — sets `--mx` / `--my` on the hovered button so the radial-glow
 * `::after` overlay tracks the cursor. ~Negligible cost (only runs while a
 * button is under the cursor) and zero React re-renders.
 */
let installed = false;

export function installButtonGlow() {
    if (installed || typeof document === 'undefined') return;
    installed = true;
    document.addEventListener('mousemove', (e) => {
        const btn = (e.target as HTMLElement | null)?.closest<HTMLElement>('.btn');
        if (!btn) return;
        const r = btn.getBoundingClientRect();
        btn.style.setProperty('--mx', `${e.clientX - r.left}px`);
        btn.style.setProperty('--my', `${e.clientY - r.top}px`);
    }, { passive: true });
}
