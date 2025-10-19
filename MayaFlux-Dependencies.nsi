!include MUI2.nsh
!include x64.nsh
!include LogicLib.nsh
!include FileFunc.nsh
!include WordFunc.nsh

; Basic installer info
Name "MayaFlux Development Environment"
OutFile "MayaFlux-Setup.exe"
Caption "Setup MayaFlux dependencies"
BrandingText "MayaFlux Development Environment"

; Require admin privileges
RequestExecutionLevel admin

; Install directory
InstallDir "C:\MayaFlux"

; Variables
Var VSWherePath
Var DiaSdkPath
Var VulkanSdkPath
Var DetectedVsVersion
Var PythonPath
Var CmakePath
Var GitPath
Var SevenZipPath

; Modern UI2 configuration
!define MUI_ABORTWARNING

; Installer pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE.txt"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Language
!insertmacro MUI_LANGUAGE "English"

; Sections
Section "MayaFlux Core" SectionCore
  SectionIn RO
  SetOutPath "$INSTDIR"
  
  ; Create directories
  CreateDirectory "$INSTDIR\bin"
  CreateDirectory "$INSTDIR\lib"
  CreateDirectory "$INSTDIR\include"
  CreateDirectory "$INSTDIR\cmake"
  CreateDirectory "$INSTDIR\projects"
  
  ; Install documentation and examples
  File "README.txt"
  File "LICENSE.txt"
  
  ; Write installation info
  WriteUninstaller "$INSTDIR\Uninstall.exe"
 
  ; Add to add/remove programs
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\MayaFlux" \
                   "DisplayName" "MayaFlux Development Environment"
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\MayaFlux" \
                   "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\MayaFlux" \
                   "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\MayaFlux" \
                   "Publisher" "MayaFlux Project"
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\MayaFlux" \
                   "DisplayVersion" "1.0.0"
  WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\MayaFlux" \
                   "NoModify" 1
  WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\MayaFlux" \
                   "NoRepair" 1
SectionEnd

Section "Build Tools" SectionBuildTools
  SectionIn RO
  Call CheckAndInstallBuildTools
SectionEnd

Section "LLVM (Required for JIT)" SectionLLVM
  SectionIn RO
  Call CheckAndInstallLLVM
SectionEnd

Section "Visual Studio Dependencies" SectionVS
  SectionIn RO
  Call CheckVisualStudio
  Call FindDiaSdk
SectionEnd

Section "Vulkan SDK" SectionVulkan
  Call InstallVulkanSDK
SectionEnd

Section "FFmpeg" SectionFFmpeg
  Call InstallFFmpeg
SectionEnd

Section "GLFW" SectionGLFW
  Call InstallGLFW
SectionEnd

Section "RtAudio" SectionRtAudio
  Call InstallRtAudio
SectionEnd

Section "Eigen3" SectionEigen
  Call InstallEigen
SectionEnd

Section "Magic Enum" SectionMagicEnum
  Call InstallMagicEnum
SectionEnd

Section "LibXml2" SectionLibXml2
  Call InstallLibXml2
SectionEnd

Section "Environment Variables" SectionEnv
  SectionIn RO
  Call SetupEnvironmentVariables
SectionEnd

Section "Generate Visual Studio Project" SectionVSProject
  Call GenerateVSProject
SectionEnd

; Function definitions
Function .onInit
  ; Check if running on 64-bit Windows
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "This installer is for 64-bit Windows only."
    Abort
  ${EndIf}
  
  ; Initialize section selection
  !insertmacro SelectSection ${SectionCore}
  !insertmacro SelectSection ${SectionBuildTools}
  !insertmacro SelectSection ${SectionLLVM}
  !insertmacro SelectSection ${SectionVS}
  !insertmacro SelectSection ${SectionVulkan}
  !insertmacro SelectSection ${SectionEnv}
FunctionEnd

Function CheckAndInstallBuildTools
  DetailPrint "Checking for required build tools..."
  
  ; Check for CMake
  Call CheckCmake
  Pop $0
  ${If} $0 == "not_found"
    DetailPrint "Installing CMake..."
    NSISdl::download_quiet \
      "https://github.com/Kitware/CMake/releases/download/v3.28.1/cmake-3.28.1-windows-x86_64.msi" \
      "$TEMP\cmake.msi"
    Pop $1
    StrCmp $1 "success" +3
      MessageBox MB_OK|MB_ICONSTOP "Failed to download CMake"
      Abort
    
    ExecWait 'msiexec /i "$TEMP\cmake.msi" /quiet /norestart' $2
    StrCmp $2 0 +3
      MessageBox MB_OK|MB_ICONEXCLAMATION "CMake installation may have completed with warnings"
    
    Delete "$TEMP\cmake.msi"
  ${EndIf}
  
  ; Check for Git
  Call CheckGit
  Pop $0
  ${If} $0 == "not_found"
    DetailPrint "Installing Git..."
    NSISdl::download_quiet \
      "https://github.com/git-for-windows/git/releases/download/v2.43.0.windows.1/Git-2.43.0-64-bit.exe" \
      "$TEMP\git.exe"
    Pop $1
    StrCmp $1 "success" +3
      MessageBox MB_OK|MB_ICONSTOP "Failed to download Git"
      Abort
    
    ExecWait '"$TEMP\git.exe" /SILENT /NORESTART' $2
    StrCmp $2 0 +3
      MessageBox MB_OK|MB_ICONEXCLAMATION "Git installation may have completed with warnings"
    
    Delete "$TEMP\git.exe"
  ${EndIf}
  
  ; Check for 7-Zip (needed for extractions)
  Call Check7Zip
  Pop $0
  ${If} $0 == "not_found"
    DetailPrint "Installing 7-Zip..."
    NSISdl::download_quiet \
      "https://www.7-zip.org/a/7z2407-x64.exe" \
      "$TEMP\7z.exe"
    Pop $1
    StrCmp $1 "success" +3
      MessageBox MB_OK|MB_ICONSTOP "Failed to download 7-Zip"
      Abort
    
    ExecWait '"$TEMP\7z.exe" /S' $2
    StrCmp $2 0 +3
      MessageBox MB_OK|MB_ICONEXCLAMATION "7-Zip installation may have completed with warnings"
    
    Delete "$TEMP\7z.exe"
  ${EndIf}
  
  DetailPrint "Build tools check complete"
FunctionEnd

Function CheckCmake
  ClearErrors
  ReadRegStr $0 HKLM "SOFTWARE\Kitware\CMake" "InstallDir"
  IfErrors 0 CmakeFound
  
  ; Also check in PATH
  nsExec::ExecToStack 'cmake --version'
  Pop $0
  Pop $1
  StrCmp $0 0 CmakeFound
  
  StrCpy $0 "not_found"
  Goto CmakeDone
  
  CmakeFound:
    StrCpy $CmakePath $0
    StrCpy $0 "found"
  
  CmakeDone:
    Push $0
FunctionEnd

Function CheckGit
  ClearErrors
  ReadRegStr $0 HKLM "SOFTWARE\GitForWindows" "InstallPath"
  IfErrors 0 GitFound
  
  ; Also check in PATH
  nsExec::ExecToStack 'git --version'
  Pop $0
  Pop $1
  StrCmp $0 0 GitFound
  
  StrCpy $0 "not_found"
  Goto GitDone
  
  GitFound:
    StrCpy $GitPath $0
    StrCpy $0 "found"
  
  GitDone:
    Push $0
FunctionEnd

Function Check7Zip
  ClearErrors
  ReadRegStr $0 HKLM "SOFTWARE\7-Zip" "Path"
  IfErrors 0 SevenZipFound
  
  ; Check common installation path
  IfFileExists "$PROGRAMFILES64\7-Zip\7z.exe" 0 SevenZipNotFound
  
  SevenZipFound:
    StrCpy $SevenZipPath $0
    StrCpy $0 "found"
    Goto SevenZipDone
  
  SevenZipNotFound:
    StrCpy $0 "not_found"
  
  SevenZipDone:
    Push $0
FunctionEnd

Function CheckAndInstallLLVM
  DetailPrint "Checking for LLVM development files..."
  
  ; Strategy: Check multiple possible LLVM installations
  ; 1. Check our custom installation first
  ; 2. Check system LLVM with CMake config
  ; 3. Download and install our version if needed
  
  ; Check our custom installation
  IfFileExists "C:\Program Files\LLVM_Libs\lib\cmake\llvm\LLVMConfig.cmake" LLVMExists
  
  ; Check system LLVM installations
  ReadRegStr $0 HKLM "SOFTWARE\LLVM\LLVM" ""
  ${If} $0 != ""
    IfFileExists "$0\lib\cmake\llvm\LLVMConfig.cmake" SystemLLVMFound
  ${EndIf}
  
  ; Check common LLVM installation paths
  ${ForEach} $1 13 21 + 1
    ${If} ${FileExists} "C:\Program Files\LLVM\$1\lib\cmake\llvm\LLVMConfig.cmake"
      StrCpy $0 "C:\Program Files\LLVM\$1"
      Goto SystemLLVMFound
    ${EndIf}
  ${Next}
  
  ; No suitable LLVM found, install our version
  Goto InstallLLVM
  
  SystemLLVMFound:
    DetailPrint "Found system LLVM at: $0"
    ; Verify it has the required components
    IfFileExists "$0\include\llvm\IR\LLVMContext.h" 0 InstallLLVM
    IfFileExists "$0\lib\LLVMCore.lib" 0 InstallLLVM
    DetailPrint "System LLVM has development files, using existing installation"
    Goto LLVMExists
  
  InstallLLVM:
    DetailPrint "Downloading LLVM 21.1.3 (development version)..."
    NSISdl::download_quiet \
      "https://github.com/llvm/llvm-project/releases/download/llvmorg-21.1.3/clang+llvm-21.1.3-x86_64-pc-windows-msvc.tar.xz" \
      "$TEMP\llvm.tar.xz"
    Pop $0
    StrCmp $0 "success" +3
      MessageBox MB_OK|MB_ICONSTOP "Failed to download LLVM: $0"
      Abort
    
    DetailPrint "Extracting LLVM to Program Files..."
    CreateDirectory "C:\Program Files\LLVM_Libs"
    
    ; Use tar if available, otherwise 7-Zip
    IfFileExists "$SYSDIR\tar.exe" UseTar
    IfFileExists "$PROGRAMFILES64\7-Zip\7z.exe" Use7Zip
    MessageBox MB_OK|MB_ICONSTOP "No extraction tool available (tar or 7-Zip required)"
    Abort
    
    UseTar:
      nsExec::ExecToStack 'tar -xf "$TEMP\llvm.tar.xz" -C "C:\Program Files\LLVM_Libs" --strip-components=1'
      Pop $0
      Goto ExtractCheck
      
    Use7Zip:
      ; Extract .tar.xz requires two steps with 7-Zip
      CreateDirectory "$TEMP\llvm_temp"
      nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TEMP\llvm.tar.xz" -o"$TEMP\llvm_temp" -y'
      Pop $0
      ${If} $0 == 0
        ; Find the .tar file that was extracted
        FindFirst $1 $2 "$TEMP\llvm_temp\*.tar"
        ${If} $2 != ""
          nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TEMP\llvm_temp\$2" -o"C:\Program Files\LLVM_Libs" -y'
          Pop $0
        ${Else}
          StrCpy $0 1 ; Error
        ${EndIf}
        FindClose $1
      ${EndIf}
    
    ExtractCheck:
    ${If} $0 != 0
      MessageBox MB_OK|MB_ICONSTOP "LLVM extraction failed"
      Abort
    ${EndIf}
    
    ; Verify extraction
    IfFileExists "C:\Program Files\LLVM_Libs\lib\cmake\llvm\LLVMConfig.cmake" +3
      MessageBox MB_OK|MB_ICONSTOP "LLVM extraction verification failed"
      Abort
    
    DetailPrint "LLVM installed successfully"
  
  LLVMExists:
    DetailPrint "LLVM development files available"
  
  ; Cleanup
  Delete "$TEMP\llvm.tar.xz"
  RMDir /r "$TEMP\llvm_temp"
FunctionEnd

Function CheckVisualStudio
  DetailPrint "Checking for Visual Studio..."
  
  StrCpy $VSWherePath "$PROGRAMFILES32\Microsoft Visual Studio\Installer\vswhere.exe"
  IfFileExists $VSWherePath VSFound
  
  MessageBox MB_YESNO|MB_ICONQUESTION \
    "Visual Studio Build Tools not found.$\n$\n\
     MayaFlux requires Visual Studio 2019 or 2022 with C++ tools.$\n\
     Install Visual Studio Build Tools now?" \
     IDYES InstallVSTools IDNO VSCancel
     
  InstallVSTools:
    DetailPrint "Downloading Visual Studio Build Tools..."
    NSISdl::download_quiet \
      "https://aka.ms/vs/17/release/vs_BuildTools.exe" \
      "$TEMP\vs_buildtools.exe"
    Pop $0
    StrCmp $0 "success" +3
      MessageBox MB_OK|MB_ICONSTOP "Failed to download Visual Studio Build Tools"
      Abort
    
    DetailPrint "Installing Visual Studio Build Tools (this may take several minutes)..."
    ExecWait '"$TEMP\vs_buildtools.exe" --quiet --wait --norestart --includeRecommended --add Microsoft.VisualStudio.Workload.VCTools' $0
    StrCmp $0 0 +3
      MessageBox MB_OK|MB_ICONSTOP "Visual Studio Build Tools installation failed"
      Abort
    
    Delete "$TEMP\vs_buildtools.exe"
    Goto VSFound
  
  VSCancel:
    MessageBox MB_OK|MB_ICONSTOP "Visual Studio Build Tools are required for MayaFlux."
    Abort
  
  VSFound:
  DetailPrint "Visual Studio found"
FunctionEnd

Function FindDiaSdk
  DetailPrint "Locating DIA SDK..."
  
  StrCpy $VSWherePath "$PROGRAMFILES32\Microsoft Visual Studio\Installer\vswhere.exe"
  IfFileExists $VSWherePath +3
    MessageBox MB_OK|MB_ICONSTOP "vswhere.exe not found"
    Abort
  
  ; Check common DIA SDK locations
  StrCpy $DiaSdkPath ""
  
  ${ForEach} $0 2017 2022 + 1
    ${If} ${FileExists} "C:\Program Files\Microsoft Visual Studio\$0\Professional\DIA SDK\lib\amd64\diaguids.lib"
      StrCpy $DiaSdkPath "C:\Program Files\Microsoft Visual Studio\$0\Professional\DIA SDK"
      Goto DiaFound
    ${EndIf}
    ${If} ${FileExists} "C:\Program Files\Microsoft Visual Studio\$0\Community\DIA SDK\lib\amd64\diaguids.lib"
      StrCpy $DiaSdkPath "C:\Program Files\Microsoft Visual Studio\$0\Community\DIA SDK" 
      Goto DiaFound
    ${EndIf}
    ${If} ${FileExists} "C:\Program Files (x86)\Microsoft Visual Studio\$0\Professional\DIA SDK\lib\amd64\diaguids.lib"
      StrCpy $DiaSdkPath "C:\Program Files (x86)\Microsoft Visual Studio\$0\Professional\DIA SDK"
      Goto DiaFound
    ${EndIf}
    ${If} ${FileExists} "C:\Program Files (x86)\Microsoft Visual Studio\$0\Community\DIA SDK\lib\amd64\diaguids.lib"
      StrCpy $DiaSdkPath "C:\Program Files (x86)\Microsoft Visual Studio\$0\Community\DIA SDK"
      Goto DiaFound
    ${EndIf}
  ${Next}
  
  ; Check registry for older versions
  ${ForEach} $1 14.0 16.0 + 1.0
    ReadRegStr $2 HKLM "SOFTWARE\Microsoft\VisualStudio\$1\Setup\VS" "ProductDir"
    ${If} $2 != ""
      ${If} ${FileExists} "$2DIA SDK\lib\amd64\diaguids.lib"
        StrCpy $DiaSdkPath "$2DIA SDK"
        Goto DiaFound
      ${EndIf}
    ${EndIf}
  ${Next}
  
  MessageBox MB_OK|MB_ICONSTOP "DIA SDK not found. Please install Visual Studio with C++ development tools."
  Abort
  
  DiaFound:
  DetailPrint "DIA SDK found at: $DiaSdkPath"
  
  ; Create compatibility layer for LLVM
  CreateDirectory "C:\Program Files\LLVM_Libs\dia_sdk_compat\lib\amd64"
  CopyFiles "$DiaSdkPath\lib\amd64\diaguids.lib" "C:\Program Files\LLVM_Libs\dia_sdk_compat\lib\amd64\"
FunctionEnd

Function InstallVulkanSDK
  DetailPrint "Checking Vulkan SDK..."
  
  ; Check if already installed
  ReadRegStr $0 HKLM "SOFTWARE\Khronos\VulkanSDK" "InstalledVersion"
  IfErrors 0 VulkanExists
  
  DetailPrint "Installing Vulkan SDK..."
  NSISdl::download_quiet \
    "https://sdk.lunarg.com/sdk/download/latest/windows/vulkan-sdk.exe" \
    "$TEMP\vulkan-sdk.exe"
  Pop $1
  StrCmp $1 "success" +3
    MessageBox MB_OK|MB_ICONSTOP "Failed to download Vulkan SDK"
    Abort
  
  DetailPrint "Running Vulkan SDK installer..."
  ExecWait '"$TEMP\vulkan-sdk.exe" /S' $2
  ${If} $2 != 0
    MessageBox MB_OK|MB_ICONEXCLAMATION "Vulkan SDK installation may have completed with warnings"
  ${EndIf}
  
  Delete "$TEMP\vulkan-sdk.exe"
  
  VulkanExists:
  ReadRegStr $VulkanSdkPath HKLM "SOFTWARE\Khronos\VulkanSDK" "InstalledPath"
  DetailPrint "Vulkan SDK installed at: $VulkanSdkPath"
FunctionEnd

Function InstallFFmpeg
  DetailPrint "Installing FFmpeg..."
  
  IfFileExists "C:\Program Files\FFmpeg\bin\ffmpeg.exe" FFmpegExists
  
  NSISdl::download_quiet \
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full-shared.7z" \
    "$TEMP\ffmpeg.7z"
  Pop $0
  StrCmp $0 "success" +3
    MessageBox MB_OK|MB_ICONSTOP "Failed to download FFmpeg"
    Abort
  
  CreateDirectory "C:\Program Files\FFmpeg"
  
  ; Extract with 7-Zip
  IfFileExists "$PROGRAMFILES64\7-Zip\7z.exe" +3
    MessageBox MB_OK|MB_ICONSTOP "7-Zip required for FFmpeg extraction"
    Abort
  
  nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TEMP\ffmpeg.7z" -o"C:\Program Files\FFmpeg" -y'
  Pop $0
  
  ; Move from versioned subdirectory
  FindFirst $1 $2 "C:\Program Files\FFmpeg\ffmpeg-*"
  StrCmp $2 "" FFmpegCheck
    CopyFiles /SILENT "C:\Program Files\FFmpeg\$2\*" "C:\Program Files\FFmpeg\"
    RMDir /r "C:\Program Files\FFmpeg\$2"
    FindClose $1
  
  FFmpegCheck:
  IfFileExists "C:\Program Files\FFmpeg\bin\ffmpeg.exe" FFmpegExists
    MessageBox MB_OK|MB_ICONSTOP "FFmpeg installation failed"
    Abort
  
  FFmpegExists:
  DetailPrint "FFmpeg installed"
  Delete "$TEMP\ffmpeg.7z"
FunctionEnd

Function InstallGLFW
  DetailPrint "Installing GLFW..."
  
  ; Check for existing GLFW with DLL support
  Call CheckGLFWWithDLL
  Pop $0
  ${If} $0 == "found"
    DetailPrint "Compatible GLFW already installed"
    Return
  ${EndIf}
  
  ; Install our version
  NSISdl::download_quiet \
    "https://github.com/glfw/glfw/releases/download/3.4/glfw-3.4.bin.WIN64.zip" \
    "$TEMP\glfw.zip"
  Pop $0
  StrCmp $0 "success" +3
    MessageBox MB_OK|MB_ICONSTOP "Failed to download GLFW"
    Abort
  
  CreateDirectory "C:\Program Files\GLFW"
  IfFileExists "$PROGRAMFILES64\7-Zip\7z.exe" +3
    MessageBox MB_OK|MB_ICONSTOP "7-Zip required for FFmpeg extraction"
    Abort
  nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TEMP\glfw.zip" -o"C:\Program Files\GLFW" -y'
  Pop $0
  
  ; Move from versioned subdirectory
  FindFirst $1 $2 "C:\Program Files\GLFW\glfw-*"
  StrCmp $2 "" GLFWCheck
    CopyFiles /SILENT "C:\Program Files\GLFW\$2\*" "C:\Program Files\GLFW\"
    RMDir /r "C:\Program Files\GLFW\$2"
    FindClose $1
  
  GLFWCheck:
  IfFileExists "C:\Program Files\GLFW\include\GLFW\glfw3.h" GLFWExists
    MessageBox MB_OK|MB_ICONSTOP "GLFW installation failed"
    Abort
  
  GLFWExists:
  DetailPrint "GLFW installed"
  Delete "$TEMP\glfw.zip"
FunctionEnd

Function CheckGLFWWithDLL
  ; Check common GLFW installation paths for DLL support
  ${ForEach} $0 3 4 + 1
    ${If} ${FileExists} "C:\Program Files\GLFW\lib-vc2022\glfw3.dll"
      StrCpy $1 "found"
      Goto CheckDone
    ${EndIf}
    ${If} ${FileExists} "C:\Program Files\GLFW\lib-vc2019\glfw3.dll"
      StrCpy $1 "found"
      Goto CheckDone
    ${EndIf}
  ${Next}
  
  StrCpy $1 "not_found"
  
  CheckDone:
  Push $1
FunctionEnd

Function InstallRtAudio
  DetailPrint "Installing RtAudio..."
  
  IfFileExists "C:\Program Files\RtAudio\include\rtaudio\RtAudio.h" RtAudioExists
  
  ; Download and build RtAudio
  DetailPrint "Downloading RtAudio source..."
  NSISdl::download_quiet \
    "https://github.com/thestk/rtaudio/archive/refs/tags/6.0.1.zip" \
    "$TEMP\rtaudio.zip"
  Pop $0
  StrCmp $0 "success" +3
    MessageBox MB_OK|MB_ICONSTOP "Failed to download RtAudio"
    Abort
  
  CreateDirectory "$TEMP\rtaudio-src"
  IfFileExists "$PROGRAMFILES64\7-Zip\7z.exe" +3
    MessageBox MB_OK|MB_ICONSTOP "7-Zip required for FFmpeg extraction"
    Abort

  nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TEMP\rtaudio.zip" -o"$TEMP\rtaudio-src" -y'
  Pop $0
  
  DetailPrint "Building RtAudio (this may take a moment)..."
  
  ; Create simple CMake build script
  FileOpen $0 "$TEMP\build_rtaudio.bat" w
  FileWrite $0 '@echo off$\r$\n'
  FileWrite $0 'set VSWHERE="%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"$\r$\n'
  FileWrite $0 'for /f "usebackq tokens=*" %%i in (`%VSWHERE% -latest -property installationPath`) do set VSPATH=%%i$\r$\n'
  FileWrite $0 'call "%VSPATH%\VC\Auxiliary\Build\vcvars64.bat"$\r$\n'
  FileWrite $0 'cmake -S "$TEMP\rtaudio-src\rtaudio-6.0.1" -B "$TEMP\rtaudio-build" -DCMAKE_INSTALL_PREFIX="C:\Program Files\RtAudio" -DCMAKE_BUILD_TYPE=Release -DRTAUDIO_BUILD_TESTING=OFF -DRTAUDIO_API_WASAPI=ON -DRTAUDIO_API_DS=ON -G "Visual Studio 17 2022" -A x64$\r$\n'
  FileWrite $0 'cmake --build "$TEMP\rtaudio-build" --config Release$\r$\n'
  FileWrite $0 'cmake --install "$TEMP\rtaudio-build" --config Release$\r$\n'
  FileClose $0
  
  nsExec::ExecToStack '"$TEMP\build_rtaudio.bat"'
  Pop $0
  
  RtAudioExists:
  DetailPrint "RtAudio installed"
  Delete "$TEMP\rtaudio.zip"
  RMDir /r "$TEMP\rtaudio-src"
  RMDir /r "$TEMP\rtaudio-build"
  Delete "$TEMP\build_rtaudio.bat"
FunctionEnd

Function InstallEigen
  DetailPrint "Installing Eigen3..."
  
  IfFileExists "C:\Program Files\Eigen3\Eigen\Dense" EigenExists
  
  NSISdl::download_quiet \
    "https://gitlab.com/libeigen/eigen/-/archive/3.4.0/eigen-3.4.0.zip" \
    "$TEMP\eigen.zip"
  Pop $0
  StrCmp $0 "success" +3
    MessageBox MB_OK|MB_ICONSTOP "Failed to download Eigen3"
    Abort
  
  CreateDirectory "C:\Program Files\Eigen3"
  CreateDirectory "$TEMP\rtaudio-src"
  IfFileExists "$PROGRAMFILES64\7-Zip\7z.exe" +3
    MessageBox MB_OK|MB_ICONSTOP "7-Zip required for FFmpeg extraction"
    Abort

  nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TEMP\eigen.zip" -o"C:\Program Files\Eigen3" -y'
  Pop $0
  
  ; Move from versioned subdirectory
  FindFirst $1 $2 "C:\Program Files\Eigen3\eigen-*"
  StrCmp $2 "" EigenCheck
    CopyFiles /SILENT "C:\Program Files\Eigen3\$2\*" "C:\Program Files\Eigen3\"
    RMDir /r "C:\Program Files\Eigen3\$2"
    FindClose $1
  
  EigenCheck:
  IfFileExists "C:\Program Files\Eigen3\Eigen\Dense" EigenExists
    MessageBox MB_OK|MB_ICONSTOP "Eigen3 installation failed"
    Abort
  
  EigenExists:
  DetailPrint "Eigen3 installed"
  Delete "$TEMP\eigen.zip"
FunctionEnd

Function InstallMagicEnum
  DetailPrint "Installing Magic Enum..."
  
  IfFileExists "C:\Program Files\magic_enum\include\magic_enum.hpp" MagicEnumExists
  
  NSISdl::download_quiet \
    "https://github.com/Neargye/magic_enum/archive/refs/tags/v0.9.5.zip" \
    "$TEMP\magic_enum.zip"
  Pop $0
  StrCmp $0 "success" +3
    MessageBox MB_OK|MB_ICONSTOP "Failed to download Magic Enum"
    Abort
  
  CreateDirectory "C:\Program Files\magic_enum\include"
  CreateDirectory "$TEMP\rtaudio-src"
  IfFileExists "$PROGRAMFILES64\7-Zip\7z.exe" +3
    MessageBox MB_OK|MB_ICONSTOP "7-Zip required for FFmpeg extraction"
    Abort

  nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TEMP\magic_enum.zip" -o"$TEMP\magic_enum_src" -y'
  Pop $0
  
  CopyFiles "$TEMP\magic_enum_src\magic_enum-0.9.5\include\*" "C:\Program Files\magic_enum\include\"
  
  MagicEnumExists:
  DetailPrint "Magic Enum installed"
  Delete "$TEMP\magic_enum.zip"
  RMDir /r "$TEMP\magic_enum_src"
FunctionEnd

Function InstallLibXml2
  DetailPrint "Installing LibXml2..."
  
  IfFileExists "C:\Program Files\LibXml2\include\libxml\xmlversion.h" LibXml2Exists
  
  NSISdl::download_quiet \
    "https://download.gnome.org/sources/libxml2/2.15/libxml2-2.15.0.tar.xz" \
    "$TEMP\libxml2.tar.xz"
  Pop $0
  StrCmp $0 "success" +3
    MessageBox MB_OK|MB_ICONSTOP "Failed to download LibXml2"
    Abort
  
  CreateDirectory "C:\Program Files\LibXml2"
  
  ; Extract using available tools
  IfFileExists "$SYSDIR\tar.exe" UseTarLibXml2
  IfFileExists "$PROGRAMFILES64\7-Zip\7z.exe" Use7ZipLibXml2
  MessageBox MB_OK|MB_ICONSTOP "No extraction tool available for LibXml2"
  Abort
  
  UseTarLibXml2:
    nsExec::ExecToStack 'tar -xf "$TEMP\libxml2.tar.xz" -C "C:\Program Files\LibXml2" --strip-components=1'
    Goto LibXml2Check
    
  Use7ZipLibXml2:
    CreateDirectory "$TEMP\libxml2_temp"
    nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TEMP\libxml2.tar.xz" -o"$TEMP\libxml2_temp" -y'
    ${If} $0 == 0
      FindFirst $1 $2 "$TEMP\libxml2_temp\*.tar"
      ${If} $2 != ""
        nsExec::ExecToStack '"$PROGRAMFILES64\7-Zip\7z.exe" x "$TEMP\libxml2_temp\$2" -o"C:\Program Files\LibXml2" -y'
      ${EndIf}
      FindClose $1
    ${EndIf}
  
  LibXml2Check:
  IfFileExists "C:\Program Files\LibXml2\include\libxml\xmlversion.h" LibXml2Exists
    MessageBox MB_OK|MB_ICONSTOP "LibXml2 installation failed"
    Abort
  
  LibXml2Exists:
  DetailPrint "LibXml2 installed"
  Delete "$TEMP\libxml2.tar.xz"
  RMDir /r "$TEMP\libxml2_temp"
FunctionEnd

Function SetupEnvironmentVariables
  DetailPrint "Setting up environment variables..."
  
  ; Set LLVM_DIR for CMake (critical for find_package(LLVM))
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "LLVM_DIR" "C:\Program Files\LLVM_Libs\lib\cmake\llvm"
  
  ; Set MayaFlux root
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "MAYAFLUX_ROOT" "$INSTDIR"
  
  ; Set dependency roots for CMake
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "GLFW_ROOT" "C:\Program Files\GLFW"
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "FFMPEG_ROOT" "C:\Program Files\FFmpeg"
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "RTAUDIO_ROOT" "C:\Program Files\RtAudio"
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "EIGEN3_ROOT" "C:\Program Files\Eigen3"
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "MAGIC_ENUM_ROOT" "C:\Program Files\magic_enum"
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "LIBXML2_ROOT" "C:\Program Files\LibXml2"
  
  ; Set Vulkan SDK path
  ${If} $VulkanSdkPath != ""
    WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
      "VULKAN_SDK" "$VulkanSdkPath"
  ${EndIf}
  
  ; Set DIA SDK path if found
  ${If} $DiaSdkPath != ""
    WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
      "DIA_SDK_PATH" "$DiaSdkPath"
  ${EndIf}
  
  ; Add to PATH using EnVar plugin
  EnVar::SetHKCU
  EnVar::Check "PATH" "$INSTDIR\bin"
  Pop $0
  ${If} $0 != 0
    EnVar::AddValue "PATH" "$INSTDIR\bin"
  ${EndIf}
  
  ; Add common library paths to PATH
  EnVar::Check "PATH" "C:\Program Files\FFmpeg\bin"
  Pop $0
  ${If} $0 != 0
    EnVar::AddValue "PATH" "C:\Program Files\FFmpeg\bin"
  ${EndIf}
  
  EnVar::Check "PATH" "C:\Program Files\LLVM_Libs\bin"
  Pop $0
  ${If} $0 != 0
    EnVar::AddValue "PATH" "C:\Program Files\LLVM_Libs\bin"
  ${EndIf}
  
  ${If} $VulkanSdkPath != ""
    EnVar::Check "PATH" "$VulkanSdkPath\Bin"
    Pop $0
    ${If} $0 != 0
      EnVar::AddValue "PATH" "$VulkanSdkPath\Bin"
    ${EndIf}
  ${EndIf}
  
  ; Set CMAKE_PREFIX_PATH for CMake to find all dependencies
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "CMAKE_PREFIX_PATH"
  StrCpy $1 "C:\Program Files\LLVM_Libs;C:\Program Files\FFmpeg;C:\Program Files\GLFW;C:\Program Files\RtAudio;C:\Program Files\Eigen3;C:\Program Files\magic_enum;C:\Program Files\LibXml2"
  ${If} $0 != ""
    StrCpy $1 "$0;$1"
  ${EndIf}
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
    "CMAKE_PREFIX_PATH" "$1"
  
  ; Broadcast environment changes
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  
  DetailPrint "Environment variables configured"
FunctionEnd

Function GenerateVSProject
  DetailPrint "Generating Visual Studio project..."
  
  ; Create a simple CMakeLists.txt for the project
  FileOpen $0 "$INSTDIR\projects\CMakeLists.txt" w
  FileWrite $0 "cmake_minimum_required(VERSION 3.20)$\r$\n"
  FileWrite $0 "project(MayaFluxProject)$\r$\n$\r$\n"
  FileWrite $0 "set(CMAKE_CXX_STANDARD 17)$\r$\n"
  FileWrite $0 "set(CMAKE_CXX_STANDARD_REQUIRED ON)$\r$\n$\r$\n"
  FileWrite $0 "# Find dependencies$\r$\n"
  FileWrite $0 "find_package(LLVM REQUIRED)$\r$\n"
  FileWrite $0 "find_package(Vulkan REQUIRED)$\r$\n"
  FileWrite $0 "find_package(PkgConfig REQUIRED)$\r$\n"
  FileWrite $0 "pkg_check_modules(FFMPEG REQUIRED libavcodec libavformat libavutil)$\r$\n$\r$\n"
  FileWrite $0 "# Include directories$\r$\n"
  FileWrite $0 "include_directories(${LLVM_INCLUDE_DIRS})$\r$\n"
  FileWrite $0 "include_directories(${Vulkan_INCLUDE_DIR})$\r$\n"
  FileWrite $0 "include_directories(${FFMPEG_INCLUDE_DIRS})$\r$\n"
  FileWrite $0 "include_directories(C:/Program Files/GLFW/include)$\r$\n"
  FileWrite $0 "include_directories(C:/Program Files/RtAudio/include)$\r$\n"
  FileWrite $0 "include_directories(C:/Program Files/Eigen3)$\r$\n"
  FileWrite $0 "include_directories(C:/Program Files/magic_enum/include)$\r$\n"
  FileWrite $0 "include_directories(C:/Program Files/LibXml2/include)$\r$\n$\r$\n"
  FileWrite $0 "add_executable(mayaflux_app main.cpp)$\r$\n$\r$\n"
  FileWrite $0 "# Link libraries$\r$\n"
  FileWrite $0 "target_link_libraries(mayaflux_app $\r$\n"
  FileWrite $0 "    ${LLVM_LIBS}$\r$\n"
  FileWrite $0 "    ${Vulkan_LIBRARY}$\r$\n"
  FileWrite $0 "    ${FFMPEG_LIBRARIES}$\r$\n"
  FileWrite $0 "    C:/Program Files/GLFW/lib-vc2022/glfw3.lib$\r$\n"
  FileWrite $0 "    C:/Program Files/RtAudio/lib/rtaudio.lib$\r$\n"
  FileWrite $0 "    C:/Program Files/LibXml2/lib/libxml2.lib$\r$\n"
  FileWrite $0 ")$\r$\n"
  FileClose $0
  
  ; Create a simple project.cpp example
  ; FileClose $0
  
  ; Create build script
  FileOpen $0 "$INSTDIR\projects\build.bat" w
  FileWrite $0 "@echo off$\r$\n"
  FileWrite $0 "echo Building MayaFlux project...$\r$\n"
  FileWrite $0 "cmake -B build -G $\"Visual Studio 17 2022$\" -A x64$\r$\n"
  FileWrite $0 "cmake --build build --config Release$\r$\n"
  FileWrite $0 "echo Build complete!$\r$\n"
  FileWrite $0 "pause$\r$\n"
  FileClose $0
  
  DetailPrint "Visual Studio project template created in $INSTDIR\projects"
  MessageBox MB_OK "Visual Studio project template created in $\r$\n$INSTDIR\projects$\r$\n$\r$\nRun 'build.bat' to compile the example."
FunctionEnd

; Uninstaller
Section "Uninstall"
  ; Remove registry entries
  DeleteRegKey HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\MayaFlux"
  
  ; Remove environment variables
  DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "MAYAFLUX_ROOT"
  DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "LLVM_DIR"
  DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "GLFW_ROOT"
  DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "FFMPEG_ROOT"
  DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "RTAUDIO_ROOT"
  DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "EIGEN3_ROOT"
  DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "MAGIC_ENUM_ROOT"
  DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "LIBXML2_ROOT"
  
  ; Remove from PATH
  EnVar::SetHKCU
  EnVar::DeleteValue "PATH" "$INSTDIR\bin"
  EnVar::DeleteValue "PATH" "C:\Program Files\FFmpeg\bin"
  EnVar::DeleteValue "PATH" "C:\Program Files\LLVM_Libs\bin"
  
  ; Update CMAKE_PREFIX_PATH
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "CMAKE_PREFIX_PATH"
  ${WordReplace} $0 "C:\Program Files\LLVM_Libs;C:\Program Files\FFmpeg;C:\Program Files\GLFW;C:\Program Files\RtAudio;C:\Program Files\Eigen3;C:\Program Files\magic_enum;C:\Program Files\LibXml2" "" "+" $1
  WriteRegStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "CMAKE_PREFIX_PATH" "$1"
  
  ; Broadcast environment changes
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  
  ; Remove install directory
  RMDir /r "$INSTDIR"
  
  ; Note: We don't remove the dependencies as they might be used by other applications
  MessageBox MB_YESNO|MB_ICONQUESTION "Would you like to remove the installed dependencies (LLVM, FFmpeg, GLFW, etc.)?$\r$\n$\r$\nThis is not recommended if other applications might be using them." \
    IDNO NoDependencyRemove
    
  ; Remove dependencies (optional)
  RMDir /r "C:\Program Files\LLVM_Libs"
  RMDir /r "C:\Program Files\FFmpeg"
  RMDir /r "C:\Program Files\GLFW"
  RMDir /r "C:\Program Files\RtAudio"
  RMDir /r "C:\Program Files\Eigen3"
  RMDir /r "C:\Program Files\magic_enum"
  RMDir /r "C:\Program Files\LibXml2"
  
  NoDependencyRemove:
SectionEnd

; Section descriptions
LangString DESC_SectionCore ${LANG_ENGLISH} "Core MayaFlux files and documentation."
LangString DESC_SectionBuildTools ${LANG_ENGLISH} "Required build tools (CMake, Git, 7-Zip)."
LangString DESC_SectionLLVM ${LANG_ENGLISH} "LLVM compiler infrastructure (required for JIT compilation)."
LangString DESC_SectionVS ${LANG_ENGLISH} "Visual Studio dependencies and DIA SDK."
LangString DESC_SectionVulkan ${LANG_ENGLISH} "Vulkan SDK for graphics and compute."
LangString DESC_SectionFFmpeg ${LANG_ENGLISH} "FFmpeg for multimedia processing."
LangString DESC_SectionGLFW ${LANG_ENGLISH} "GLFW for window and input management."
LangString DESC_SectionRtAudio ${LANG_ENGLISH} "RtAudio for audio input/output."
LangString DESC_SectionEigen ${LANG_ENGLISH} "Eigen3 for linear algebra."
LangString DESC_SectionMagicEnum ${LANG_ENGLISH} "Magic Enum for enum reflection."
LangString DESC_SectionLibXml2 ${LANG_ENGLISH} "LibXml2 for XML parsing."
LangString DESC_SectionEnv ${LANG_ENGLISH} "Set up environment variables for development."
LangString DESC_SectionVSProject ${LANG_ENGLISH} "Generate a Visual Studio project template."

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionCore} $(DESC_SectionCore)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionBuildTools} $(DESC_SectionBuildTools)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionLLVM} $(DESC_SectionLLVM)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionVS} $(DESC_SectionVS)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionVulkan} $(DESC_SectionVulkan)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionFFmpeg} $(DESC_SectionFFmpeg)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionGLFW} $(DESC_SectionGLFW)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionRtAudio} $(DESC_SectionRtAudio)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionEigen} $(DESC_SectionEigen)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionMagicEnum} $(DESC_SectionMagicEnum)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionLibXml2} $(DESC_SectionLibXml2)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionEnv} $(DESC_SectionEnv)
  !insertmacro MUI_DESCRIPTION_TEXT ${SectionVSProject} $(DESC_SectionVSProject)
!insertmacro MUI_FUNCTION_DESCRIPTION_END
