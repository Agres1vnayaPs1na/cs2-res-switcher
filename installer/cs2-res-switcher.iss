; Inno Setup script for cs2-res-switcher.
; Build: powershell -File build-installer.ps1   (needs Inno Setup 6)

#define AppName "cs2-res-switcher"
#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif
#define AppExe "cs2-res-switcher.exe"

[Setup]
AppId={{7C3E2B1A-5D4F-4C8E-9A61-2F0B7D3C9E15}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename={#AppName}-setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#AppExe}
; Same named mutex the app creates: setup asks the user to exit it first,
; so the desktop resolution is always restored properly.
AppMutex=Local\cs2-res-switcher

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "ukrainian"; MessagesFile: "compiler:Languages\Ukrainian.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[CustomMessages]
english.AutoStart=Start with Windows
ukrainian.AutoStart=Запускати разом з Windows
russian.AutoStart=Запускать вместе с Windows

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart"; Description: "{cm:AutoStart}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\dist\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"
Name: "{group}\config.toml"; Filename: "{app}\config.toml"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"
Name: "{userdesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"; Tasks: desktopicon
Name: "{userstartup}\{#AppName}"; Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"; Tasks: autostart

[Run]
; Store the language picked in setup as the app's UI language.
Filename: "{app}\{#AppExe}"; Parameters: "--set-language {code:AppLang} --exit"; WorkingDir: "{app}"; Flags: runhidden waituntilterminated
Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{app}\config.toml"
Type: files; Name: "{app}\cs2-res-switcher.log"

[Code]
function AppLang(Param: String): String;
begin
  if ActiveLanguage = 'ukrainian' then
    Result := 'uk'
  else
    Result := 'en';
end;
