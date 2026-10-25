# Turns QuickEdit off for the console window it runs in, so a click in a build window cannot freeze the
# build (owner, 2026-10-25: a click left play.bat's build frozen for 34 minutes until Esc). Only this
# window changes; it ends when the window closes. Run it from a .bat before cargo:
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools\build\no_quickedit.ps1
Add-Type -Namespace CrabBreeze -Name Console -MemberDefinition @'
[DllImport("kernel32.dll")] public static extern System.IntPtr GetStdHandle(int handle);
[DllImport("kernel32.dll")] public static extern bool GetConsoleMode(System.IntPtr handle, out uint mode);
[DllImport("kernel32.dll")] public static extern bool SetConsoleMode(System.IntPtr handle, uint mode);
'@
$stdin = [CrabBreeze.Console]::GetStdHandle(-10)
[uint32]$mode = 0
if ([CrabBreeze.Console]::GetConsoleMode($stdin, [ref]$mode)) {
    # ENABLE_QUICK_EDIT_MODE (0x40) off; ENABLE_EXTENDED_FLAGS (0x80) on, which makes the change stick.
    [void][CrabBreeze.Console]::SetConsoleMode($stdin, ($mode -band (-bnot 0x40)) -bor 0x80)
}
