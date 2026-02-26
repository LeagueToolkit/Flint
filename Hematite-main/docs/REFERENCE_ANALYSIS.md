# Reference Code Analysis

> **Analyzed:** 14 Python files in `Reference-Code/` directory  
> **Date:** 2026-01-11  
> **Status:** ✅ Complete

---

## File Inventory

```
Reference-Code/
├── main.py                    # FastAPI orchestration + detection functions
├── bnk_json_parser.py         # BNK audio file parser utility
├── logging_config.py          # Logging configuration
├── fixes/
│   ├── genericfix.py          # Base class for all fixes
│   ├── fixhealthbar.py        # Health bar style fix
│   ├── fixstaticmat.py        # TextureName → TexturePath rename
│   ├── fixstaticmat2.py       # SamplerName → TextureName rename
│   ├── fixblackicon.py        # Black icon .dds → .tex conversion
│   ├── dds_to_tex_converter.py # DDS to TEX particle effects fix
│   ├── fixvfxshape.py         # VFX shape patch 14.1 fix (COMPLEX)
│   ├── championbinremover.py  # Champion bin file removal
│   ├── bnkremover.py          # BNK audio file removal by version
│   ├── fantome_branding.py    # Fantome metadata branding (non-critical)
│   └── wadcache.py            # WAD file caching system
```

---

## Fix Analysis: Health Bar Fix

**File:** `fixes/fixhealthbar.py`

### What It Detects

- **Condition:** Missing `UnitHealthBarStyle` field or incorrect value in `SkinCharacterDataProperties`
- **File Type:** `.bin`
- **BIN Path Pattern:** Any skin bin containing `SkinCharacterDataProperties`

### What It Fixes

- **Action:** Adds or modifies `UnitHealthBarStyle` field with value `12`
- **Implementation:** Searches for `SkinCharacterDataProperties` entries, checks for `CharacterHealthBarDataRecord` embed, adds/fixes `UnitHealthBarStyle`

### Key Code Snippets

```python
HEALTHBAR_NUMBER = 12

# Detection: Check if HealthBarData exists
has_healthbardata_flag = any(i.hash_type == BIN_HASH["CharacterHealthBarDataRecord"] for i in entry.data)

if not has_healthbardata_flag:
    # Fix: Append HealthBarData with UnitHealthBarStyle
    entry.data.append(HealthBarData)
else:
    # Or fix: Change existing value to 12
    inside_healthbar.data = HEALTHBAR_NUMBER
```

### BIN Fields Involved

| Field Name | Data Type | Detection Value | Fix Value |
|------------|-----------|-----------------|-----------|
| `UnitHealthBarStyle` | `u8` | missing or ≠12 | `12` |
| `HealthBarData` | `embed` | missing | Added with child |
| `CharacterHealthBarDataRecord` | `embed` (type) | - | - |
| `SkinCharacterDataProperties` | `embed` (type) | - | - |

### Translation to JSON Schema

```json
{
  "fix_id": "healthbar_fix",
  "name": "Missing HP Bar",
  "detect": {
    "type": "missing_or_wrong_field",
    "entry_type": "SkinCharacterDataProperties",
    "embed_type": "CharacterHealthBarDataRecord",
    "field": "UnitHealthBarStyle",
    "expected_value": 12
  },
  "apply": {
    "type": "add_or_set_field",
    "field": "UnitHealthBarStyle",
    "value": 12,
    "data_type": "u8",
    "parent_embed": {
      "field": "HealthBarData",
      "type": "CharacterHealthBarDataRecord"
    }
  }
}
```

### Complexity Rating

- [x] Simple (direct field add/change)
- [ ] Medium
- [ ] Complex

---

## Fix Analysis: Static Material Fix 1

**File:** `fixes/fixstaticmat.py`

### What It Detects

- **Condition:** `TextureName` field exists in `StaticMaterialShaderSamplerDef` (should be `TexturePath`)
- **File Type:** `.bin`

### What It Fixes

- **Action:** Renames field hash from `TextureName` to `TexturePath`
- **Implementation:** Simple hash replacement

### Key Code Snippets

```python
for sampler_value in sampler_def.data:
    if sampler_value.hash == HASHES["TextureName"]:
        sampler_value.hash = HASHES["TexturePath"]
```

### BIN Fields Involved

| Field Name | Data Type | Detection | Fix |
|------------|-----------|-----------|-----|
| `TextureName` | `string` | exists | rename to `TexturePath` |
| `TexturePath` | `string` | - | target hash |
| `SamplerValues` | `list2[embed]` | - | parent container |
| `StaticMaterialDef` | entry type | - | entry filter |

### Translation to JSON Schema

```json
{
  "fix_id": "staticmat_texturepath",
  "name": "White Model (TextureName)",
  "detect": {
    "type": "field_exists",
    "entry_type": "StaticMaterialDef",
    "path": "SamplerValues.*.TextureName"
  },
  "apply": {
    "type": "rename_field_hash",
    "from": "TextureName",
    "to": "TexturePath"
  }
}
```

### Complexity Rating

- [x] Simple (hash rename)
- [ ] Medium
- [ ] Complex

---

## Fix Analysis: Static Material Fix 2

**File:** `fixes/fixstaticmat2.py`

### What It Detects

- **Condition:** `SamplerName` field exists (should be `TextureName`)
- **File Type:** `.bin`

### What It Fixes

- **Action:** Renames field hash from `SamplerName` to `TextureName`

### Key Code Snippets

```python
if sampler_value.hash == HASHES["SamplerName"]:
    sampler_value.hash = HASHES["TextureName"]
```

### Translation to JSON Schema

```json
{
  "fix_id": "staticmat_samplername",
  "name": "White Model (SamplerName)",
  "detect": {
    "type": "field_exists",
    "entry_type": "StaticMaterialDef",
    "path": "SamplerValues.*.SamplerName"
  },
  "apply": {
    "type": "rename_field_hash",
    "from": "SamplerName",
    "to": "TextureName"
  }
}
```

### Complexity Rating

- [x] Simple (hash rename)
- [ ] Medium
- [ ] Complex

---

## Fix Analysis: Black Icons Fix

**File:** `fixes/fixblackicon.py`

### What It Detects

- **Condition:** Icon fields (`iconAvatar`, `iconCircle`, `iconSquare`) reference `.dds` files that DON'T exist in the WAD
- **File Type:** `.bin`
- **Dependencies:** Requires WAD cache to check file existence

### What It Fixes

- **Action:** Changes `.dds` extension to `.tex` in string value
- **Implementation:** Uses xxhash to compute path hash, checks WAD cache, replaces extension

### Key Code Snippets

```python
# Detection: Check if icon path ends in .dds and file NOT in WAD
if isinstance(field.data, str) and field.data.lower().endswith('.dds'):
    path_hash = xxh64(path_normalized).hexdigest()
    if not self.wad_cache.has_dds_file(path_hash):
        # Fix: Replace .dds with .tex
        field.data = field.data[:-4] + '.tex'
```

### BIN Fields Involved

| Field Name | Data Type | Detection | Fix |
|------------|-----------|-----------|-----|
| `iconAvatar` | `string` | ends with `.dds` | replace with `.tex` |
| `iconCircle` | `string` | ends with `.dds` | replace with `.tex` |
| `iconSquare` | `string` | ends with `.dds` | replace with `.tex` |

### Translation to JSON Schema

```json
{
  "fix_id": "black_icons",
  "name": "Black/Missing Icons",
  "detect": {
    "type": "string_extension_not_in_wad",
    "entry_type": "SkinCharacterDataProperties",
    "fields": ["iconAvatar", "iconCircle", "iconSquare"],
    "extension": ".dds"
  },
  "apply": {
    "type": "replace_extension",
    "from": ".dds",
    "to": ".tex",
    "condition": "not_in_wad"
  }
}
```

### Complexity Rating

- [ ] Simple
- [x] Medium (requires WAD cache check)
- [ ] Complex

---

## Fix Analysis: DDS to TEX Converter

**File:** `fixes/dds_to_tex_converter.py`

### What It Detects

- **Condition:** ANY `.dds` string reference to official paths that don't exist in WAD
- **File Type:** `.bin`
- **Scope:** Broader than black icons - covers all texture references

### What It Fixes

- **Action:** Replaces `.dds` with `.tex` in texture path strings

### Key Code Snippets

```python
official_prefixes = [
    'assets/characters/',
    'assets/shared/materials/',
    'assets/shared/particles/',
    'assets/particles/'
]

if not is_official_path:
    return (0, 1)  # Skip custom paths

if not self.wad_cache.has_dds_file(path_hash):
    field.data = field.data[:-4] + '.tex'
```

### Translation to JSON Schema

```json
{
  "fix_id": "dds_to_tex",
  "name": "Broken Particle Effects",
  "detect": {
    "type": "recursive_string_search",
    "pattern": "*.dds",
    "path_prefixes": [
      "assets/characters/",
      "assets/shared/materials/",
      "assets/shared/particles/",
      "assets/particles/"
    ],
    "condition": "not_in_wad"
  },
  "apply": {
    "type": "replace_extension",
    "from": ".dds",
    "to": ".tex"
  }
}
```

### Complexity Rating

- [ ] Simple
- [x] Medium (recursive search + WAD check)
- [ ] Complex

---

## Fix Analysis: Champion Bin Remover

**File:** `fixes/championbinremover.py`

### What It Detects

- **Condition:** BIN files containing champion-specific entry types
- **File Type:** `.bin` files within WAD
- **Detection Method:** Entry type hash matching

### Champion Data Types (Entry Type Hashes)

```python
champion_data_types = {
    'SpellObject', 
    'StatStoneData', 
    'SkinCharacterMetaDataProperties', 
    'CharacterRecord', 
    'ChampionRuneRecommendationsContext', 
    'RecSpellRankUpInfoList', 
    'ItemRecommendationOverrideSet', 
    'ItemRecommendationContextList', 
    'StatStoneSet', 
    'AbilityObject'
}
```

### What It Fixes

- **Action:** REMOVES the entire BIN file from WAD
- **Implementation:** Filters out WAD chunks matching champion bin hashes

### Translation to JSON Schema

```json
{
  "fix_id": "champion_bin_remover",
  "name": "Champion Bin Files",
  "detect": {
    "type": "entry_type_exists",
    "entry_types": [
      "SpellObject",
      "StatStoneData", 
      "SkinCharacterMetaDataProperties",
      "CharacterRecord",
      "ChampionRuneRecommendationsContext",
      "RecSpellRankUpInfoList",
      "ItemRecommendationOverrideSet",
      "ItemRecommendationContextList",
      "StatStoneSet",
      "AbilityObject"
    ]
  },
  "apply": {
    "type": "remove_file_from_wad"
  }
}
```

### Complexity Rating

- [ ] Simple
- [x] Medium (WAD chunk removal)
- [ ] Complex

---

## Fix Analysis: BNK Remover

**File:** `fixes/bnkremover.py`

### What It Detects

- **Condition:** BNK files with version NOT in allowed list `[134, 145]`
- **File Type:** `.bnk` files within WAD
- **Detection Method:** Parse BKHD section header for version number

### BNK Version Parsing

```python
def parse_bnk_version(self, bnk_data: bytes) -> int:
    # Find BKHD section
    if section_id_str == 'BKHD':
        # Version is first 4 bytes after section header
        version = int.from_bytes(bnk_data[offset+8:offset+12], 'little')
        return version
```

### What It Fixes

- **Action:** REMOVES BNK files with incompatible versions
- **Allowed Versions:** `134`, `145` (keep these, remove others)

### Translation to JSON Schema

```json
{
  "fix_id": "bnk_remover",
  "name": "Broken Sound Files",
  "detect": {
    "type": "bnk_version_not_in",
    "allowed_versions": [134, 145]
  },
  "apply": {
    "type": "remove_file_from_wad"
  }
}
```

### Complexity Rating

- [ ] Simple
- [x] Medium (binary parsing + WAD removal)
- [ ] Complex

---

## Fix Analysis: VFX Shape Fix

**File:** `fixes/fixvfxshape.py`

### What It Detects

- **Condition:** VFX emitter shapes with old-format structures (pre-patch 14.1)
- **File Type:** `.bin` files containing VFX definitions
- **Issues Detected:**
  1. `BirthTranslation` inside `Shape` (should be outside)
  2. `EmitOffset`, `EmitRotationAngles`, `EmitRotationAxes` need conversion
  3. Shape structures containing `ConstantValue` or `Dynamics`

### Key Hashes (Hex Values)

```python
'NewBirthTranslation': 0x563d4a22
'NewShapeHash': 0x3bf0b4ed
# Shape type conversions:
# 0x3dbe415d - Radius/Height/Flags format
# 0xee39916f - Simple EmitOffset format
# 0x4f4e2ed7 - Default format
```

### What It Fixes

Complex multi-step transformation:

1. Move `BirthTranslation` from inside `Shape` to emitter level
2. Extract `Radius`/`Height` from `EmitOffset.ConstantValue.Vec3`
3. Convert shape hash type based on analyzed structure
4. Rebuild shape with new format

### Key Code Snippets

```python
# Move BirthTranslation outside Shape
if inside_of_shape.hash == self.vfx_hashes['BirthTranslation']:
    birth_translation = BINField()
    birth_translation.hash = self.vfx_hashes['NewBirthTranslation']  # 0x563d4a22
    birth_translation.type = BINType.EMBED
    birth_translation.hash_type = '68dc32b6'
    emitter.data.append(birth_translation)
    inside_of_shape.data = []

# Convert shape to new format
shape.hash = self.vfx_hashes['NewShapeHash']  # 0x3bf0b4ed
shape.type = BINType.POINTER
shape.hash_type = '3dbe415d'  # or 'ee39916f' or '4f4e2ed7'
```

### BIN Fields Involved

| Field Name | Data Type | Role |
|------------|-----------|------|
| `VfxSystemDefinitionData` | entry type | Container |
| `ComplexEmitterDefinitionData` | embed | Emitter container |
| `SimpleEmitterDefinitionData` | embed | Emitter container |
| `Shape` | pointer | Shape definition |
| `BirthTranslation` | embed | Particle birth position |
| `EmitOffset` | embed | Emission offset |
| `EmitRotationAngles` | embed | Rotation angles |
| `EmitRotationAxes` | list[vec3] | Rotation axes |
| `ConstantValue` | vec3 | Constant vector value |
| `Dynamics` | embed | Dynamic value container |
| `Radius` | f32 | Shape radius |
| `Height` | f32 | Shape height |
| `Flags` | u8 | Shape flags |

### Translation to JSON Schema

```json
{
  "fix_id": "vfx_shape",
  "name": "VFX Shape Issues (Patch 14.1)",
  "detect": {
    "type": "custom",
    "handler": "detect_vfx_shape_issues"
  },
  "apply": {
    "type": "custom",
    "handler": "fix_vfx_shape"
  }
}
```

### Complexity Rating

- [ ] Simple
- [ ] Medium
- [x] Complex (requires custom Rust logic)

### Notes

This fix CANNOT be represented as pure JSON configuration. It requires custom Rust implementation because:
1. Multi-level structure traversal
2. Field extraction and re-creation
3. Conditional shape type selection
4. Moving fields between parent/child contexts

---

# Pattern Analysis

## Detection Types Found

| Type | Description | Used By |
|------|-------------|---------|
| `missing_field` | Field doesn't exist in BIN | healthbar |
| `wrong_value` | Field exists with incorrect value | healthbar |
| `field_exists` | Field exists (shouldn't) | staticmat 1&2 |
| `string_extension` | String ends with extension | black_icons, dds_to_tex |
| `not_in_wad` | Path hash not in WAD cache | black_icons, dds_to_tex |
| `entry_type_exists` | Entry has specific type hash | champion_bin |
| `binary_version` | Custom binary header parsing | bnk_remover |
| `custom` | Complex multi-field analysis | vfx_shape |

## Action Types Found

| Type | Description | Used By |
|------|-------------|---------|
| `add_field` | Insert new field into entry | healthbar |
| `set_field` | Change existing field value | healthbar |
| `rename_field_hash` | Change field's hash (key) | staticmat 1&2 |
| `replace_extension` | String extension replacement | black_icons, dds_to_tex |
| `remove_file_from_wad` | Delete WAD chunk | champion_bin, bnk_remover |
| `custom` | Complex multi-step transformation | vfx_shape |

## BIN Data Types Used

| Rust Type | BIN Type | Example Field |
|-----------|----------|---------------|
| `u8` | `BINType.U8` | `UnitHealthBarStyle` |
| `f32` | `BINType.F32` | `Radius`, `Height` |
| `String` | `BINType.STRING` | `TexturePath`, `iconAvatar` |
| `Vec3` | `BINType.VEC3` | `ConstantValue` |
| `Embed` | `BINType.EMBED` | `HealthBarData`, `Shape` |
| `Pointer` | `BINType.POINTER` | Converted shapes |
| `List2<Embed>` | `BINType.LIST2` | `SamplerValues` |

---

# Proposed Unified Schema

```json
{
  "version": "1.0.0",
  "last_updated": "2026-01-11",
  "fixes": {
    "healthbar_fix": {
      "name": "Missing HP Bar",
      "description": "Adds UnitHealthBarStyle field to fix invisible health bars",
      "enabled": true,
      "severity": "high",
      "detect": {
        "type": "missing_or_wrong_field",
        "entry_type": "SkinCharacterDataProperties",
        "embed_path": "HealthBarData",
        "embed_type": "CharacterHealthBarDataRecord",
        "field": "UnitHealthBarStyle",
        "expected_value": 12
      },
      "apply": {
        "type": "ensure_field",
        "field": "UnitHealthBarStyle",
        "value": 12,
        "data_type": "u8",
        "create_parent": {
          "field": "HealthBarData",
          "type": "CharacterHealthBarDataRecord"
        }
      }
    },
    "staticmat_texturepath": {
      "name": "White Model (TextureName)",
      "description": "Renames TextureName to TexturePath in material definitions",
      "enabled": true,
      "severity": "critical",
      "detect": {
        "type": "field_hash_exists",
        "entry_type": "StaticMaterialDef",
        "path": "SamplerValues.*.TextureName"
      },
      "apply": {
        "type": "rename_hash",
        "from_hash": "TextureName",
        "to_hash": "TexturePath"
      }
    },
    "staticmat_samplername": {
      "name": "White Model (SamplerName)",
      "description": "Renames SamplerName to TextureName in material definitions",
      "enabled": true,
      "severity": "critical",
      "detect": {
        "type": "field_hash_exists",
        "entry_type": "StaticMaterialDef",
        "path": "SamplerValues.*.SamplerName"
      },
      "apply": {
        "type": "rename_hash",
        "from_hash": "SamplerName",
        "to_hash": "TextureName"
      }
    },
    "black_icons": {
      "name": "Black/Missing Icons",
      "description": "Converts .dds icon references to .tex when file not in WAD",
      "enabled": true,
      "severity": "medium",
      "detect": {
        "type": "string_extension_not_in_wad",
        "entry_type": "SkinCharacterDataProperties",
        "fields": ["iconAvatar", "iconCircle", "iconSquare"],
        "extension": ".dds"
      },
      "apply": {
        "type": "replace_string_extension",
        "from": ".dds",
        "to": ".tex"
      }
    },
    "dds_to_tex": {
      "name": "Broken Particle Effects",
      "description": "Converts all .dds texture references to .tex when file not in WAD",
      "enabled": true,
      "severity": "high",
      "detect": {
        "type": "recursive_string_extension_not_in_wad",
        "extension": ".dds",
        "path_prefixes": [
          "assets/characters/",
          "assets/shared/materials/",
          "assets/shared/particles/",
          "assets/particles/"
        ]
      },
      "apply": {
        "type": "replace_string_extension",
        "from": ".dds",
        "to": ".tex"
      }
    },
    "champion_bin_remover": {
      "name": "Champion Bin Files",
      "description": "Removes champion data files that break mods after patches",
      "enabled": true,
      "severity": "high",
      "detect": {
        "type": "entry_type_exists_any",
        "entry_types": [
          "SpellObject",
          "StatStoneData",
          "SkinCharacterMetaDataProperties",
          "CharacterRecord",
          "ChampionRuneRecommendationsContext",
          "RecSpellRankUpInfoList",
          "ItemRecommendationOverrideSet",
          "ItemRecommendationContextList",
          "StatStoneSet",
          "AbilityObject"
        ]
      },
      "apply": {
        "type": "remove_from_wad"
      }
    },
    "bnk_remover": {
      "name": "Broken Sound Files",
      "description": "Removes BNK audio files with incompatible versions",
      "enabled": true,
      "severity": "low",
      "detect": {
        "type": "bnk_version_not_in",
        "allowed_versions": [134, 145]
      },
      "apply": {
        "type": "remove_from_wad"
      }
    },
    "vfx_shape": {
      "name": "VFX Shape Issues",
      "description": "Fixes particle shape structures broken by patch 14.1",
      "enabled": true,
      "severity": "critical",
      "detect": {
        "type": "custom",
        "handler": "detect_vfx_shape"
      },
      "apply": {
        "type": "custom",
        "handler": "fix_vfx_shape"
      }
    }
  }
}
```

---

# Complex Fixes Requiring Custom Rust Logic

## Fix: VFX Shape

**Why it's complex:**
- Requires traversing 5+ levels of nested structures
- Must extract values from old format and restructure into new format
- Shape type selection depends on analyzed field combinations
- Fields must be moved between parent/child contexts

**Proposed Rust Interface:**

```rust
/// Custom handler for VFX shape detection
pub fn detect_vfx_shape(bin: &BinFile) -> Vec<VfxShapeIssue> {
    // 1. Find VfxSystemDefinitionData entries
    // 2. Traverse ComplexEmitterDefinitionData/SimpleEmitterDefinitionData
    // 3. Check Shape fields for old-format markers:
    //    - BirthTranslation inside Shape
    //    - EmitOffset/EmitRotationAngles/EmitRotationAxes presence
    // 4. Return list of issues with locations
}

/// Custom handler for VFX shape fix
pub fn fix_vfx_shape(bin: &mut BinFile) -> Result<u32, Error> {
    // 1. For each detected issue:
    //    - Move BirthTranslation to emitter level (hash: 0x563d4a22)
    //    - Extract Radius/Height from EmitOffset.ConstantValue
    //    - Determine target shape type (0x3dbe415d, 0xee39916f, 0x4f4e2ed7)
    //    - Rebuild Shape with new hash (0x3bf0b4ed) and correct type
    // 2. Return count of fixes applied
}
```

**JSON Config:**

```json
{
  "detect": { "type": "custom", "handler": "detect_vfx_shape" },
  "apply": { "type": "custom", "handler": "fix_vfx_shape" }
}
```

---

# Dependencies Between Fixes

| Fix | Depends On | Reason |
|-----|------------|--------|
| `black_icons` | WAD cache | Must check if .dds exists |
| `dds_to_tex` | WAD cache | Must check if .dds exists |
| `champion_bin_remover` | WAD structure | Removes WAD chunks |
| `bnk_remover` | WAD structure | Removes WAD chunks |

**Order Recommendation:**
1. Build WAD cache first (single pass)
2. Apply BIN-modifying fixes (healthbar, staticmat, icons, dds_to_tex, vfx_shape)
3. Apply WAD-removing fixes (champion_bin, bnk_remover)
4. Rebuild WAD/Fantome

---

**Last Updated:** 2026-01-11
