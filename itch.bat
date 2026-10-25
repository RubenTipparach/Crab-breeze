@echo off
rem Package Crab Breeze for itch.io: double-click. Builds the release game as play.bat does, with the
rem `shipping` feature (no on-screen performance readout or frame graph), into its own folder
rem (target\itch) so it never disturbs play.bat's build. Then lays the game out as a player runs it
rem (crab-breeze.exe beside data and assets: the game finds them from the folder it starts in) in
rem dist\crab-breeze, and zips that to dist\crab-breeze-windows.zip for upload.
cd /d "%~dp0"
set CARGO_TARGET_DIR=%~dp0target\itch
rem No QuickEdit in this window (a click would freeze the build), and a watchdog that beeps if the
rem build stalls anyway (tools\build).
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\build\no_quickedit.ps1"
start "" /b powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\build\watch_build.ps1"
cargo build --release -p cb-app --features shipping || (pause & exit /b 1)
set OUT=%~dp0dist\crab-breeze
if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%" || (pause & exit /b 1)
copy /y "%CARGO_TARGET_DIR%\release\crab-breeze.exe" "%OUT%\" >nul || (pause & exit /b 1)
robocopy "%~dp0data" "%OUT%\data" /e /njh /njs /nfl /ndl >nul
if errorlevel 8 (echo copying data failed & pause & exit /b 1)
robocopy "%~dp0assets" "%OUT%\assets" /e /njh /njs /nfl /ndl >nul
if errorlevel 8 (echo copying assets failed & pause & exit /b 1)
if exist "%~dp0dist\crab-breeze-windows.zip" del /q "%~dp0dist\crab-breeze-windows.zip"
powershell -NoProfile -Command "Compress-Archive -Path '%OUT%\*' -DestinationPath '%~dp0dist\crab-breeze-windows.zip'" || (pause & exit /b 1)
echo.
echo Ready for itch: dist\crab-breeze-windows.zip (the folder it holds is dist\crab-breeze)
pause
