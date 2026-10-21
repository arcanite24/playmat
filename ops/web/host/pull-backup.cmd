@echo off
rem Fetches the newest Playmat backup pack from a server over SSH and keeps the newest 30 here. Run daily by a
rem scheduled task on the machine that holds the off-site copies (Multivac). The SSH alias must use a key that the
rem server restricts to "playmat-offsite export" (see ops/web/README.md, "Off-site backups").
rem
rem   pull-backup.cmd [ALIAS] [DIR]      defaults: playmat-backup  %USERPROFILE%\playmat-backups
setlocal
set ALIAS=%~1
if "%ALIAS%"=="" set ALIAS=playmat-backup
set DIR=%~2
if "%DIR%"=="" set DIR=%USERPROFILE%\playmat-backups
if not exist "%DIR%" mkdir "%DIR%"
for /f %%t in ('powershell -NoProfile -Command "(Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')"') do set STAMP=%%t
rem cmd redirection keeps the bytes intact (PowerShell 5.1 would re-encode them)
ssh -o BatchMode=yes %ALIAS% > "%DIR%\playmat-%STAMP%.tar.gz.part"
if errorlevel 1 (
  del "%DIR%\playmat-%STAMP%.tar.gz.part" 2>nul
  echo pull failed
  exit /b 1
)
move /y "%DIR%\playmat-%STAMP%.tar.gz.part" "%DIR%\playmat-%STAMP%.tar.gz" >nul
rem keep the newest 30
for /f "skip=30 delims=" %%f in ('dir /b /o-n "%DIR%\playmat-*.tar.gz"') do del "%DIR%\%%f"
echo pulled playmat-%STAMP%.tar.gz
