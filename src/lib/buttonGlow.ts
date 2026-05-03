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
        const target = (e.target as HTMLElement | null)?.closest<HTMLElement>(
            '.btn, .np-champ-card',
        );
        if (!target) return;
        const r = target.getBoundingClientRect();
        target.style.setProperty('--mx', `${e.clientX - r.left}px`);
        target.style.setProperty('--my', `${e.clientY - r.top}px`);
    }, { passive: true });
}
