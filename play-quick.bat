@echo off
rem Play Crab Breeze, rebuilt fast: the same as play.bat, but built with the `play` profile (no
rem whole-program optimisation, incremental), so a rebuild after a change is much quicker. The game
rem runs somewhat slower than with play.bat; measure performance with play.bat or the perf suite.
rem It builds into its own folder (target\play-quick), so it never waits on another build.
cd /d "%~dp0"
set CARGO_TARGET_DIR=%~dp0target\play-quick
cargo build --profile play -p cb-app || (pause & exit /b 1)
if "%~1"=="" (
  "%CARGO_TARGET_DIR%\play\crab-breeze.exe" --mech mechs/foreman.ron
) else (
  "%CARGO_TARGET_DIR%\play\crab-breeze.exe" %*
)
if errorlevel 1 pause
