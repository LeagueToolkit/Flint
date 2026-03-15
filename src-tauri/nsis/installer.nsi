; Flint Custom Dark-Themed NSIS Installer
; Based on Tauri v2 template with dark theme customization
; Uses Tauri's handlebars variables for proper path injection

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

; ============================================================================
; Tauri-Provided Variables (injected via handlebars at build time)
; These are used throughout the installer script
; ============================================================================

; ============================================================================
; Dark Theme Configuration
; ============================================================================

; Modern UI Configuration - Dark Theme Colors
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_UNFINISHPAGE_NOAUTOCLOSE

; Branding
BrandingText "{{product_name}} {{version}}"

; ============================================================================
; Installer Graphics (Using Tauri Variables)
; ============================================================================

{{#if installer_icon}}
!define MUI_ICON "{{installer_icon}}"
!define MUI_UNICON "{{installer_icon}}"
{{/if}}

{{#if header_image}}
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "{{header_image}}"
!define MUI_HEADERIMAGE_UNBITMAP "{{header_image}}"
!define MUI_HEADERIMAGE_RIGHT
{{/if}}

{{#if sidebar_image}}
!define MUI_WELCOMEFINISHPAGE_BITMAP "{{sidebar_image}}"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "{{sidebar_image}}"
{{/if}}

; ============================================================================
; Installer Configuration
; ============================================================================

Name "{{product_name}}"
OutFile "{{out_file}}"

; Current user installation (no admin required)
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\{{product_name}}"
!define INSTALLMODE "currentUser"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{product_name}}"

ShowInstDetails show
ShowUnInstDetails show

; ============================================================================
; Modern UI Pages
; ============================================================================

; License page (only if license file exists)
{{#if license}}
!insertmacro MUI_PAGE_LICENSE "{{license}}"
{{/if}}

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

!define MUI_FINISHPAGE_RUN "$INSTDIR\{{main_binary_name}}.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch {{product_name}}"
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Languages
!insertmacro MUI_LANGUAGE "English"

; ============================================================================
; Version Info
; ============================================================================
VIProductVersion "{{version}}.0"
VIAddVersionKey "ProductName" "{{product_name}}"
{{#if short_description}}
VIAddVersionKey "FileDescription" "{{short_description}}"
{{/if}}
VIAddVersionKey "FileVersion" "{{version}}"
VIAddVersionKey "ProductVersion" "{{version}}"
{{#if copyright}}
VIAddVersionKey "LegalCopyright" "{{copyright}}"
{{/if}}
{{#if manufacturer}}
VIAddVersionKey "CompanyName" "{{manufacturer}}"
{{/if}}

; ============================================================================
; Installer Sections
; ============================================================================

Section "Install" InstallSection
  SetOutPath "$INSTDIR"

  ; Copy main executable and all resources
  {{#each resources}}
  File "{{this}}"
  {{/each}}

  ; Create shortcuts
  CreateDirectory "$SMPROGRAMS\{{product_name}}"
  CreateShortcut "$SMPROGRAMS\{{product_name}}\{{product_name}}.lnk" "$INSTDIR\{{main_binary_name}}.exe"
  CreateShortcut "$DESKTOP\{{product_name}}.lnk" "$INSTDIR\{{main_binary_name}}.exe"

  ; Write uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Registry entries for Windows Add/Remove Programs (current user)
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "{{product_name}}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" "$INSTDIR\{{main_binary_name}}.exe"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "{{version}}"
  {{#if manufacturer}}
  WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "{{manufacturer}}"
  {{/if}}
  WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoRepair" 1
SectionEnd

; ============================================================================
; Uninstaller Section
; ============================================================================

Section "Uninstall"
  ; Remove files and directories
  Delete "$INSTDIR\{{main_binary_name}}.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir /r "$INSTDIR"

  ; Remove shortcuts
  Delete "$DESKTOP\{{product_name}}.lnk"
  Delete "$SMPROGRAMS\{{product_name}}\{{product_name}}.lnk"
  RMDir "$SMPROGRAMS\{{product_name}}"

  ; Remove registry entries (current user)
  DeleteRegKey HKCU "${UNINSTKEY}"
SectionEnd

; ============================================================================
; Installer Functions
; ============================================================================

Function .onInit
  ; Check for existing installation (current user)
  ReadRegStr $0 HKCU "${UNINSTKEY}" "InstallLocation"

  ${If} $0 != ""
    MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION \
      "{{product_name}} is already installed at:$\n$0$\n$\nClick OK to upgrade or Cancel to abort." \
      IDOK upgrade
    Abort

    upgrade:
    ; Uninstall previous version
    ExecWait '"$0\uninstall.exe" /S _?=$0'
    Delete "$0\uninstall.exe"
  ${EndIf}
FunctionEnd
