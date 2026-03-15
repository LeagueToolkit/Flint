; Flint Custom Dark-Themed NSIS Installer
; Based on Tauri's default template with dark theme customization

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

!define PRODUCT_NAME "Flint"
!define PRODUCT_VERSION "${VERSION}"
!define PRODUCT_PUBLISHER "RitoShark"
!define PRODUCT_WEB_SITE "https://github.com/RitoShark/Flint"

; ============================================================================
; Dark Theme Configuration
; ============================================================================

; Custom colors (dark theme)
!define MUI_BGCOLOR "1a1a1a"              ; Dark background
!define MUI_TEXTCOLOR "e0e0e0"            ; Light text
!define MUI_ABORTWARNING

; Custom page colors
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_UNFINISHPAGE_NOAUTOCLOSE

; Installer branding
BrandingText "${PRODUCT_NAME} ${PRODUCT_VERSION}"

; ============================================================================
; Installer Graphics (Placeholders - replace these!)
; ============================================================================

; Sidebar image (left side): 164x314 pixels, BMP format
!define MUI_WELCOMEFINISHPAGE_BITMAP "..\installer-assets\installer-sidebar.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "..\installer-assets\installer-sidebar.bmp"

; Header image (top banner): 150x57 pixels, BMP format
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "..\installer-assets\installer-header.bmp"
!define MUI_HEADERIMAGE_UNBITMAP "..\installer-assets\installer-header.bmp"
!define MUI_HEADERIMAGE_RIGHT

; Icon
!define MUI_ICON "..\icons\icon.ico"
!define MUI_UNICON "..\icons\icon.ico"

; ============================================================================
; Installer Configuration
; ============================================================================

Name "${PRODUCT_NAME}"
OutFile "${OUT_FILE}"
InstallDir "$LOCALAPPDATA\${PRODUCT_NAME}"
InstallDirRegKey HKCU "Software\${PRODUCT_NAME}" ""
ShowInstDetails show
ShowUnInstDetails show

; Request application privileges
RequestExecutionLevel user

; ============================================================================
; Pages
; ============================================================================

; Installer pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${LICENSE_FILE}"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\${PRODUCT_NAME}.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${PRODUCT_NAME}"
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Language
!insertmacro MUI_LANGUAGE "English"

; ============================================================================
; Installer Section
; ============================================================================

Section "MainSection" SEC01
  SetOutPath "$INSTDIR"
  SetOverwrite on

  ; Copy all application files
  File /r "${INSTALL_DIR}\*.*"

  ; Create desktop shortcut
  CreateShortcut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_NAME}.exe"

  ; Create start menu shortcuts
  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortcut "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_NAME}.exe"
  CreateShortcut "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall.lnk" "$INSTDIR\uninstall.exe"
SectionEnd

; ============================================================================
; Post-Install Section
; ============================================================================

Section -Post
  ; Write uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Write registry keys for Add/Remove Programs
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "DisplayIcon" "$INSTDIR\${PRODUCT_NAME}.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"

  ; Store installation path
  WriteRegStr HKCU "Software\${PRODUCT_NAME}" "" "$INSTDIR"
SectionEnd

; ============================================================================
; Uninstaller Section
; ============================================================================

Section Uninstall
  ; Remove files
  Delete "$INSTDIR\uninstall.exe"
  RMDir /r "$INSTDIR"

  ; Remove shortcuts
  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"
  RMDir /r "$SMPROGRAMS\${PRODUCT_NAME}"

  ; Remove registry keys
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
  DeleteRegKey HKCU "Software\${PRODUCT_NAME}"
SectionEnd

; ============================================================================
; Custom Functions
; ============================================================================

Function .onInit
  ; Check if already installed
  ReadRegStr $R0 HKCU "Software\${PRODUCT_NAME}" ""
  StrCmp $R0 "" done

  MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION \
  "${PRODUCT_NAME} is already installed. $\n$\nClick 'OK' to remove the previous version or 'Cancel' to cancel this upgrade." \
  IDOK uninst
  Abort

uninst:
  ; Run the uninstaller
  ClearErrors
  ExecWait '$R0\uninstall.exe _?=$R0'

done:
FunctionEnd
