; ============================================================================
; Weave - MayaFlux Windows Installer
; ============================================================================
; Mirrors the structure of macOS postinstall for consistency
; - Downloads and installs MayaFlux
; - Installs dependencies via PowerShell package manager
; - Sets up environment variables
; - Provides both CLI and GUI project creation tools

!include MUI2.nsh
!include x64.nsh
!include LogicLib.nsh

; ============================================================================
; BASIC CONFIGURATION
; ============================================================================

Name "Weave - MayaFlux Initialization"
OutFile "Weave.exe"
Caption "Weave: MayaFlux Workspace Initialization"

RequestExecutionLevel admin
InstallDir "C:\MayaFlux"

; ============================================================================
; VARIABLES
; ============================================================================

Var InstallLog
Var MayaFluxRepo
Var MayaFluxTag
Var MayaFluxDownloadUrl
Var SkipDownload
Var TempDir
Var ScriptsDir
Var ASSET_NAME

; ============================================================================
; MODERN UI2 SETUP
; ============================================================================

!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

; ============================================================================
; INSTALLER INITIALIZATION
; ============================================================================

Function .onInit
  ${If} ${RunningX64}
    ; Running on 64-bit Windows - OK
  ${Else}
    MessageBox MB_OK|MB_ICONSTOP "This installer requires 64-bit Windows."
    Abort
  ${EndIf}
  
  StrCpy $TempDir "$TEMP\Weave-Install"
  CreateDirectory "$TempDir"
  
  StrCpy $InstallLog "$LOCALAPPDATA\weave_install.log"
  
  ; Initialize log
  FileOpen $0 "$InstallLog" w
  FileWrite $0 "==========================================\r\n"
  FileWrite $0 "Weave Installation Log\r\n"
  FileWrite $0 "Started: $(DATE) $(TIME)\r\n"
  FileWrite $0 "==========================================\r\n\r\n"
  FileClose $0
FunctionEnd

; ============================================================================
; UTILITY FUNCTIONS
; ============================================================================

Function LogMessage
  ; Input: message on stack
  Pop $0
  FileOpen $1 "$InstallLog" a
  FileSeek $1 0 END
  FileWrite $1 "[Weave] $0$\r$\n"
  FileClose $1
  DetailPrint "[Weave] $0"
FunctionEnd

Function LogError
  ; Input: error message on stack
  Pop $0
  FileOpen $1 "$InstallLog" a
  FileSeek $1 0 END
  FileWrite $1 "[Weave ERROR] $0$\r$\n"
  FileClose $1
  DetailPrint "[Weave ERROR] $0"
  MessageBox MB_OK|MB_ICONSTOP "[Weave] $0$\n$\nCheck log: $InstallLog"
  Abort
FunctionEnd

; ============================================================================
; INSTALLER SECTIONS
; ============================================================================

Section "MayaFlux Installation" SectionMayaFlux
  SectionIn RO
  
  Push "Starting MayaFlux installation"
  Call LogMessage
  
  DetailPrint "MayaFlux will be installed to: $INSTDIR"
  Push "Installation directory: $INSTDIR"
  Call LogMessage
  
  ; Step 1: Check if already installed
  Call CheckExistingInstallation
  
  ; Step 2: Download MayaFlux
  Call DownloadMayaFlux
  
  ; Step 3: Extract MayaFlux
  Call ExtractMayaFlux
  
SectionEnd

Section "Dependencies" SectionDeps
  SectionIn RO
  
  DetailPrint ""
  Push "Installing dependencies"
  Call LogMessage
  Call InstallDependencies
  
SectionEnd

Section "Environment Setup" SectionEnv
  SectionIn RO
  
  DetailPrint ""
  Push "Configuring environment"
  Call LogMessage
  Call SetupEnvironment
  
SectionEnd

Section "Weave Tools" SectionTools
  SectionIn RO
  
  DetailPrint ""
  Push "Installing Weave tools"
  Call LogMessage
  Call InstallWeaveTools
  
SectionEnd

Section "Weave GUI Application" SectionWeaveGui
  SectionIn RO
  
  Push "Installing Weave GUI application"
  Call LogMessage
  
  CreateDirectory "$INSTDIR\gui"
  CreateDirectory "$SMPROGRAMS\Weave"
  
  ; Copy GUI executable
  ${If} ${FileExists} "$EXEDIR\..\WeaveGUI\Weave.exe"
    SetOutPath "$INSTDIR\gui"
    File "$EXEDIR\..\WeaveGUI\Weave.exe"
    
    Push "Weave.exe installed to $INSTDIR\gui"
    Call LogMessage
    
    ; Create Start Menu shortcut
    CreateShortCut "$SMPROGRAMS\Weave\Weave Project Creator.lnk" \
      "$INSTDIR\gui\Weave.exe" \
      "" \
      "$INSTDIR\gui\Weave.exe" \
      0 \
      SW_SHOWNORMAL \
      "" \
      "Create new MayaFlux projects"
    
    Push "Start Menu shortcut created"
    Call LogMessage
    
    ; Create Desktop shortcut (optional)
    CreateShortCut "$DESKTOP\Weave.lnk" \
      "$INSTDIR\gui\Weave.exe" \
      "" \
      "$INSTDIR\gui\Weave.exe" \
      0
    
    Push "Desktop shortcut created"
    Call LogMessage
  ${Else}
    Push "WARNING: Weave.exe not found at $EXEDIR\..\WeaveGUI\Weave.exe"
    Call LogMessage
  ${EndIf}
  
SectionEnd

; ============================================================================
; IMPLEMENTATION FUNCTIONS
; ============================================================================

Function CheckExistingInstallation
  ${If} ${FileExists} "$INSTDIR\lib\MayaFluxLib.dll"
    Push "MayaFlux already installed, skipping download"
    Call LogMessage
    StrCpy $SkipDownload "true"
  ${Else}
    StrCpy $SkipDownload "false"
  ${EndIf}
FunctionEnd

; -----------------------------------------
; Download MayaFlux
; -----------------------------------------

Function DownloadMayaFlux
  ${If} $SkipDownload == "true"
    Return
  ${EndIf}

  Push "Fetching latest MayaFlux release..."
  Call LogMessage

  ; Call PowerShell script to fetch and download release
  nsExec::ExecToStack 'powershell -NoProfile -ExecutionPolicy Bypass -File "$EXEDIR\..\windows\scripts\get_mayaflux_release.ps1" -OutputDir "$TempDir" -Download'
  Pop $0

  ${If} $0 != 0
    ${If} ${FileExists} "$TempDir\error.txt"
      FileOpen $1 "$TempDir\error.txt" r
      FileRead $1 $1
      FileClose $1
      Push "ERROR: $1"
    ${Else}
      Push "ERROR: PowerShell script failed (exit code $0)"
    ${EndIf}
    Call LogError
    Return
  ${EndIf}

  ; Read tag and URL from files
  FileOpen $0 "$TempDir\tag.txt" r
  FileRead $0 $MayaFluxTag
  FileClose $0

  FileOpen $0 "$TempDir\url.txt" r
  FileRead $0 $MayaFluxDownloadUrl
  FileClose $0

  ${If} $MayaFluxTag == ""
    Push "ERROR: Could not parse release tag"
    Call LogError
    Return
  ${EndIf}

  ${If} $MayaFluxDownloadUrl == ""
    Push "ERROR: Could not parse download URL"
    Call LogError
    Return
  ${EndIf}

  Push "Latest release: $MayaFluxTag"
  Call LogMessage

  Push "Download successful"
  Call LogMessage
FunctionEnd

; -----------------------------------------
; Extract MayaFlux
; -----------------------------------------

Function ExtractMayaFlux
  ${If} $SkipDownload == "true"
    Return
  ${EndIf}
  
  Push "Extracting MayaFlux to $INSTDIR"
  Call LogMessage
  
  CreateDirectory "$INSTDIR"
  CreateDirectory "$TempDir\extract"
  
  ${If} ${FileExists} "$PROGRAMFILES64\7-Zip\7z.exe"
    DetailPrint "Using 7-Zip for extraction"
    nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TempDir\mayaflux.7z" -o"$TempDir\extract" -y'
    Pop $0
  ${Else}
    Push "ERROR: 7-Zip not found. Please install from https://www.7-zip.org/"
    Call LogMessage
    Return
  ${EndIf}
  
  ${If} $0 != 0
    Push "Extraction failed with exit code: $0"
    Call LogError
  ${EndIf}
  
  ; Use xcopy to move dist_staging contents to $INSTDIR
  nsExec::ExecToStack 'xcopy "$TempDir\extract\dist_staging\*" "$INSTDIR\" /E /Y /I'
  Pop $0
  
  ${If} $0 != 0
    Push "Copy failed with exit code: $0"
    Call LogError
  ${EndIf}
  
  ; Verify
  ${IfNot} ${FileExists} "$INSTDIR\lib\MayaFluxLib.lib"
    Push "Verification failed - MayaFluxLib.lib not found at $INSTDIR\lib"
    Call LogError
  ${EndIf}
  
  Push "MayaFlux extracted and verified successfully"
  Call LogMessage
  
  Delete "$TempDir\mayaflux.7z"
  RMDir /r "$TempDir\extract"
FunctionEnd

Function InstallDependencies
  Push "Checking for PowerShell dependency installer"
  Call LogMessage
  
  StrCpy $ScriptsDir "$EXEDIR\..\windows\scripts"
  
  ${If} ${FileExists} "$ScriptsDir\install_package.ps1"
    Push "Found install_package.ps1"
    Call LogMessage
    
    DetailPrint "Installing dependencies (this may take several minutes)..."
    
    ; Set env var so PowerShell can find the script
    nsExec::ExecToStack 'cmd /c "set SCRIPTS_DIR=$ScriptsDir && powershell -ExecutionPolicy Bypass -NoProfile -File "$ScriptsDir\install_package.ps1" -PackagesFile "$ScriptsDir\packages.psd1""'
    Pop $0
    
    ${If} $0 == 0
      Push "All dependencies installed successfully"
      Call LogMessage
    ${Else}
      Push "ERROR: Dependency installation failed with exit code: $0"
      Call LogError
    ${EndIf}
  ${Else}
    Push "ERROR: install_package.ps1 not found"
    Call LogError
  ${EndIf}
FunctionEnd

Function SetupEnvironment
  Push "Setting MAYAFLUX_ROOT environment variable"
  Call LogMessage
  
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "MAYAFLUX_ROOT" "$INSTDIR"
  
  Push "Adding MayaFlux to PATH"
  Call LogMessage
  
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "PATH"
  StrCmp $0 "" PathEmpty PathNotEmpty
  
  PathEmpty:
    StrCpy $0 "$INSTDIR\bin"
    Goto WritePath
  
  PathNotEmpty:
    ; Just prepend to PATH
    StrCpy $0 "$INSTDIR\bin;$0"
  
  WritePath:
    WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
      "PATH" "$0"
  
  Push "Setting CMAKE_PREFIX_PATH"
  Call LogMessage
  
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "CMAKE_PREFIX_PATH" "$INSTDIR"
  
  ; Broadcast environment change
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  
  Push "Environment configured"
  Call LogMessage
FunctionEnd

Function InstallWeaveTools
  CreateDirectory "$INSTDIR\share\weave\templates"
  
  ; Try multiple possible locations for templates
  ; Build dir structure: build/windows/Weave.exe
  ; So EXEDIR = build/windows
  ; Templates should be at: build/windows/templates (copied during build)
  
  Push "Installing project templates..."
  Call LogMessage
  
  ; Try: build/templates (copied during installer build)
  ${If} ${FileExists} "$EXEDIR\templates\*.*"
    Push "Found templates at: $EXEDIR\templates"
    Call LogMessage
    
    nsExec::ExecToStack 'xcopy "$EXEDIR\templates\*" "$INSTDIR\share\weave\templates\" /E /Y /I'
    Pop $0
    
    ${If} $0 == "0"
      Push "Project templates installed successfully"
      Call LogMessage
      Return
    ${EndIf}
  ${EndIf}
  
  ; Try: repo root/templates (if running from repo)
  ${If} ${FileExists} "$EXEDIR\..\..\..\templates\*.*"
    Push "Found templates at: $EXEDIR\..\..\..\templates"
    Call LogMessage
    
    nsExec::ExecToStack 'xcopy "$EXEDIR\..\..\..\templates\*" "$INSTDIR\share\weave\templates\" /E /Y /I'
    Pop $0
    
    ${If} $0 == "0"
      Push "Project templates installed successfully"
      Call LogMessage
      Return
    ${EndIf}
  ${EndIf}
  
  ; If we get here, templates weren't found but don't fail the install
  Push "WARNING: Project templates not found during installation"
  Call LogMessage
  Push "WARNING: GUI may not work correctly until templates are manually copied to: $INSTDIR\share\weave\templates"
  Call LogMessage
  Push "WARNING: You can copy from: <repo>\templates to the above location"
  Call LogMessage
  
  ToolsComplete:
    Push "Weave tools installation complete"
    Call LogMessage
FunctionEnd

; ============================================================================
; COMPLETION
; ============================================================================

Function .onInstSuccess
  DetailPrint ""
  DetailPrint "=========================================="
  DetailPrint "MayaFlux Installation Complete!"
  DetailPrint "=========================================="
  DetailPrint ""
  DetailPrint "Next steps:"
  DetailPrint "  1. Restart your terminal or Command Prompt"
  DetailPrint ""
  DetailPrint "  2. Create a new project:"
  DetailPrint "     weave new MyProject"
  DetailPrint ""
  DetailPrint "  3. Build and run:"
  DetailPrint "     cd MyProject"
  DetailPrint "     mkdir build && cd build"
  DetailPrint "     cmake .. && cmake --build . --config Release"
  DetailPrint "     .\MyProject.exe"
  DetailPrint ""
  DetailPrint "Documentation: https://github.com/MayaFlux/MayaFlux"
  DetailPrint "Installation log: $InstallLog"
  DetailPrint ""
  
  ; Open log file
  ExecShell "open" "$InstallLog"

 Push "Installation succeeded. Launching Weave GUI..."
  Call LogMessage
  
  ; Launch the GUI application
  ExecShell "open" "$INSTDIR\gui\Weave.exe"
  
  DetailPrint ""
  DetailPrint "Weave Project Creator is launching..."
  DetailPrint ""
  DetailPrint "Documentation: https://github.com/MayaFlux/MayaFlux"
  DetailPrint "Installation log: $InstallLog"
  DetailPrint ""
FunctionEnd

Function .onInstFailed
  DetailPrint ""
  DetailPrint "Installation failed"
  DetailPrint "Check log for details: $InstallLog"
  
  Push "Installation failed. See log file for details."
  Call LogMessage
FunctionEnd
