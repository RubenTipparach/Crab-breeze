# Watches a cargo build started by play.bat, play-quick.bat or itch.bat and warns when it stalls
# (owner, 2026-10-25: "somethings seriously wrong with the build, it has taken more than 30 mins",
# "can you have something detect that in thef utrue?"). That build sat 34 minutes with cargo using
# 2 s of CPU and no compiler running: the console was in a QuickEdit selection, which freezes any
# program that writes to it until Esc. A build is stalled when cargo is running, no rustc or linker is,
# and cargo's own CPU time has not moved for $StallS seconds. Then it beeps and says what to do, once
# per stall. It exits when cargo does.
#
# Usage (from a .bat, before cargo): start "" /b powershell -NoProfile -ExecutionPolicy Bypass -File tools\build\watch_build.ps1
param([int]$StallS = 60, [int]$StartupS = 120)

$started = Get-Date
$lastCpu = -1.0
$still = 0
$warned = $false
while ($true) {
    Start-Sleep -Seconds 5
    $cargo = Get-Process cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        if (((Get-Date) - $started).TotalSeconds -gt $StartupS) { break }
        continue
    }
    $working = Get-Process rustc, link, rust-lld -ErrorAction SilentlyContinue
    $cpu = [double]($cargo | ForEach-Object { $_.TotalProcessorTime.TotalSeconds } | Measure-Object -Sum).Sum
    if ($working -or [math]::Abs($cpu - $lastCpu) -gt 0.05) {
        $still = 0
        $warned = $false
    } else {
        $still += 5
    }
    $lastCpu = $cpu
    if ($still -ge $StallS -and -not $warned) {
        [console]::beep(880, 300); [console]::beep(660, 300)
        Write-Host ""
        Write-Host "BUILD STALLED: nothing has compiled for $still s."
        Write-Host "  If the window title says 'Select', press Esc (a click froze it)."
        Write-Host "  Otherwise press Ctrl+C and run the .bat again (nothing compiled is lost)."
        $warned = $true
    }
}
