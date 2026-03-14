# League HUD System - Full Implementation Plan

> **Vision**: Complete replication of League of Legends HUD system with full modification capabilities

## 📋 Current Status (Phase 1 - COMPLETE ✅)

### Implemented Features
- ✅ Visual drag-and-drop editor for HUD elements
- ✅ Undo/redo system (200 steps)
- ✅ Multi-select and batch operations
- ✅ Layer and group visibility controls
- ✅ Search functionality
- ✅ Save/load `.ritobin` files
- ✅ 1600x1200 League resolution canvas

### Current Limitations
- Only supports `uibase.bin` (loading screen HUD)
- No in-game HUD parsing
- No keybinding system
- No live preview simulation
- No element interaction logic
- Manual file navigation required

---

## 🎯 Phase 2: Full HUD File Discovery & Parsing

### Goals
Parse ALL League HUD files, not just loading screen

### HUD File Locations (League Installation)
```
DATA/FINAL/clientstates/
├── loadingscreen/
│   └── ux/loadingscreenclassic/uibase/uibase.bin  ✅ Current support
├── game/
│   ├── hud/
│   │   ├── hudbase.bin                            📍 Main in-game HUD
│   │   ├── scoreboard.bin                         📍 Tab scoreboard
│   │   ├── shop.bin                               📍 Shop UI (P key)
│   │   ├── champion_selection.bin                 📍 Champion abilities display
│   │   ├── minimap.bin                            📍 Minimap elements
│   │   ├── items.bin                              📍 Item slots (1-7 keys)
│   │   └── abilities.bin                          📍 QWER ability slots
│   └── notifications/
│       └── notifications.bin                      📍 Announcements (kills, etc.)
└── shared/
    └── hud_shared.bin                             📍 Shared HUD resources
```

### Implementation Tasks

#### 2.1 HUD File Scanner
**File**: `src-tauri/src/core/hud/scanner.rs`
```rust
pub struct HudFileScanner {
    league_path: PathBuf,
    discovered_files: HashMap<HudFileType, PathBuf>,
}

pub enum HudFileType {
    LoadingScreen,    // uibase.bin
    MainHud,          // hudbase.bin
    Scoreboard,       // scoreboard.bin
    Shop,             // shop.bin
    ChampionAbilities,
    Minimap,
    Items,
    Notifications,
    Shared,
}

impl HudFileScanner {
    pub fn scan_league_installation(&mut self) -> Result<()>;
    pub fn get_file_path(&self, file_type: HudFileType) -> Option<&Path>;
    pub fn export_all_to_project(&self, project_path: &Path) -> Result<()>;
}
```

**Features**:
- Auto-detect League installation via settings
- Scan `DATA/FINAL/clientstates/` for all HUD BIN files
- Cache discovered paths
- Export all HUD files to project structure
- Validate file integrity (magic bytes, size checks)

#### 2.2 Multi-File HUD Parser
**File**: `src-tauri/src/core/hud/multi_parser.rs`
```rust
pub struct HudSystem {
    files: HashMap<HudFileType, HudData>,
    cross_references: Vec<HudReference>,
}

pub struct HudReference {
    from_file: HudFileType,
    to_file: HudFileType,
    element_id: String,
    reference_type: ReferenceType,
}

pub enum ReferenceType {
    Include,        // File includes another
    Override,       // Element overrides another
    Link,           // Element links to another
    Dependency,     // Requires another element
}

impl HudSystem {
    pub fn load_all_files(&mut self, paths: &HashMap<HudFileType, PathBuf>) -> Result<()>;
    pub fn resolve_references(&mut self) -> Result<()>;
    pub fn get_element_dependencies(&self, element_id: &str) -> Vec<String>;
    pub fn validate_consistency(&self) -> Vec<ValidationError>;
}
```

**Features**:
- Load multiple HUD files simultaneously
- Resolve cross-file element references
- Track element dependencies
- Validate element links and overrides
- Merge shared resources into specific HUDs

#### 2.3 Frontend Multi-File Manager
**File**: `src/components/hud/HUDSystemManager.tsx`
```tsx
interface HudSystemState {
    files: Record<HudFileType, HudFileData | null>;
    activeFile: HudFileType;
    crossReferences: HudReference[];
    validationErrors: ValidationError[];
}

export const HUDSystemManager: React.FC = () => {
    // Displays all discovered HUD files
    // Switch between files (tabs or dropdown)
    // Show cross-references visually
    // Highlight validation errors
    // Bulk import/export
}
```

---

## 🎮 Phase 3: Keybinding System

### Goals
Simulate League keybindings to trigger HUD element visibility and interactions

### Default League Keybindings
| Key | Action | HUD Elements Affected |
|-----|--------|----------------------|
| `P` | Toggle Shop | `shop.bin` elements show/hide |
| `Tab` | Toggle Scoreboard | `scoreboard.bin` elements show/hide |
| `O` | Toggle Advanced Stats | Stats overlay elements |
| `Z` | Toggle Target Champions Only | Targeting indicator |
| `C` | Open Champion Stats | Champion panel elements |
| `ESC` | Close All Menus | Hide shop, scoreboard, settings |
| `1-7` | Use Items | Highlight item slots |
| `Q/W/E/R` | Cast Abilities | Highlight ability icons, show cooldowns |
| `D/F` | Summoner Spells | Highlight summoner spell slots |

### Implementation Tasks

#### 3.1 Keybinding Engine
**File**: `src/components/hud/KeybindingEngine.tsx`
```tsx
interface Keybinding {
    key: string;
    modifiers: ('ctrl' | 'shift' | 'alt')[];
    action: KeybindAction;
    targetElements: string[];  // Element IDs affected
}

enum KeybindAction {
    ToggleVisibility,
    ShowTemporary,
    Highlight,
    TriggerAnimation,
    UpdateState,
}

class KeybindingEngine {
    private bindings: Map<string, Keybinding>;
    private activeKeys: Set<string>;

    registerBinding(binding: Keybinding): void;
    handleKeyDown(event: KeyboardEvent): void;
    handleKeyUp(event: KeyboardEvent): void;
    executeAction(action: KeybindAction, elements: string[]): void;
}
```

**Features**:
- Capture keyboard input in HUD editor
- Map keys to HUD element actions
- Support modifier keys (Ctrl, Shift, Alt)
- Temporary vs. toggle visibility (Tab vs. P)
- Visual feedback for active keybinds

#### 3.2 Element State System
**File**: `src/lib/stores/hudStateStore.ts`
```tsx
interface ElementState {
    visible: boolean;
    highlighted: boolean;
    animating: boolean;
    opacity: number;
    customState: Record<string, any>;
}

interface HudState {
    elements: Record<string, ElementState>;
    activeKeybinds: Set<string>;
    shopOpen: boolean;
    scoreboardOpen: boolean;
    gamePhase: 'pregame' | 'ingame' | 'postgame';
}

export const useHudState = () => {
    const [state, setState] = useState<HudState>(/* initial */);

    const toggleElement = (elementId: string) => { /* ... */ };
    const setElementVisibility = (elementId: string, visible: boolean) => { /* ... */ };
    const highlightElement = (elementId: string, duration?: number) => { /* ... */ };

    return { state, toggleElement, setElementVisibility, highlightElement };
};
```

#### 3.3 Live Preview Mode
**File**: `src/components/hud/LivePreview.tsx`
```tsx
export const LivePreview: React.FC<{ hudData: HudData }> = ({ hudData }) => {
    const { state, toggleElement } = useHudState();
    const keybindEngine = useKeybindingEngine();

    return (
        <div className="hud-live-preview" onKeyDown={keybindEngine.handleKeyDown}>
            {/* Render HUD with live keybinding support */}
            {/* Show/hide elements based on keybind state */}
            {/* Display keybind hints (P for shop, Tab for scoreboard) */}
        </div>
    );
};
```

**Features**:
- Real-time keybinding simulation
- Visual keybind hints overlay
- Element state animations
- Multiple HUD file coordination (shop + main HUD)

---

## 🎨 Phase 4: Visual Enhancements

### 4.1 Element Type Icons
Display actual League icons for each element type:
- Ability icons (Q/W/E/R with champion-specific images)
- Item icons (from item atlas)
- Summoner spell icons (Flash, Ignite, etc.)
- UI element icons (gold, CS, KDA)

### 4.2 Texture Atlas Integration
**File**: `src/components/hud/AtlasPreview.tsx`
- Load HUD texture atlas from League files
- Display sprite previews for each element
- Support sprite sheet slicing (UV coordinates)
- Drag-and-drop texture replacement

### 4.3 Animation Preview
- Show ability cooldown radial animations
- Item slot glow effects
- Level-up button shine
- Kill/death notification fly-ins

---

## 🔧 Phase 5: Advanced Editing Features

### 5.1 Element Templates
Pre-configured templates for common modifications:
- "Centered HUD" - Move elements to screen center
- "Minimal HUD" - Hide non-essential elements
- "Pro View" - Replicate professional player layouts
- "Colorblind Mode" - Adjust element colors

### 5.2 Bulk Operations
- Scale all elements by percentage
- Move element groups together (lock positions)
- Copy/paste element properties
- Mirror positions (left ↔ right)

### 5.3 Validation & Testing
- Check element overlap warnings
- Test at different resolutions (1920x1080, 2560x1440, etc.)
- Validate required elements exist
- Export compatibility check (League version)

---

## 📦 Phase 6: Project Integration

### 6.1 HUD Project Type
**File**: `src-tauri/src/commands/hud_project.rs`
```rust
pub async fn create_hud_project(
    name: String,
    hud_files: Vec<HudFileType>,
    league_path: String,
) -> Result<String> {
    // Create project structure
    // Import selected HUD files from League
    // Convert all to .ritobin
    // Create project metadata
    // Initialize default keybindings
}
```

**Project Structure**:
```
MyHUDMod/
├── project.json          # Project metadata
├── keybindings.json      # Custom keybinding config
├── content/
│   ├── DATA/
│   │   └── FINAL/
│   │       └── clientstates/
│   │           ├── game/
│   │           │   └── hud/
│   │           │       ├── hudbase.ritobin
│   │           │       ├── scoreboard.ritobin
│   │           │       ├── shop.ritobin
│   │           │       └── ...
│   │           └── loadingscreen/
│   │               └── ux/.../uibase.ritobin
└── preview/              # Screenshots/previews
```

### 6.2 Export Options
- Export as `.fantome` mod (League mod manager)
- Export as WAD files (direct installation)
- Export individual HUD files
- Export with/without keybinding configs

---

## 🚀 Implementation Roadmap

### Sprint 1: File Discovery (1-2 weeks)
- [ ] Implement `HudFileScanner`
- [ ] Add League HUD file path detection
- [ ] Create bulk import UI
- [ ] Test with all League HUD files

### Sprint 2: Multi-File Support (1-2 weeks)
- [ ] Implement `HudSystem` multi-file parser
- [ ] Add cross-reference resolution
- [ ] Create `HUDSystemManager` UI
- [ ] Add file switching (tabs)

### Sprint 3: Keybinding System (2 weeks)
- [ ] Implement `KeybindingEngine`
- [ ] Create `hudStateStore` with Zustand
- [ ] Add `LivePreview` component
- [ ] Test keybinding simulation

### Sprint 4: Visual Polish (1 week)
- [ ] Add texture atlas preview
- [ ] Implement element icons
- [ ] Add animation preview
- [ ] Create element templates

### Sprint 5: Project Integration (1 week)
- [ ] Create HUD project type
- [ ] Add bulk export functionality
- [ ] Implement validation system
- [ ] Add export to Fantome/WAD

---

## 🎯 Success Criteria

- [ ] Support all major League HUD files (8+ files)
- [ ] Keybinding system works for P (shop), Tab (scoreboard), etc.
- [ ] Live preview shows/hides elements correctly
- [ ] Cross-file element references resolved
- [ ] Export produces working League mods
- [ ] No element overlap/validation errors
- [ ] Load time < 2 seconds for full HUD system

---

## 📚 Technical Notes

### HUD File Format (ritobin)
```json
{
  "type": "PROP",
  "version": 1,
  "linked": ["path/to/shared.bin"],
  "#<hash>": {
    "name": "AbilityDisplayQ",
    "type": "UiElementIconData",
    "Layer": 5,
    "position": {
      "UIRect": {
        "position": { "x": 619, "y": 1054 },
        "Size": { "x": 66, "y": 66 },
        "SourceResolutionWidth": 1600,
        "SourceResolutionHeight": 1200
      },
      "Anchors": { "Anchor": { "x": 0.5, "y": 1.0 } }
    },
    "TextureData": {
      "mTextureName": "hud_abilities_atlas.dds",
      "mTextureUV": { "x1": 0.0, "y1": 0.0, "x2": 0.25, "y2": 0.25 }
    }
  }
}
```

### Keybinding Config Format
```json
{
  "version": "1.0.0",
  "bindings": [
    {
      "key": "P",
      "action": "TOGGLE_SHOP",
      "elements": ["ShopContainer", "ShopBackground", "ShopItemGrid"]
    },
    {
      "key": "Tab",
      "action": "TOGGLE_SCOREBOARD",
      "elements": ["ScoreboardContainer", "ScoreboardAlly", "ScoreboardEnemy"],
      "temporary": true
    }
  ]
}
```

### Element Dependency Graph
```
hudbase.bin
├── AbilityDisplayQ (depends on abilities.bin)
├── ItemSlot1 (depends on items.bin)
└── ScoreboardButton (links to scoreboard.bin)
```

---

## 🔮 Future Enhancements (Post-MVP)

- **AI-Powered Layout**: ML suggestions for optimal HUD layouts
- **Community Presets**: Share/download HUD configurations
- **Real-time Sync**: Test HUD changes in live League client (if possible)
- **3D Preview**: View HUD in 3D perspective (Summoner's Rift camera angles)
- **Accessibility**: Colorblind modes, high-contrast themes
- **Mobile Preview**: View HUD on mobile screen sizes (Wild Rift)

---

**Status**: Phase 1 Complete ✅ | Phase 2 Planning 📋
**Next Action**: Identify main `hudbase.bin` location and begin scanner implementation
**Estimated Total Time**: 6-8 weeks for full system
