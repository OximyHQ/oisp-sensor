; OISP Sensor Windows Installer
; NSIS 3.x Script
; https://nsis.sourceforge.io/

;--------------------------------
; Includes

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

;--------------------------------
; General

; Installer name and output file
Name "OISP Sensor"
OutFile "oisp-sensor-setup.exe"

; Default installation directory
InstallDir "$PROGRAMFILES64\OISP Sensor"

; Request admin privileges for installation
RequestExecutionLevel admin

; Version information (can be overridden with /DVERSION=x.y.z)
!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!define PUBLISHER "OISP"
!define WEBSITE "https://github.com/your-org/oisp-sensor"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "OISP Sensor"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "FileDescription" "OISP Sensor Installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2024 ${PUBLISHER}"

; Registry key to store uninstall information
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\OISPSensor"

;--------------------------------
; Interface Settings

!define MUI_ABORTWARNING
!define MUI_ICON "resources\oisp-icon.ico"
!define MUI_UNICON "resources\oisp-icon.ico"
!define MUI_WELCOMEFINISHPAGE_BITMAP "resources\welcome.bmp"

; Welcome page text
!define MUI_WELCOMEPAGE_TITLE "Welcome to OISP Sensor Setup"
!define MUI_WELCOMEPAGE_TEXT "This wizard will install OISP Sensor ${VERSION} on your computer.$\r$\n$\r$\nOISP Sensor captures and logs AI API traffic for monitoring and debugging.$\r$\n$\r$\nClick Next to continue."

; Finish page
!define MUI_FINISHPAGE_RUN "$INSTDIR\OISPApp.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch OISP Sensor"
!define MUI_FINISHPAGE_LINK "Visit OISP Sensor website"
!define MUI_FINISHPAGE_LINK_LOCATION "${WEBSITE}"

;--------------------------------
; Pages

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "resources\LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

;--------------------------------
; Languages

!insertmacro MUI_LANGUAGE "English"

;--------------------------------
; Installer Sections

Section "OISP Sensor" SecMain
    SectionIn RO ; Read-only (required)

    ; Set output path
    SetOutPath "$INSTDIR"

    ; Install main application files
    File "..\..\target\release\oisp-sensor.exe"
    File "..\..\target\release\oisp-redirector.exe"

    ; Install .NET application
    SetOutPath "$INSTDIR"
    File /r "..\OISPApp\bin\Release\net8.0-windows\*.*"

    ; Install WinDivert
    SetOutPath "$INSTDIR"
    File "..\deps\WinDivert-2.2.2-A\x64\WinDivert.dll"
    File "..\deps\WinDivert-2.2.2-A\x64\WinDivert64.sys"

    ; Install documentation
    File "resources\README.txt"

    ; Create start menu shortcuts
    CreateDirectory "$SMPROGRAMS\OISP Sensor"
    CreateShortCut "$SMPROGRAMS\OISP Sensor\OISP Sensor.lnk" "$INSTDIR\OISPApp.exe" "" "$INSTDIR\OISPApp.exe" 0
    CreateShortCut "$SMPROGRAMS\OISP Sensor\Uninstall.lnk" "$INSTDIR\uninstall.exe"

    ; Create desktop shortcut (optional)
    CreateShortCut "$DESKTOP\OISP Sensor.lnk" "$INSTDIR\OISPApp.exe" "" "$INSTDIR\OISPApp.exe" 0

    ; Write uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Write registry keys for Add/Remove Programs
    WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "OISP Sensor"
    WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
    WriteRegStr HKLM "${UNINST_KEY}" "QuietUninstallString" "$\"$INSTDIR\uninstall.exe$\" /S"
    WriteRegStr HKLM "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKLM "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\OISPApp.exe"
    WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
    WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
    WriteRegStr HKLM "${UNINST_KEY}" "URLInfoAbout" "${WEBSITE}"
    WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
    WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1

    ; Calculate installed size
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD HKLM "${UNINST_KEY}" "EstimatedSize" $0

SectionEnd

Section "Install CA Certificate" SecCA
    ; This section is optional - installs CA certificate to trusted root
    ; Only runs if user has already generated a CA certificate

    IfFileExists "$LOCALAPPDATA\OISP\oisp-ca.crt" 0 skip_ca
        MessageBox MB_YESNO "Do you want to install the OISP CA certificate?$\r$\n$\r$\nThis is required for HTTPS interception." IDNO skip_ca
        nsExec::ExecToLog 'certutil.exe -user -addstore Root "$LOCALAPPDATA\OISP\oisp-ca.crt"'
        Pop $0
        ${If} $0 != 0
            MessageBox MB_OK "CA certificate installation requires manual action.$\r$\nYou can install it later from the tray app."
        ${EndIf}
    skip_ca:

SectionEnd

;--------------------------------
; Descriptions

LangString DESC_SecMain ${LANG_ENGLISH} "Install OISP Sensor application files."
LangString DESC_SecCA ${LANG_ENGLISH} "Install CA certificate for HTTPS interception (optional)."

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecMain} $(DESC_SecMain)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCA} $(DESC_SecCA)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

;--------------------------------
; Uninstaller Section

Section "Uninstall"

    ; Kill running processes
    nsExec::ExecToLog 'taskkill /F /IM OISPApp.exe'
    nsExec::ExecToLog 'taskkill /F /IM oisp-sensor.exe'
    nsExec::ExecToLog 'taskkill /F /IM oisp-redirector.exe'

    ; Wait for processes to exit
    Sleep 1000

    ; Remove files
    Delete "$INSTDIR\oisp-sensor.exe"
    Delete "$INSTDIR\oisp-redirector.exe"
    Delete "$INSTDIR\OISPApp.exe"
    Delete "$INSTDIR\OISPApp.dll"
    Delete "$INSTDIR\OISPApp.deps.json"
    Delete "$INSTDIR\OISPApp.runtimeconfig.json"
    Delete "$INSTDIR\*.dll"
    Delete "$INSTDIR\WinDivert.dll"
    Delete "$INSTDIR\WinDivert64.sys"
    Delete "$INSTDIR\README.txt"
    Delete "$INSTDIR\uninstall.exe"

    ; Remove shortcuts
    Delete "$SMPROGRAMS\OISP Sensor\OISP Sensor.lnk"
    Delete "$SMPROGRAMS\OISP Sensor\Uninstall.lnk"
    RMDir "$SMPROGRAMS\OISP Sensor"
    Delete "$DESKTOP\OISP Sensor.lnk"

    ; Remove installation directory
    RMDir "$INSTDIR"

    ; Remove registry keys
    DeleteRegKey HKLM "${UNINST_KEY}"

    ; Optionally remove CA certificate
    MessageBox MB_YESNO "Remove OISP CA certificate from trusted store?" IDNO skip_remove_ca
        nsExec::ExecToLog 'certutil.exe -user -delstore Root "OISP Sensor CA"'
    skip_remove_ca:

    ; Note: We don't remove user data at %LOCALAPPDATA%\OISP
    ; User may want to keep settings and events

SectionEnd

;--------------------------------
; Functions

Function .onInit
    ; Check for 64-bit Windows
    ${IfNot} ${RunningX64}
        MessageBox MB_OK|MB_ICONSTOP "OISP Sensor requires 64-bit Windows."
        Abort
    ${EndIf}

    ; Check Windows version (Windows 10+)
    ${If} ${AtLeastWin10}
        ; OK
    ${Else}
        MessageBox MB_OK|MB_ICONEXCLAMATION "OISP Sensor is designed for Windows 10/11. Installation may proceed but some features may not work."
    ${EndIf}

    ; Check for .NET 8 runtime
    ; Note: Self-contained apps don't need this check
    ; nsExec::ExecToStack 'dotnet --list-runtimes'
    ; Pop $0
    ; ${If} $0 != 0
    ;     MessageBox MB_YESNO ".NET 8 Runtime not detected. Continue anyway?" IDYES continue
    ;     Abort
    ; ${EndIf}
    ; continue:
FunctionEnd

Function un.onInit
    ; Confirm uninstallation
    MessageBox MB_YESNO "Are you sure you want to uninstall OISP Sensor?" IDYES +2
    Abort
FunctionEnd
