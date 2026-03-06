# State Management

Flint uses **Zustand 4** for state management, organized into focused domain slices for better performance and maintainability.

## Store Architecture

All stores are combined into a single `useAppState()` hook for backward compatibility, but components can also import individual stores for selective re-renders.

### Store Slices

#### appMetadataStore
**Purpose**: Application-wide status and metadata

**State**:
- `hashLoaded` (boolean) - Whether CommunityDragon hashtables are loaded
- `hashInfo` (object) - Hashtable statistics and source information
- `logs` (array) - Frontend and backend log messages
- `verboseLogging` (boolean) - Debug logging toggle

**Actions**:
- `setHashLoaded(loaded)` - Update hash loading status
- `setHashInfo(info)` - Update hashtable metadata
- `addLog(log)` - Append log message
- `clearLogs()` - Clear all logs
- `setVerboseLogging(enabled)` - Toggle debug logs

**Usage**:
```typescript
const { hashLoaded, hashInfo } = useAppMetadataStore();
```

---

#### configStore
**Purpose**: User configuration and settings (persisted to localStorage)

**State**:
- `leaguePath` (string) - League of Legends installation path
- `creatorName` (string) - Mod creator name (for exports)
- `projectName` (string) - Active project name
- `isFirstRun` (boolean) - First-time setup flag
- `autoUpdateEnabled` (boolean) - Auto-update toggle
- `skippedUpdateVersion` (string|null) - Last skipped update version

**Actions**:
- `setLeaguePath(path)` - Update League installation path
- `setCreatorName(name)` - Update creator name
- `setProjectName(name)` - Update project name
- `setFirstRun(isFirst)` - Set first-run flag
- `setAutoUpdateEnabled(enabled)` - Toggle auto-updates
- `setSkippedUpdateVersion(version)` - Mark update as skipped
- `loadFromLocalStorage()` - Load persisted config on startup
- `saveToLocalStorage()` - Persist config to localStorage

**Persistence**: Automatically saved to `localStorage` on changes

**Usage**:
```typescript
const { leaguePath, setLeaguePath } = useConfigStore();
```

---

#### projectTabStore
**Purpose**: Multi-tab project workspace management

**State**:
- `tabs` (array) - Open project tabs with metadata
  ```typescript
  interface ProjectTab {
    id: string;
    projectPath: string;
    projectName: string;
    fileTree: TreeNode[];
    selectedNode: TreeNode | null;
    expandedFolders: Set<string>;
    selectedCheckpoint: string | null;
    refatherEnabled: boolean;
    concatBinsEnabled: boolean;
  }
  ```
- `activeTabId` (string|null) - Currently active project tab

**Actions**:
- `addTab(tab)` - Open new project tab
- `removeTab(id)` - Close project tab
- `setActiveTab(id)` - Switch to project tab
- `updateTab(id, updates)` - Partial tab update
- `setFileTree(id, tree)` - Update project file tree
- `setSelectedNode(id, node)` - Update selected file
- `toggleFolder(id, path)` - Expand/collapse folder
- `setRefatherEnabled(id, enabled)` - Toggle refathering
- `setConcatBinsEnabled(id, enabled)` - Toggle BIN concatenation

**Usage**:
```typescript
const { tabs, activeTabId, setActiveTab } = useProjectTabStore();
const activeTab = tabs.find(t => t.id === activeTabId);
```

---

#### navigationStore
**Purpose**: View routing and navigation state

**State**:
- `currentView` (string) - Active view (`'preview'|'extract'|'wad-explorer'|'welcome'`)
- `navigationHistory` (array) - View history stack
- `canGoBack` (boolean) - Whether back navigation is possible

**Actions**:
- `setCurrentView(view)` - Navigate to view
- `goBack()` - Navigate to previous view
- `resetNavigation()` - Clear history and return to welcome

**Usage**:
```typescript
const { currentView, setCurrentView } = useNavigationStore();
```

---

#### wadExtractStore
**Purpose**: Individual WAD file browsing sessions

**State**:
- `extractSessions` (array) - Open WAD browser sessions
  ```typescript
  interface ExtractSession {
    id: string;
    wadPath: string;
    wadName: string;
    chunks: WadChunk[];
    selectedChunk: WadChunk | null;
    expandedFolders: Set<string>;
    searchQuery: string;
  }
  ```
- `activeExtractId` (string|null) - Currently active WAD session

**Actions**:
- `openExtractSession(session)` - Open WAD file in new tab
- `closeExtractSession(id)` - Close WAD session
- `setActiveExtract(id)` - Switch to WAD session
- `updateExtractSession(id, updates)` - Partial session update
- `setSelectedChunk(id, chunk)` - Select file for preview
- `setSearchQuery(id, query)` - Filter WAD files

**Usage**:
```typescript
const { extractSessions, activeExtractId } = useWadExtractStore();
const activeSession = extractSessions.find(s => s.id === activeExtractId);
```

---

#### wadExplorerStore
**Purpose**: Unified game WAD virtual filesystem browser

**State**:
- `isOpen` (boolean) - Whether WAD Explorer is active
- `wads` (array) - All game WAD files
  ```typescript
  interface WadExplorerWad {
    path: string;
    name: string;
    category: string;
    status: 'idle' | 'loading' | 'loaded' | 'error';
    chunks: WadChunk[];
  }
  ```
- `scanStatus` (string) - Scan progress (`'idle'|'scanning'|'complete'|'error'`)
- `selectedChunk` (object|null) - Selected file for preview
- `expandedWads` (Set) - Expanded WAD files in tree
- `expandedFolders` (Set) - Expanded folders in tree
- `searchQuery` (string) - Filter across all WADs

**Actions**:
- `openWadExplorer()` - Open WAD Explorer view
- `closeWadExplorer()` - Close WAD Explorer
- `setScanStatus(status)` - Update scan progress
- `setWads(wads)` - Set WAD list after scan
- `setWadStatus(path, status)` - Update individual WAD status
- `setWadChunks(path, chunks)` - Load chunks for WAD
- `setSelectedChunk(chunk)` - Select file for preview
- `toggleWad(path)` - Expand/collapse WAD in tree
- `toggleFolder(path)` - Expand/collapse folder
- `setSearchQuery(query)` - Filter files across WADs

**Usage**:
```typescript
const { isOpen, wads, selectedChunk } = useWadExplorerStore();
```

---

#### championStore
**Purpose**: Champion data caching and skin metadata

**State**:
- `champions` (Map) - Champion data from Data Dragon API
- `loadingChampions` (boolean) - API fetch status

**Actions**:
- `loadChampionData(championKey)` - Fetch champion from API
- `loadMultipleChampions(keys)` - Batch fetch champions
- `clearCache()` - Clear champion cache

**Usage**:
```typescript
const { champions, loadChampionData } = useChampionStore();
```

---

#### modalStore
**Purpose**: Modal dialogs and context menus

**State**:
- `activeModal` (string|null) - Currently open modal
- `modalProps` (object) - Props passed to active modal
- `contextMenu` (object|null) - Context menu state
  ```typescript
  interface ContextMenuState {
    x: number;
    y: number;
    items: ContextMenuItem[];
  }
  ```

**Actions**:
- `openModal(modal, props)` - Show modal dialog
- `closeModal()` - Hide active modal
- `openContextMenu(x, y, items)` - Show context menu
- `closeContextMenu()` - Hide context menu

**Usage**:
```typescript
const { activeModal, openModal, closeModal } = useModalStore();
```

---

#### notificationStore
**Purpose**: Toast notifications and status messages

**State**:
- `notifications` (array) - Active toast notifications
  ```typescript
  interface Notification {
    id: string;
    type: 'info' | 'success' | 'warning' | 'error';
    message: string;
    duration?: number;
  }
  ```

**Actions**:
- `showNotification(notification)` - Display toast
- `removeNotification(id)` - Dismiss toast
- `clearAll()` - Clear all notifications

**Usage**:
```typescript
const { showNotification } = useNotificationStore();
showNotification({
  type: 'success',
  message: 'Project exported successfully!',
  duration: 3000
});
```

---

## Navigation Coordination

The `navigationCoordinator.ts` module manages complex navigation flows between different tab types.

### Tab Switching Logic

**Single Source of Truth**: `currentView` determines what's displayed:
- `'preview'` - Show active project tab
- `'extract'` - Show active WAD session
- `'wad-explorer'` - Show WAD Explorer
- `'welcome'` - Show welcome screen

**Active Tab Pointers**: `activeTabId` and `activeExtractId` are "last active" pointers that are NOT nulled when switching to a different tab type. This allows seamless switching back to the last active tab of each type.

### Tab Close Fallback Chains

**Project Tab Close** (`REMOVE_TAB`):
1. Switch to another project tab (if any)
2. Switch to active extract session (if any)
3. Switch to WAD Explorer (if open)
4. Show welcome screen

**Extract Session Close** (`CLOSE_EXTRACT_SESSION`):
1. Switch to another extract session (if any)
2. Switch to active project tab (if any)
3. Switch to WAD Explorer (if open)
4. Show welcome screen

**WAD Explorer Close** (`CLOSE_WAD_EXPLORER`):
1. Switch to active project tab (if any)
2. Switch to active extract session (if any)
3. Show welcome screen

### Keyboard Shortcuts

- `Ctrl+W` - Close current tab based on `currentView`
- `Ctrl+Tab` / `Ctrl+Shift+Tab` - Cycle through all tabs
- `Ctrl+1...9` - Switch to tab by index

## Best Practices

### Performance
1. **Use selective subscriptions**: Import individual stores instead of `useAppState()` when possible
   ```typescript
   // ❌ Bad - re-renders on any state change
   const state = useAppState();

   // ✅ Good - only re-renders when tabs change
   const { tabs } = useProjectTabStore();
   ```

2. **Memoize derived state**: Use `useMemo` for computed values
   ```typescript
   const activeTab = useMemo(
     () => tabs.find(t => t.id === activeTabId),
     [tabs, activeTabId]
   );
   ```

3. **Batch updates**: Combine multiple state changes
   ```typescript
   // ❌ Bad - triggers two re-renders
   setActiveTab(id);
   setCurrentView('preview');

   // ✅ Good - single update
   updateTab(id, { active: true, view: 'preview' });
   ```

### State Persistence
- **configStore** automatically persists to localStorage
- Other stores are session-only (cleared on app restart)
- Backend maintains its own persistent state in project files

### Testing
- Stores are plain JavaScript objects - easy to test without React
- Use `create()` from Zustand to create isolated store instances for tests
- Mock Tauri commands when testing components that call backend
