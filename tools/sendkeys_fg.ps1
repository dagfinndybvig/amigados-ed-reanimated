param(
  [long]$Hwnd,
  [string]$Keys = ""
)
# Send keystrokes to a window by hwnd, robustly taking foreground first
# (AttachThreadInput) and VERIFYING focus before sending. Prevents the
# classic trap where SetForegroundWindow silently fails and SendKeys
# types into an unrelated window (see tools/README.md, gotcha 1).
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class SKFG {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
}
"@
$h = [IntPtr]$Hwnd
$ok = $false
for ($i = 0; $i -lt 5 -and -not $ok; $i++) {
  $fg = [SKFG]::GetForegroundWindow()
  $fgPid = 0
  $fgThread = [SKFG]::GetWindowThreadProcessId($fg, [ref]$fgPid)
  $tPid = 0
  $targetThread = [SKFG]::GetWindowThreadProcessId($h, [ref]$tPid)
  $myThread = [SKFG]::GetCurrentThreadId()
  [void][SKFG]::AttachThreadInput($myThread, $fgThread, $true)
  [void][SKFG]::AttachThreadInput($myThread, $targetThread, $true)
  [void][SKFG]::ShowWindow($h, 9)  # SW_RESTORE
  [void][SKFG]::SetForegroundWindow($h)
  [void][SKFG]::AttachThreadInput($myThread, $fgThread, $false)
  [void][SKFG]::AttachThreadInput($myThread, $targetThread, $false)
  Start-Sleep -Milliseconds 500
  if ([SKFG]::GetForegroundWindow() -eq $h) { $ok = $true }
}
if (-not $ok) { Write-Output "FOCUS FAILED"; exit 1 }
[System.Windows.Forms.SendKeys]::SendWait($Keys)
Write-Output ("SENT: " + $Keys)
