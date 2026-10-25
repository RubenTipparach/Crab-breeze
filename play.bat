@echo off
rem Play Crab Breeze on Windows: double-click. Rebuilds the release game only if the code changed,
rem then starts it in the Foreman. Arguments replace the default, e.g. play.bat --mech mechs/driller.ron
rem Esc opens the menu; Controls lists every key.
rem It builds into its own folder (target\play), so it never waits on another build of the game.
cd /d "%~dp0"
set CARGO_TARGET_DIR=%~dp0target\play
cargo build --release -p cb-app || (pause & exit /b 1)
if "%~1"=="" (
  "%CARGO_TARGET_DIR%\release\crab-breeze.exe" --mech mechs/foreman.ron
) else (
  "%CARGO_TARGET_DIR%\release\crab-breeze.exe" %*
)
if errorlevel 1 pause
