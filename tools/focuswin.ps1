param([long]$Hwnd)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class FW {
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
}
"@
$h = [IntPtr]$Hwnd
$before = [FW]::GetForegroundWindow()
$sb = New-Object System.Text.StringBuilder 256
[void][FW]::GetWindowText($before, $sb, 256)
Write-Output ("foreground before: " + $sb.ToString())

$targetPid = 0
$fgPid = 0
$targetThread = [FW]::GetWindowThreadProcessId($h, [ref]$targetPid)
$myThread = [FW]::GetCurrentThreadId()
$fgThread = [FW]::GetWindowThreadProcessId($before, [ref]$fgPid)

[void][FW]::AttachThreadInput($myThread, $fgThread, $true)
[void][FW]::AttachThreadInput($myThread, $targetThread, $true)
[void][FW]::ShowWindow($h, 9)          # SW_RESTORE
[void][FW]::BringWindowToTop($h)
[void][FW]::SetForegroundWindow($h)
[void][FW]::AttachThreadInput($myThread, $fgThread, $false)
[void][FW]::AttachThreadInput($myThread, $targetThread, $false)
Start-Sleep -Milliseconds 400
$after = [FW]::GetForegroundWindow()
$sb2 = New-Object System.Text.StringBuilder 256
[void][FW]::GetWindowText($after, $sb2, 256)
Write-Output ("foreground after: " + $sb2.ToString())
Write-Output ("is target: " + ($after -eq $h))
