param([long]$Hwnd, [int]$X, [int]$Y, [int]$W, [int]$H)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class RS {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int w, int hgt, uint flags);
}
"@
[void][RS]::SetWindowPos([IntPtr]$Hwnd, [IntPtr]::Zero, $X, $Y, $W, $H, 0x0004)  # SWP_NOZORDER
Write-Output "RESIZED to ${W}x${H} at ${X},${Y}"
