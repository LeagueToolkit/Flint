; Flint Custom Dark-Themed NSIS Installer
; Based on Tauri v2 template with dark theme customization
; Uses Tauri's handlebars variables for proper path injection

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

; ============================================================================
; Tauri-Provided Variables (injected via handlebars)
; ============================================================================
!define PRODUCTNAME "{{product_name}}"
!define VERSION "{{version}}"
!define MANUFACTURER "{{manufacturer}}"
!define INSTALLMODE "{{install_mode}}"
!define LICENSE "{{license}}"
!define INSTALLERICON "{{installer_icon}}"
!define SIDEBARIMAGE "{{sidebar_image}}"
!define HEADERIMAGE "{{header_image}}"
!define MAINBINARYNAME "{{main_binary_name}}"
!define MAINBINARYSRCPATH "{{main_binary_path}}"
!define OUTFILE "{{out_file}}"
!define ARCH "{{arch}}"
!define BUNDLEID "{{bundle_id}}"
!define COPYRIGHT "{{copyright}}"
!define SHORTDESCRIPTION "{{short_description}}"
!define INSTALLWEBVIEW2MODE "{{install_webview2_mode}}"
!define WEBVIEW2INSTALLERARGS "{{webview2_installer_args}}"
!define WEBVIEW2BOOTSTRAPPERPATH "{{webview2_bootstrapper_path}}"
!define WEBVIEW2INSTALLERPATH "{{webview2_installer_path}}"
!define ALLOWDOWNGRADES "{{allow_downgrades}}"
!define DISPLAYLANGUAGESELECTOR "{{display_language_selector}}"

; ============================================================================
; Dark Theme Configuration
; ============================================================================

; Modern UI Configuration - Dark Theme Colors
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_UNFINISHPAGE_NOAUTOCLOSE

; Branding
BrandingText "${PRODUCTNAME} ${VERSION}"

; ============================================================================
; Installer Graphics (Using Tauri Variables)
; ============================================================================

!ifdef INSTALLERICON
  !define MUI_ICON "${INSTALLERICON}"
  !define MUI_UNICON "${INSTALLERICON}"
!endif

!ifdef HEADERIMAGE
  !define MUI_HEADERIMAGE
  !define MUI_HEADERIMAGE_BITMAP "${HEADERIMAGE}"
  !define MUI_HEADERIMAGE_UNBITMAP "${HEADERIMAGE}"
  !define MUI_HEADERIMAGE_RIGHT
!endif

!ifdef SIDEBARIMAGE
  !define MUI_WELCOMEFINISHPAGE_BITMAP "${SIDEBARIMAGE}"
  !define MUI_UNWELCOMEFINISHPAGE_BITMAP "${SIDEBARIMAGE}"
!endif

; ============================================================================
; Installer Configuration
; ============================================================================

Name "${PRODUCTNAME}"
OutFile "${OUTFILE}"

!if "${INSTALLMODE}" == "perMachine"
  RequestExecutionLevel highest
  InstallDir "$PROGRAMFILES64\${PRODUCTNAME}"
  !define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!else if "${INSTALLMODE}" == "currentUser"
  RequestExecutionLevel user
  InstallDir "$LOCALAPPDATA\${PRODUCTNAME}"
  !define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!endif

InstallDirRegKey HKCU "${UNINSTKEY}" "InstallLocation"
ShowInstDetails show
ShowUnInstDetails show

; ============================================================================
; Modern UI Pages
; ============================================================================

!ifdef LICENSE
  !insertmacro MUI_PAGE_LICENSE "${LICENSE}"
!endif

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

!define MUI_FINISHPAGE_RUN "$INSTDIR\${MAINBINARYNAME}.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${PRODUCTNAME}"
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Languages
!insertmacro MUI_LANGUAGE "English"

; ============================================================================
; Version Info
; ============================================================================
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${PRODUCTNAME}"
VIAddVersionKey "FileDescription" "${SHORTDESCRIPTION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "${COPYRIGHT}"
VIAddVersionKey "CompanyName" "${MANUFACTURER}"

; ============================================================================
; Installer Sections
; ============================================================================

Section "Install" InstallSection
  SetOutPath "$INSTDIR"

  ; Copy main executable
  File "${MAINBINARYSRCPATH}"

  ; Copy all bundled resources
  File /r "${RESOURCESPATH}\*"

  ; Create shortcuts
  CreateDirectory "$SMPROGRAMS\${PRODUCTNAME}"
  CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"

  ; Write uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Registry entries for Windows Add/Remove Programs
  !if "${INSTALLMODE}" == "perMachine"
    WriteRegStr HKLM "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
    WriteRegStr HKLM "${UNINSTKEY}" "DisplayIcon" "$INSTDIR\${MAINBINARYNAME}.exe"
    WriteRegStr HKLM "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
    WriteRegStr HKLM "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
    WriteRegStr HKLM "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKLM "${UNINSTKEY}" "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegDWORD HKLM "${UNINSTKEY}" "NoModify" 1
    WriteRegDWORD HKLM "${UNINSTKEY}" "NoRepair" 1
  !else
    WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
    WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" "$INSTDIR\${MAINBINARYNAME}.exe"
    WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
    WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegDWORD HKCU "${UNINSTKEY}" "NoModify" 1
    WriteRegDWORD HKCU "${UNINSTKEY}" "NoRepair" 1
  !endif
SectionEnd

; ============================================================================
; Uninstaller Section
; ============================================================================

Section "Uninstall"
  ; Remove files and directories
  Delete "$INSTDIR\${MAINBINARYNAME}.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir /r "$INSTDIR"

  ; Remove shortcuts
  Delete "$DESKTOP\${PRODUCTNAME}.lnk"
  Delete "$SMPROGRAMS\${PRODUCTNAME}\${PRODUCTNAME}.lnk"
  RMDir "$SMPROGRAMS\${PRODUCTNAME}"

  ; Remove registry entries
  !if "${INSTALLMODE}" == "perMachine"
    DeleteRegKey HKLM "${UNINSTKEY}"
  !else
    DeleteRegKey HKCU "${UNINSTKEY}"
  !endif
SectionEnd

; ============================================================================
; Installer Functions
; ============================================================================

Function .onInit
  ; Check for existing installation
  !if "${INSTALLMODE}" == "perMachine"
    ReadRegStr $0 HKLM "${UNINSTKEY}" "InstallLocation"
  !else
    ReadRegStr $0 HKCU "${UNINSTKEY}" "InstallLocation"
  !endif

  ${If} $0 != ""
    MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION \
      "${PRODUCTNAME} is already installed at:$\n$0$\n$\nClick OK to upgrade or Cancel to abort." \
      IDOK upgrade
    Abort

    upgrade:
    ; Uninstall previous version
    ExecWait '"$0\uninstall.exe" /S _?=$0'
    Delete "$0\uninstall.exe"
  ${EndIf}
FunctionEnd
