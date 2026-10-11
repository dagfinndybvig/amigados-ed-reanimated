param([long]$Hwnd)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class RW {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int w, int hgt, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
}
"@
$h = [IntPtr]$Hwnd
# raise via topmost toggle (bypasses foreground lock), then un-topmost
[void][RW]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x0003)   # TOPMOST + NOSIZE|NOMOVE
[void][RW]::SetWindowPos($h, [IntPtr](-2), 0, 0, 0, 0, 0x0003)   # NOTOPMOST
Write-Output "RAISED $Hwnd"
