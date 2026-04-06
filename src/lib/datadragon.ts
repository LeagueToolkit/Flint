/**
 * Flint - Data Dragon / CommunityDragon API
 * 
 * Fetches champion and skin data from Riot's official APIs
 */

const DDRAGON_BASE_URL = "https://ddragon.leagueoflegends.com";
const CDRAGON_BASE_URL = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1";

// Types
export interface DDragonChampion {
    id: number;
    name: string;
    alias: string;
}

export interface DDragonSkin {
    id: number;
    name: string;
    num: number;
    isBase: boolean;
    splashPath?: string;
    tilePath?: string;
}

// Cache for API responses
let cachedPatch: string | null = null;
let cachedChampions: DDragonChampion[] | null = null;

// Image blob cache — maps URL → blob URL for reuse
const imageBlobCache = new Map<string, string>();
// In-flight fetches to avoid duplicate requests
const imageFetchQueue = new Map<string, Promise<string>>();

/**
 * Fetch an image URL and return a cached blob URL.
 * First call fetches + caches; subsequent calls return instantly.
 */
export function getCachedImageUrl(url: string): string | null {
    return imageBlobCache.get(url) ?? null;
}

/**
 * Preload an image URL into the blob cache.
 * Returns the blob URL. Safe to call multiple times (deduped).
 */
export async function preloadImage(url: string): Promise<string> {
    const cached = imageBlobCache.get(url);
    if (cached) return cached;

    const inflight = imageFetchQueue.get(url);
    if (inflight) return inflight;

    const promise = fetch(url)
        .then(res => {
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            return res.blob();
        })
        .then(blob => {
            const blobUrl = URL.createObjectURL(blob);
            imageBlobCache.set(url, blobUrl);
            imageFetchQueue.delete(url);
            return blobUrl;
        })
        .catch(() => {
            imageFetchQueue.delete(url);
            return url; // fallback to original URL
        });

    imageFetchQueue.set(url, promise);
    return promise;
}

/**
 * Preload all champion icons in the background.
 */
export async function preloadChampionIcons(champions: DDragonChampion[]): Promise<void> {
    const batch = champions.map(c => preloadImage(getChampionIconUrl(c.id)));
    await Promise.allSettled(batch);
}

/**
 * Preload all skin splashes for a champion (uses CDragon — whitelisted in CSP).
 */
export async function preloadSkinSplashes(championId: number, skins: DDragonSkin[]): Promise<void> {
    const batch = skins.map(s => preloadImage(getSkinSplashCDragonUrl(championId, s.id)));
    await Promise.allSettled(batch);
}

/**
 * Fetch with retry logic
 */
async function fetchWithRetry<T>(url: string, retries = 3): Promise<T> {
    try {
        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        return await response.json();
    } catch (error) {
        if (retries > 0) {
            await new Promise((resolve) => setTimeout(resolve, 1000));
            return fetchWithRetry(url, retries - 1);
        }
        throw error;
    }
}

/**
 * Get latest patch version
 */
export async function getLatestPatch(): Promise<string> {
    if (cachedPatch) return cachedPatch;

    try {
        const versions = await fetchWithRetry<string[]>(`${DDRAGON_BASE_URL}/api/versions.json`);
        cachedPatch = versions[0];  // First is latest
        return cachedPatch;
    } catch (error) {
        console.error("Failed to fetch patch versions:", error);
        return "14.23.1";  // Fallback
    }
}

/**
 * Fetch all champions from CommunityDragon
 */
export async function fetchChampions(): Promise<DDragonChampion[]> {
    if (cachedChampions) return cachedChampions;

    try {
        // Use CommunityDragon champion summary (simpler format)
        const url = `${CDRAGON_BASE_URL}/champion-summary.json`;
        interface ChampionSummary {
            id: number;
            name: string;
            alias: string;
        }
        const champions = await fetchWithRetry<ChampionSummary[]>(url);

        // Filter out special entries (id < 0 or Doom Bots) and map to our type
        cachedChampions = champions
            .filter(c => c.id > 0 && c.id < 10000)
            .map(c => ({
                id: c.id,
                name: c.name,
                alias: c.alias
            }))
            .sort((a, b) => a.name.localeCompare(b.name));

        return cachedChampions;
    } catch (error) {
        console.error("Failed to fetch champions:", error);
        throw error;
    }
}

/**
 * Fetch with a timeout (no retries — fail fast).
 */
async function fetchWithTimeout<T>(url: string, timeoutMs = 8000): Promise<T> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
        const response = await fetch(url, { signal: controller.signal });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const text = await response.text();
        return JSON.parse(text) as T;
    } finally {
        clearTimeout(timer);
    }
}

interface CDragonSkinData {
    id: number;
    name?: string;
    isBase?: boolean;
    splashPath?: string;
    tilePath?: string;
}

function mapCDragonSkins(skins: CDragonSkinData[]): DDragonSkin[] {
    return skins.map(skin => ({
        id: skin.id,
        name: skin.name || `Skin ${skin.id}`,
        num: skin.id % 1000,
        isBase: skin.isBase || skin.id % 1000 === 0,
        splashPath: skin.splashPath,
        tilePath: skin.tilePath
    }));
}

/**
 * Fetch skins for a specific champion.
 * Tries CommunityDragon first, falls back to DataDragon.
 * Throws on total failure so the caller can show an error.
 */
export async function fetchChampionSkins(championId: number, alias?: string): Promise<DDragonSkin[]> {
    const errors: string[] = [];

    // Try CommunityDragon
    try {
        const url = `${CDRAGON_BASE_URL}/champions/${championId}.json`;
        const champion = await fetchWithTimeout<{ skins?: CDragonSkinData[] }>(url);
        if (champion.skins && champion.skins.length > 0) {
            return mapCDragonSkins(champion.skins);
        }
    } catch (err) {
        errors.push(`CDragon: ${err instanceof Error ? err.message : err}`);
    }

    // Fallback: DataDragon
    if (alias) {
        try {
            const patch = await getLatestPatch();
            const url = `${DDRAGON_BASE_URL}/cdn/${patch}/data/en_US/champion/${alias}.json`;
            const response = await fetchWithTimeout<{
                data?: Record<string, { skins?: Array<{ id: string; num: number; name: string }> }>;
            }>(url);
            const champData = response.data?.[alias];
            if (champData?.skins && champData.skins.length > 0) {
                return champData.skins.map(skin => ({
                    id: parseInt(skin.id, 10),
                    name: skin.name === 'default' ? alias : skin.name,
                    num: skin.num,
                    isBase: skin.num === 0,
                }));
            }
        } catch (err) {
            errors.push(`DDragon: ${err instanceof Error ? err.message : err}`);
        }
    }

    // If both failed, throw so caller can surface the error
    if (errors.length > 0) {
        throw new Error(`Failed to fetch skins: ${errors.join('; ')}`);
    }

    return [{ id: championId * 1000, name: 'Base', num: 0, isBase: true }];
}

/**
 * Get champion icon URL from CommunityDragon
 */
export function getChampionIconUrl(championId: number): string {
    return `${CDRAGON_BASE_URL}/champion-icons/${championId}.png`;
}

/**
 * Get skin splash URL from DataDragon
 */
export function getSkinSplashUrl(alias: string, skinNum: number): string {
    return `${DDRAGON_BASE_URL}/cdn/img/champion/splash/${alias}_${skinNum}.jpg`;
}

/**
 * Get skin splash URL from CommunityDragon (fallback)
 */
export function getSkinSplashCDragonUrl(championId: number, skinId: number): string {
    return `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-splashes/${championId}/${skinId}.jpg`;
}

/**
 * Clear cached data (useful if user wants to refresh)
 */
export function clearCache(): void {
    cachedPatch = null;
    cachedChampions = null;
    for (const blobUrl of imageBlobCache.values()) {
        URL.revokeObjectURL(blobUrl);
    }
    imageBlobCache.clear();
    imageFetchQueue.clear();
}
