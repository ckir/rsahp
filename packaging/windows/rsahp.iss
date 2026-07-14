; rsahp Windows installer (per-user, no admin). Version injected via /DMyAppVersion.
#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#define MyAppName "rsahp"
#define MyAppExeName "rsahp-desktop.exe"

[Setup]
AppId={{A9F3C2E1-7B4D-4E8A-9C1F-000000000001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
DefaultDirName={localappdata}\Programs\rsahp
DefaultGroupName=rsahp
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\..\dist
OutputBaseFilename=rsahp-setup-{#MyAppVersion}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "rsahp.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\rsahp"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\rsahp.ico"
Name: "{userdesktop}\rsahp"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\rsahp.ico"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch rsahp"; Flags: nowait postinstall skipifsilent
