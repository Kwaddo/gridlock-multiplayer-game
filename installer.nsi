!include "MUI2.nsh"

Name "Gridlock"
OutFile "dist\Gridlock-Setup.exe"
InstallDir "$LOCALAPPDATA\Gridlock"
InstallDirRegKey HKCU "Software\Gridlock" "InstallDir"
RequestExecutionLevel user

!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Gridlock" SecMain
    SetOutPath "$INSTDIR"
    File "dist\windows\client.exe"
    File "dist\windows\server.exe"
    File /r "dist\windows\assets"

    CreateDirectory "$SMPROGRAMS\Gridlock"
    CreateShortcut "$SMPROGRAMS\Gridlock\Gridlock.lnk" "$INSTDIR\client.exe"

    WriteUninstaller "$INSTDIR\Uninstall.exe"

    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Gridlock" "DisplayName" "Gridlock"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Gridlock" "UninstallString" "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Gridlock" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Gridlock" "Publisher" "Kwaddo"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Gridlock" "NoModify" "1"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Gridlock" "NoRepair" "1"

    WriteRegStr HKCU "Software\Gridlock" "InstallDir" "$INSTDIR"
SectionEnd

Section "Uninstall"
    Delete "$INSTDIR\client.exe"
    Delete "$INSTDIR\server.exe"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir /r "$INSTDIR\assets"
    RMDir "$INSTDIR"

    Delete "$SMPROGRAMS\Gridlock\Gridlock.lnk"
    RMDir "$SMPROGRAMS\Gridlock"

    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Gridlock"
    DeleteRegKey HKCU "Software\Gridlock"
SectionEnd
