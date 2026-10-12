# Capture a window with PrintWindow (works even when the window is
# fully occluded; does not steal focus or disturb the user).
param(
  [long]$Hwnd,
  [string]$OutPng
)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class PW2 {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
}
"@
$h = [IntPtr]$Hwnd
$r = New-Object PW2+RECT
[void][PW2]::GetWindowRect($h, [ref]$r)
$w = $r.Right - $r.Left; $hgt = $r.Bottom - $r.Top
if ($w -le 0 -or $hgt -le 0) { throw "bad rect" }
$bmp = New-Object System.Drawing.Bitmap($w, $hgt)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
[void][PW2]::PrintWindow($h, $dc, 2)   # PW_RENDERFULLCONTENT
$g.ReleaseHdc($dc)
$g.Dispose()
$bmp.Save($OutPng, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output ("CAPTURED ${w}x${hgt} -> $OutPng")
