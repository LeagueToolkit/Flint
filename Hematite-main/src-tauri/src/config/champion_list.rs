//! Champion List Manager
//!
//! Fetches and caches champion/subchamp data from GitHub for context-aware
//! pattern matching. Used to determine if a skin belongs to a champion or subchamp.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::cache;

/// GitHub URL for champion list (hosted in main Hematite repo)
const CHAMPION_LIST_URL: &str = "https://raw.githubusercontent.com/RitoShark/Hematite/main/config/champion_list.json";

/// Cache TTL for champion list (1 week - champions don't change often)
const CHAMPION_CACHE_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// Champion list with all champions, subchamps, and known HP bar values.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChampionList {
    /// Schema version
    pub version: String,
    
    /// List of all champion names
    pub champions: Vec<String>,
    
    /// Map of champion name -> subchamp names
    /// e.g., "Aphelios" -> ["ApheliosTurret"]
    pub subchamps: HashMap<String, Vec<String>>,
    
    /// Known healthbar style values for specific characters/units.
    /// Used for the healthbar fix when a character needs a specific value.
    /// Key: lowercase character name, Value: UnitHealthBarStyle value
    #[serde(default)]
    pub healthbar_values: HashMap<String, u8>,
}

impl ChampionList {
    /// Create an empty champion list
    pub fn new() -> Self {
        Self {
            version: "0.0.0".to_string(),
            champions: Vec::new(),
            subchamps: HashMap::new(),
            healthbar_values: HashMap::new(),
        }
    }

    /// Load champion list from GitHub or cache.
    ///
    /// Strategy:
    /// 1. Try cache first (if < 1 week old)
    /// 2. Try HTTP fetch from GitHub
    /// 3. Fall back to embedded default list
    pub fn load() -> Result<Self> {
        // Try cache first
        if let Ok(cache_path) = cache::get_cache_path("champion_list.json") {
            if let Ok(cached) = cache::read_cached_file::<Self>(&cache_path, CHAMPION_CACHE_TTL_SECS) {
                log::info!("Loaded champion list from cache (version {})", cached.version);
                return Ok(cached);
            }
        }

        // Try HTTP fetch
        match Self::fetch_from_github() {
            Ok(list) => {
                // Cache the result
                if let Ok(cache_path) = cache::get_cache_path("champion_list.json") {
                    if let Err(e) = cache::write_cache_file(&cache_path, &list) {
                        log::warn!("Failed to cache champion list: {}", e);
                    }
                }
                log::info!("Fetched champion list from GitHub (version {})", list.version);
                return Ok(list);
            }
            Err(e) => {
                log::warn!("Failed to fetch champion list from GitHub: {}", e);
            }
        }

        // Fall back to embedded default
        log::info!("Using embedded champion list");
        Ok(Self::embedded_default())
    }

    /// Fetch champion list from GitHub
    fn fetch_from_github() -> Result<Self> {
        let response = reqwest::blocking::get(CHAMPION_LIST_URL)
            .context("Failed to fetch champion list from GitHub")?;
        
        if !response.status().is_success() {
            anyhow::bail!("GitHub returned status: {}", response.status());
        }
        
        let list: Self = response.json()
            .context("Failed to parse champion list JSON")?;
        
        Ok(list)
    }

    /// Embedded default champion list for offline use.
    /// This should be updated periodically.
    fn embedded_default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            champions: vec![
                "Aatrox", "Ahri", "Akali", "Akshan", "Alistar", "Ambessa", "Amumu", 
                "Anivia", "Annie", "Aphelios", "Ashe", "AurelionSol", "Aurora", "Azir",
                "Bard", "Belveth", "Blitzcrank", "Brand", "Braum", "Briar",
                "Caitlyn", "Camille", "Cassiopeia", "Chogath", "Corki",
                "Darius", "Diana", "DrMundo", "Draven",
                "Ekko", "Elise", "Evelynn", "Ezreal",
                "Fiddlesticks", "Fiora", "Fizz",
                "Galio", "Gangplank", "Garen", "Gnar", "Gragas", "Graves", "Gwen",
                "Hecarim", "Heimerdinger", "Hwei",
                "Illaoi", "Irelia", "Ivern",
                "Janna", "JarvanIV", "Jax", "Jayce", "Jhin", "Jinx",
                "Kaisa", "Kalista", "Karma", "Karthus", "Kassadin", "Katarina", 
                "Kayle", "Kayn", "Kennen", "Khazix", "Kindred", "Kled", "KogMaw", "Ksante",
                "Leblanc", "LeeSin", "Leona", "Lillia", "Lissandra", "Lucian", "Lulu", "Lux",
                "Malphite", "Malzahar", "Maokai", "MasterYi", "Milio", "MissFortune", "Mordekaiser", "Morgana",
                "Naafiri", "Nami", "Nasus", "Nautilus", "Neeko", "Nidalee", "Nilah", "Nocturne", "Nunu",
                "Olaf", "Orianna", "Ornn",
                "Pantheon", "Poppy", "Pyke",
                "Qiyana", "Quinn",
                "Rakan", "Rammus", "RekSai", "Rell", "Renata", "Renekton", "Rengar", "Riven", "Rumble", "Ryze",
                "Samira", "Sejuani", "Senna", "Seraphine", "Sett", "Shaco", "Shen", "Shyvana", 
                "Singed", "Sion", "Sivir", "Skarner", "Smolder", "Sona", "Soraka", "Swain", "Sylas", "Syndra",
                "TahmKench", "Taliyah", "Talon", "Taric", "Teemo", "Thresh", "Tristana", "Trundle", "Tryndamere", "TwistedFate", "Twitch",
                "Udyr", "Urgot",
                "Varus", "Vayne", "Veigar", "Velkoz", "Vex", "Vi", "Viego", "Viktor", "Vladimir", "Volibear",
                "Warwick", "Wukong",
                "Xayah", "Xerath", "XinZhao",
                "Yasuo", "Yone", "Yorick", "Yuumi",
                "Zac", "Zed", "Zeri", "Ziggs", "Zilean", "Zoe", "Zyra",
            ].into_iter().map(String::from).collect(),
            subchamps: [
                ("Aphelios", vec!["ApheliosTurret"]),
                ("Azir", vec!["AzirSoldier", "AzirSunDisc"]),
                ("Elise", vec!["EliseSpider", "EliseSpiderling"]),
                ("Heimerdinger", vec!["HeimerdingerTurret"]),
                ("Illaoi", vec!["IllaoiTentacle"]),
                ("Ivern", vec!["Daisy"]),
                ("Kalista", vec!["KalistaSpawn"]),
                ("Kindred", vec!["Lamb", "Wolf"]),
                ("Kled", vec!["Skaarl"]),
                ("Malzahar", vec!["MalzaharVoidling"]),
                ("Maokai", vec!["MaokaiSapling"]),
                ("Naafiri", vec!["NaafiriPackmate"]),
                ("Nidalee", vec!["NidaleeCougar"]),
                ("Nunu", vec!["Willump"]),
                ("Orianna", vec!["OriannaBall"]),
                ("Quinn", vec!["Valor"]),
                ("RekSai", vec!["RekSaiTunnel"]),
                ("Shaco", vec!["ShacoBox", "ShacoClone"]),
                ("Syndra", vec!["SyndraSphere"]),
                ("Teemo", vec!["TeemoMushroom"]),
                ("Yorick", vec!["YorickBigGhoul", "YorickGhoulMelee", "YorickMaiden"]),
                ("Zac", vec!["ZacBlob"]),
                ("Zed", vec!["ZedShadow"]),
                ("Zyra", vec!["ZyraPlant", "ZyraSeed"]),
            ].into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().map(String::from).collect()))
            .collect(),
            // Known healthbar style values for specific characters/units
            healthbar_values: [
                // Map entities
                ("turretrubble", 3), ("nexus", 8), ("turret", 8), ("inhibitor", 3),
                ("voidgate", 1), ("trundlewall", 1), ("noxtorra", 3),
                // Jungle monsters
                ("sru_crabward", 6), ("sru_crab", 5), ("sru_red", 5), ("sru_blue", 5),
                ("sru_gromp", 5), ("sru_murkwolf", 5), ("sru_razorbeak", 5), ("sru_krug", 5),
                ("sru_baron", 7), ("sru_riftherald", 5), ("sru_riftherald_mercenary", 5),
                ("sru_atakhan", 5), ("sru_atakhan_crown", 22), ("sru_horde", 5),
                ("sru_jungle_companions", 5),
                // Dragons
                ("sru_dragon", 5), ("sru_dragon_elder", 7), ("sru_dragon_earth", 5),
                ("sru_dragon_fire", 5), ("sru_dragon_water", 5), ("sru_dragon_air", 5),
                ("sru_dragon_chemtech", 5), ("sru_dragon_hextech", 5), ("sru_dragon_ruined", 5),
                // Plants
                ("sru_plant_satchel", 6), ("sru_plant_demon", 6), ("sru_plant_health", 6),
                ("sru_plant_vision", 6), ("cherry_plant_powerup", 6), ("cherry_plant_bridge", 6),
                ("slime_plant_satchel", 6),
                // Minions
                ("sru_chaosminionsuper", 1), ("sru_orderminionsuper", 1),
                ("ha_chaosminionsuper", 1), ("ha_orderminionsuper", 1),
                // Game mode specific - Arena/Cherry
                ("cherry_feeneypult", 3), ("urf_feeneypult", 3), ("cherry_battlesled", 3),
                ("cherry_audiencemale", 5), ("cherry_audiencefemale", 5),
                ("cherry_goh_atakhan", 5), ("cherry_destructible_column", 3),
                ("cherry_447122_blackhole", 5),
                // Swarm/Strawberry mode
                ("strawberry_miniboss_rampingspeed", 7), ("strawberry_miniboss_bruiser", 7),
                ("strawberry_miniboss_artillery", 7), ("strawberry_miniboss_dasher", 7),
                ("strawberry_miniboss_shooter", 7), ("strawberry_boss_belveth", 7),
                ("strawberry_boss_reksai", 7), ("strawberry_boss_briar", 7),
                ("strawberry_boss_aatrox", 7), ("strawberry_tibbers", 5),
                ("strawberry_enemy_exploder", 22), ("strawberry_enemy_fast", 22),
                ("strawberry_enemy_poisonous", 22), ("strawberry_enemy_shield", 22),
                ("strawberry_enemy_divider", 22), ("strawberry_enemy_lancer", 22),
                ("strawberry_enemy_hunter", 22), ("strawberry_enemy_swarmer", 22),
                ("strawberry_enemy_cannon", 22), ("strawberry_enemy_shy", 22),
                ("strawberry_enemy_minidasher", 22), ("strawberry_enemy_minireksai", 22),
                ("strawberry_enemy_belvethspawn_dasher", 22),
                ("strawberry_enemy_basicmelee1_pix", 22), ("strawberry_enemy_basicmelee2_ghoul", 22),
                ("strawberry_enemy_basicmelee3_fuemigo", 22), ("strawberry_enemy_basicmelee4_red", 22),
                ("strawberry_enemy_bruisermelee1_tibbers", 22), ("strawberry_enemy_bruisermelee2_daisy", 22),
                ("strawberry_poi_turretbase", 6), ("strawberry_poi_mapb_turret", 6),
                ("strawberry_poi_mapd_turret", 6), ("strawberry_poi_turretbase_mapb", 6),
                ("strawberry_boss_reksai_tunnel", 5), ("strawberry_boss_belveth_coral", 6),
                ("strawberry_boss_aatrox_chainpillar", 6), ("strawberry_spire_vaseobject", 6),
                ("strawberry_destructible_metalbox", 6), ("strawberry_eventenemy_lootchonklin", 5),
                // Nexus Blitz
                ("nexusblitz_lootgoblin", 5), ("nexusblitz_nexuswalking", 5),
                ("nexusblitz_nexusrooted", 5), ("nexusblitz_jungleguardian", 5),
                ("nexusblitz_sorakabot", 5), ("nexusblitz_shopkeeper", 21),
                // Slime mode
                ("slime_environmentminion", 5), ("slime_scuttleracer", 6),
                ("slime_crabward", 6), ("slime_crab", 6), ("slime_warthog", 3),
                ("slime_kingporo", 5),
                // ARAM
                ("ha_ap_chaosturretshrine", 3), ("ha_ap_ordershrineturret", 3),
                ("kingporo", 5),
                // Crepe/Durian modes
                ("crepe_piximander", 5), ("crepe_chainsaw", 5), ("crepe_shooter", 5),
                ("crepe_gremlin", 5), ("crepe_fish", 5), ("crepe_blue_monkey", 5),
                ("crepe_beardy", 5), ("crepe_flyguy", 5), ("crepe_lieutenant", 5),
                ("durian_scuttletrainengine", 5), ("durian_scuttletraincompartment", 1),
                // Brawl mode
                ("brawl_turret", 8), ("brawl_shopkeeper_kobuko", 22),
                ("brawl_shopkeeper_norra", 15), ("brawl_environment_yuumi", 22),
                // SR special
                ("sr_infernalrift_crystalmeep", 5), ("nightmarebots_malzahar_riftherald", 5),
                ("ultbookannietibbers", 5),
                // Champion summons/subchamps with specific values
                ("annietibbers", 5), ("azirsundisc", 3), ("ireliablades", 5),
                ("ivernminion", 5), ("naafiripackmate", 9), ("yorickbigghoul", 5),
                ("zacrebirthbloblet", 1),
                // TFT Champions
                ("tftchampion", 15), ("tft_teamcannon", 22), ("tft12_sylas", 15),
                ("tft13_augment", 17), ("tft13_goop", 17), ("tft3_5_augment", 17),
                ("tft5_hellionprop", 15), ("tft5_radiantnpc", 21),
                ("tftevent5yr_cupcakenpc", 15), ("tftevent5yr_birthdaypengunpc", 15),
                ("tftevent5yr_crabnpc", 15), ("tftevent_ct_chonccnpc", 15),
                ("tftevents_lunar2023_bunny", 21),
                // TFT Tutorial units
                ("tfttutorial_kassadin", 15), ("tfttutorial_galio", 15), ("tfttutorial_warwick", 15),
                ("tfttutorial_nidaleecougar", 15), ("tfttutorial_nidalee", 15),
                ("tfttutorial_veigar", 15), ("tfttutorial_blitzcrank", 15),
                ("tfttutorial_fiora", 15), ("tfttutorial_elisespiderling", 15),
                ("tfttutorial_evelynn", 15), ("tfttutorial_darius", 15),
                ("tfttutorial_vi", 15), ("tfttutorial_twistedfate", 15),
                ("tfttutorial_jayce", 15), ("tfttutorial_elisespider", 15),
                ("tfttutorial_gangplankbarrel", 15), ("tfttutorial_lulu", 15),
                ("tfttutorial_leona", 15), ("tfttutorial_camille", 15),
                ("tfttutorial_jinx", 15), ("tfttutorial_jayceranged", 15),
                ("tfttutorial_tristana", 15), ("tfttutorial_kayle", 15),
                ("tfttutorial_ahri", 15), ("tfttutorial_zed", 15),
                ("tfttutorial_reksai", 15), ("tfttutorial_gangplank", 15),
                ("tfttutorial_varus", 15), ("tfttutorial_vayne", 15),
                ("tfttutorial_shen", 15), ("tfttutorial_garen", 15),
                ("tfttutorial_akali", 15), ("tfttutorial_lissandra", 15),
                ("tfttutorial_braum", 15), ("tfttutorial_katarina", 15),
                ("tfttutorial_elise", 15), ("tfttutorial_lucian", 15),
                ("tfttutorial_graves", 15), ("tfttutorial_ashe", 15),
                ("tfttutorial_khazix", 15), ("tfttutorial_pyke", 15),
                // TFT Pets (Little Legends) - all use 21
                ("petpenguknight", 21), ("petchibiblitzcrank", 21), ("petbunny", 21),
                ("petchibiseraphine", 21), ("petgargoyle", 21), ("petbigknifedog", 21),
                ("petshark", 21), ("petsgpig", 21), ("petscuttlecrab", 21),
                ("petchibilulu", 21), ("petchibigwen", 21), ("petumbra", 21),
                ("petpegasus", 21), ("petgloop", 21), ("petsgcat", 21),
                ("petaoshin", 21), ("petchibimalphite", 21), ("petfairy", 21),
                ("pettftavatar", 21), ("petstyletwothresh", 21), ("petchoncc", 21),
                ("petpiximander", 21), ("petghosty", 21), ("petsennabunny", 21),
                ("petchibiyone", 21), ("petduckbill", 21), ("petgemtiger", 21),
                ("petdssquid", 21), ("petchibiyuumi", 21), ("pethextechbirb", 21),
                ("petsnake", 21), ("petchip", 21), ("petnimblefoot", 21),
                ("petturtle", 21), ("petbat", 21), ("petdowsie", 21),
                ("petchibiteemo", 21), ("petkiko", 21), ("petminigolem", 21),
                ("petchibiannie", 21), ("petminer", 21), ("petbuglet", 21),
                ("petstyletwojinx", 21), ("petrazorbeak", 21), ("petelegantdragon", 21),
                ("petqiyanadog", 21), ("petstyletwogaren", 21), ("petkoala", 21),
                ("petspiritfox", 21), ("petchibiyasuo", 21), ("petchibitristana", 21),
                ("petchibized", 21), ("petgrumpylion", 21), ("petgriffin", 21),
                ("petfenroar", 21), ("petbellswayer", 21), ("petchibizoe", 21),
                ("petpupdragon", 21), ("petjawdragon", 21), ("petsgshisa", 21),
                ("petchibijinx", 21), ("petchibimissfortune", 21), ("petchibivi", 21),
                ("petchibiahri", 21), ("petdsswordguy", 21), ("petowl", 21),
                ("petbaron", 21), ("petakalidragon", 21), ("petchibiirelia", 21),
                ("pethundun", 21), ("petkuroshiro", 21), ("petchibijanna", 21),
                ("petleopardgecko", 21), ("petchibiriven", 21), ("petchibiorianna", 21),
                ("petdswhale", 21), ("petchibikatarina", 21), ("petchibiakali", 21),
                ("petchibilillia", 21), ("petcreepycat", 21), ("petsmolknifedog", 21),
                ("petchickeneggtart", 21), ("petchibikaisa", 21), ("petfishball", 21),
                ("petballdragon", 21), ("petvoideye", 21), ("petchibilux", 21),
                ("petporo", 21), ("petminion", 21), ("petstyletwojhin", 21),
                ("petchibisett", 21), ("petcupcake", 21), ("petchibimorgana", 21),
                ("petscratch", 21), ("pethamster", 21), ("petchibileesin", 21),
                ("petstyletwovayne", 21), ("petchibikayle", 21), ("petmoth", 21),
                ("petchibiekko", 21), ("petchibiaatrox", 21), ("petchibicaitlyn", 21),
                ("petchibibriar", 21), ("petchibiashe", 21), ("petdoughcat", 21),
                ("petseaangel", 21), ("petchibiezreal", 21), ("petchibiamumu", 21),
                ("petchibisona", 21), ("petstyletwowarwick", 21), ("petporofluft", 21),
            ].into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        }
    }

    /// Check if a name is a champion (from static list)
    pub fn is_champion(&self, name: &str) -> bool {
        self.champions.iter().any(|c| c.eq_ignore_ascii_case(name))
    }

    /// Check if a name is a subchamp (from static list fallback)
    pub fn is_subchamp(&self, name: &str) -> bool {
        self.subchamps.values()
            .any(|subs| subs.iter().any(|s| s.eq_ignore_ascii_case(name)))
    }

    /// Get parent champion for a subchamp
    pub fn get_parent(&self, subchamp: &str) -> Option<String> {
        for (parent, subs) in &self.subchamps {
            if subs.iter().any(|s| s.eq_ignore_ascii_case(subchamp)) {
                return Some(parent.clone());
            }
        }
        None
    }

    // =========================================================================
    // HEALTHBAR VALUES
    // =========================================================================

    /// Get the known healthbar style value for a character/unit.
    ///
    /// # Arguments
    /// * `name` - Character name (case-insensitive, e.g., "AnnIeTibBers" or "sru_baron")
    ///
    /// # Returns
    /// - `Some(value)` if a known value exists for this character
    /// - `None` if no known value (should use default = 12 for champions)
    pub fn get_healthbar_value(&self, name: &str) -> Option<u8> {
        let lower = name.to_lowercase();
        self.healthbar_values.get(&lower).copied()
    }

    /// Get the healthbar style value for a character, or return default.
    ///
    /// # Arguments
    /// * `name` - Character name (case-insensitive)
    /// * `default` - Default value to use if no known value exists
    pub fn get_healthbar_value_or(&self, name: &str, default: u8) -> u8 {
        self.get_healthbar_value(name).unwrap_or(default)
    }

    /// Extract champion/subchamp name from BIN entry path.
    ///
    /// Examples:
    /// - "Characters/Jhin/Skins/Skin0" -> Some("Jhin")
    /// - "Characters/ZedShadow/Skins/Skin0" -> Some("ZedShadow")
    /// - "Items/SomeItem" -> None
    pub fn extract_name_from_path(path: &str) -> Option<String> {
        let normalized = path.replace('\\', "/");
        let parts: Vec<&str> = normalized.split('/').collect();
        
        if parts.len() >= 2 && parts[0].eq_ignore_ascii_case("Characters") {
            Some(parts[1].to_string())
        } else {
            None
        }
    }

    // =========================================================================
    // BIN-BASED DETECTION (Primary Method)
    // =========================================================================

    /// Check if a BIN file represents a subchamp by checking for missing Loadscreen field.
    ///
    /// **Detection Logic:**
    /// - Main champions have `Loadscreen` field in `SkinCharacterDataProperties`
    /// - Subchamps (ZedShadow, NaafiriPackmate, etc.) do NOT have `Loadscreen`
    ///
    /// This is the PRIMARY detection method - no external list needed!
    ///
    /// # Arguments
    /// * `bin_entries` - Iterator of entry types and their field hashes from a parsed BIN
    ///
    /// # Returns
    /// - `Some(true)` if it's a subchamp (no Loadscreen field found)
    /// - `Some(false)` if it's a main champion (has Loadscreen field)
    /// - `None` if no SkinCharacterDataProperties entries found
    pub fn is_subchamp_by_bin<'a, I>(skin_character_entries: I) -> Option<bool>
    where
        I: Iterator<Item = &'a [u32]>, // Field hashes for each SkinCharacterDataProperties entry
    {
        // Loadscreen field hash - this is the key to detection
        // TODO: Get actual hash value from league_toolkit or compute xxhash
        const LOADSCREEN_HASH: u32 = 0; // Placeholder - will be computed

        let mut found_any_entry = false;
        let mut is_sub = false;

        for field_hashes in skin_character_entries {
            found_any_entry = true;
            // If ANY entry has Loadscreen, it's not a subchamp
            // If NO entries have Loadscreen, it IS a subchamp
            is_sub = !field_hashes.contains(&LOADSCREEN_HASH);
        }

        if found_any_entry {
            Some(is_sub)
        } else {
            None
        }
    }

    // =========================================================================
    // CONTEXT DETECTION (Hybrid: BIN-first, then fallback to static list)
    // =========================================================================

    /// Determine the context type for a path (fallback method using static list).
    ///
    /// **For BIN-based detection, use `is_subchamp_by_bin()` first!**
    ///
    /// Returns:
    /// - Some("champion") if it's a main champion
    /// - Some("subchamp") if it's a subchamp (like ZedShadow)
    /// - None if unknown
    pub fn get_context(&self, path: &str) -> Option<String> {
        let name = Self::extract_name_from_path(path)?;
        
        if self.is_champion(&name) {
            Some("champion".to_string())
        } else if self.is_subchamp(&name) {
            Some("subchamp".to_string())
        } else {
            None
        }
    }

    /// Determine context using BIN data first, then fall back to static list.
    ///
    /// **Recommended method for production use.**
    ///
    /// # Arguments
    /// * `path` - Entry path like "Characters/Jhin/Skins/Skin0"
    /// * `bin_result` - Result from `is_subchamp_by_bin()`, or None if BIN not parsed
    ///
    /// # Returns
    /// - "champion" if it's a main champion
    /// - "subchamp" if it's a subchamp
    /// - "unknown" if can't determine
    pub fn get_context_with_bin(&self, path: &str, bin_result: Option<bool>) -> String {
        // BIN-based detection takes priority (most accurate)
        if let Some(is_sub) = bin_result {
            return if is_sub { "subchamp" } else { "champion" }.to_string();
        }

        // Fall back to static list
        self.get_context(path).unwrap_or_else(|| "unknown".to_string())
    }
}

impl Default for ChampionList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_champion() {
        let list = ChampionList::embedded_default();
        assert!(list.is_champion("Jhin"));
        assert!(list.is_champion("JHIN")); // Case insensitive
        assert!(list.is_champion("jhin"));
        assert!(!list.is_champion("ZedShadow")); // Subchamp, not champion
    }

    #[test]
    fn test_is_subchamp() {
        let list = ChampionList::embedded_default();
        assert!(list.is_subchamp("ZedShadow"));
        assert!(list.is_subchamp("zedshadow")); // Case insensitive
        assert!(list.is_subchamp("NaafiriPackmate"));
        assert!(!list.is_subchamp("Jhin")); // Champion, not subchamp
    }

    #[test]
    fn test_get_parent() {
        let list = ChampionList::embedded_default();
        assert_eq!(list.get_parent("ZedShadow"), Some("Zed".to_string()));
        assert_eq!(list.get_parent("NaafiriPackmate"), Some("Naafiri".to_string()));
        assert_eq!(list.get_parent("Jhin"), None); // Not a subchamp
    }

    #[test]
    fn test_extract_name_from_path() {
        assert_eq!(
            ChampionList::extract_name_from_path("Characters/Jhin/Skins/Skin0"),
            Some("Jhin".to_string())
        );
        assert_eq!(
            ChampionList::extract_name_from_path("Characters/ZedShadow/Skins/Skin0"),
            Some("ZedShadow".to_string())
        );
        assert_eq!(
            ChampionList::extract_name_from_path("Items/SomeItem"),
            None
        );
        // Also works with backslashes
        assert_eq!(
            ChampionList::extract_name_from_path("Characters\\Jhin\\Skins\\Skin0"),
            Some("Jhin".to_string())
        );
    }

    #[test]
    fn test_get_context() {
        let list = ChampionList::embedded_default();
        
        assert_eq!(
            list.get_context("Characters/Jhin/Skins/Skin0"),
            Some("champion".to_string())
        );
        assert_eq!(
            list.get_context("Characters/ZedShadow/Skins/Skin0"),
            Some("subchamp".to_string())
        );
        assert_eq!(
            list.get_context("Items/SomeItem"),
            None
        );
    }

    #[test]
    fn test_is_subchamp_by_bin() {
        // Simulate field hashes for main champion (has Loadscreen - hash 0 is placeholder)
        let champion_fields: Vec<Vec<u32>> = vec![vec![0, 123, 456]]; // Contains Loadscreen (0)
        let result = ChampionList::is_subchamp_by_bin(
            champion_fields.iter().map(|v| v.as_slice())
        );
        assert_eq!(result, Some(false)); // Not a subchamp

        // Simulate field hashes for subchamp (no Loadscreen)
        let subchamp_fields: Vec<Vec<u32>> = vec![vec![123, 456, 789]]; // No Loadscreen
        let result = ChampionList::is_subchamp_by_bin(
            subchamp_fields.iter().map(|v| v.as_slice())
        );
        assert_eq!(result, Some(true)); // Is a subchamp

        // No entries found
        let empty: Vec<Vec<u32>> = vec![];
        let result = ChampionList::is_subchamp_by_bin(
            empty.iter().map(|v| v.as_slice())
        );
        assert_eq!(result, None); // Unknown
    }

    #[test]
    fn test_get_context_with_bin() {
        let list = ChampionList::embedded_default();

        // BIN says subchamp - should override static list
        assert_eq!(
            list.get_context_with_bin("Characters/Jhin/Skins/Skin0", Some(true)),
            "subchamp"
        );

        // BIN says champion - should use that
        assert_eq!(
            list.get_context_with_bin("Characters/ZedShadow/Skins/Skin0", Some(false)),
            "champion"
        );

        // No BIN result - fall back to static list
        assert_eq!(
            list.get_context_with_bin("Characters/Jhin/Skins/Skin0", None),
            "champion"
        );
        assert_eq!(
            list.get_context_with_bin("Characters/ZedShadow/Skins/Skin0", None),
            "subchamp"
        );

        // Unknown character - returns "unknown"
        assert_eq!(
            list.get_context_with_bin("Characters/UnknownChamp/Skins/Skin0", None),
            "unknown"
        );
    }

    #[test]
    fn test_get_healthbar_value() {
        let list = ChampionList::embedded_default();

        // Known values - case insensitive
        assert_eq!(list.get_healthbar_value("annietibbers"), Some(5));
        assert_eq!(list.get_healthbar_value("AnnieTibbers"), Some(5));
        assert_eq!(list.get_healthbar_value("ANNIETIBBERS"), Some(5));
        
        assert_eq!(list.get_healthbar_value("sru_baron"), Some(7));
        assert_eq!(list.get_healthbar_value("naafiripackmate"), Some(9));
        assert_eq!(list.get_healthbar_value("petporo"), Some(21));
        assert_eq!(list.get_healthbar_value("tftchampion"), Some(15));
        
        // Unknown - returns None
        assert_eq!(list.get_healthbar_value("jhin"), None); // Champions use default
        assert_eq!(list.get_healthbar_value("unknown_thing"), None);

        // With default fallback
        assert_eq!(list.get_healthbar_value_or("sru_baron", 12), 7);
        assert_eq!(list.get_healthbar_value_or("jhin", 12), 12); // Uses default
    }
}
