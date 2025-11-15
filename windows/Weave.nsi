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

Function DownloadMayaFlux
  ${If} $SkipDownload == "true"
    Return
  ${EndIf}

  Push "Fetching latest MayaFlux release info..."
  Call LogMessage

  StrCpy $MayaFluxRepo "MayaFlux/MayaFlux"

  ; -----------------------------------------
  ; 1. Get release tag via PowerShell
  ; -----------------------------------------
  nsExec::ExecToStack 'powershell -NoProfile -Command "(Invoke-WebRequest ''https://github.com/MayaFlux/MayaFlux/releases'').Content | Select-String -Pattern ''/MayaFlux/MayaFlux/releases/tag/([^\"\""]+)'' | Select-Object -First 1 | ForEach-Object { $_.Matches[0].Groups[1].Value }"'
  Pop $0
  Pop $MayaFluxTag

  ${If} $MayaFluxTag == ""
    Push "ERROR: Could not detect latest release tag"
    Call LogError
    Return
  ${EndIf}

  Push "Latest release: $MayaFluxTag"
  Call LogMessage

  ; -----------------------------------------
  ; 2. Get Windows asset name
  ; -----------------------------------------
  nsExec::ExecToStack 'powershell -NoProfile -Command "$u=''https://github.com/MayaFlux/MayaFlex/releases/expanded_assets/' + ''$MayaFluxTag''; (Invoke-WebRequest $u).Content | Select-String -Pattern ''MayaFlux-.*windows.*\.zip'' | Select-Object -First 1 | ForEach-Object { $_.Matches[0].Value }"'
  Pop $0
  Pop $ASSET_NAME

  ${If} $ASSET_NAME == ""
    Push "ERROR: No Windows asset found for $MayaFluxTag"
    Call LogError
    Return
  ${EndIf}

  Push "Found asset: $ASSET_NAME"
  Call LogMessage

  ; -----------------------------------------
  ; 3. Build final download URL
  ; -----------------------------------------
  StrCpy $MayaFluxDownloadUrl "https://github.com/MayaFlux/MayaFlux/releases/download/$MayaFluxTag/$ASSET_NAME"

  DetailPrint "Downloading: $MayaFluxDownloadUrl"
  Push "Downloading from: $MayaFluxDownloadUrl"
  Call LogMessage

  ; -----------------------------------------
  ; 4. Download the asset
  ; -----------------------------------------
  NSISdl::download "$MayaFluxDownloadUrl" "$TempDir\mayaflux.zip"
  Pop $0

  ${If} $0 != "success"
    Push "Download failed: $0"
    Call LogError
  ${EndIf}

  Push "Download successful"
  Call LogMessage
FunctionEnd

Function ExtractMayaFlux
  ${If} $SkipDownload == "true"
    Return
  ${EndIf}
  
  Push "Extracting MayaFlux to $INSTDIR"
  Call LogMessage
  
  CreateDirectory "$INSTDIR"
  
  ; Try 7-Zip first (better for large archives)
  ${If} ${FileExists} "$PROGRAMFILES64\7-Zip\7z.exe"
    DetailPrint "Using 7-Zip for extraction"
    nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TempDir\mayaflux.zip" -o"$INSTDIR" -y'
    Pop $0
  ${Else}
    ; Fall back to PowerShell Expand-Archive
    DetailPrint "Using PowerShell for extraction"
    nsExec::ExecToStack 'powershell -Command "Expand-Archive -Path ''$TempDir\mayaflux.zip'' -DestinationPath ''$INSTDIR'' -Force"'
    Pop $0
  ${EndIf}
  
  ${If} $0 != 0
    Push "Extraction failed with exit code: $0"
    Call LogError
  ${EndIf}
  
  ; Verify extraction
  ${IfNot} ${FileExists} "$INSTDIR\lib\MayaFluxLib.dll"
    Push "Verification failed - MayaFluxLib.dll not found at $INSTDIR\lib"
    Call LogError
  ${EndIf}
  
  Push "MayaFlux extracted successfully"
  Call LogMessage
  
  ; Cleanup download
  Delete "$TempDir\mayaflux.zip"
FunctionEnd

Function InstallDependencies
  Push "Checking for PowerShell dependency installer"
  Call LogMessage
  
  StrCpy $ScriptsDir "$EXEDIR\..\windows\scripts"
  
  ${If} ${FileExists} "$ScriptsDir\install_package.ps1"
    Push "Found install_package.ps1 at: $ScriptsDir"
    Call LogMessage
    
    Push "Running PowerShell package installer"
    Call LogMessage
    
    DetailPrint "Installing dependencies (this may take several minutes)..."
    
    nsExec::ExecToStack 'powershell -ExecutionPolicy Bypass -NoProfile -File "$ScriptsDir\install_package.ps1" -PackagesFile "$ScriptsDir\packages.psd1"'
    Pop $0
    
    ${If} $0 != 0
      Push "WARNING: Dependency installation completed with exit code: $0. Some packages may need manual installation."
      Call LogMessage
    ${Else}
      Push "All dependencies installed successfully"
      Call LogMessage
    ${EndIf}
  ${Else}
    Push "WARNING: install_package.ps1 not found at $ScriptsDir"
    Call LogMessage
    Push "WARNING: Dependencies were not automatically installed. You can run setup later:"
    Call LogMessage
    Push "WARNING:   powershell -ExecutionPolicy Bypass $ScriptsDir\install_package.ps1"
    Call LogMessage
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
  
  StrCpy $ScriptsDir "$EXEDIR\..\windows\scripts"
  
  IfFileExists "$EXEDIR\..\templates\*.*" TemplatesFound TemplatesNotFound
  
  TemplatesFound:
    Push "Installing project templates"
    Call LogMessage
    
    nsExec::ExecToStack 'xcopy "$EXEDIR\..\templates\*" "$INSTDIR\share\weave\templates\" /E /Y /I'
    Pop $0
    
    StrCmp $0 "0" TemplatesSuccess TemplatesWarning
  
  TemplatesSuccess:
    Push "Project templates installed"
    Call LogMessage
    Goto ToolsComplete
  
  TemplatesWarning:
    Push "WARNING: Template installation may have completed with warnings"
    Call LogMessage
    Goto ToolsComplete
  
  TemplatesNotFound:
    Push "WARNING: Templates directory not found at $EXEDIR\..\templates"
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
FunctionEnd

Function .onInstFailed
  DetailPrint ""
  DetailPrint "Installation failed"
  DetailPrint "Check log for details: $InstallLog"
  
  Push "Installation failed. See log file for details."
  Call LogMessage
FunctionEnd
