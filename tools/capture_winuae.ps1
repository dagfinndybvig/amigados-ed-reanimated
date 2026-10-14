# Capture the WinUAE emulation window (largest visible window of the
# winuae64 process) with PrintWindow; works while occluded, no focus steal.
param([string]$OutPng)

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class CW1 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

$procs = Get-Process -Name winuae64 -ErrorAction SilentlyContinue
if (-not $procs) { Write-Output "NO PROCESS"; exit 1 }
$pids = @($procs | ForEach-Object { $_.Id })
$best = [IntPtr]::Zero
$bestArea = 0
$cb = [CW1+EnumProc]{ param($h, $l)
  $wpid = 0
  [void][CW1]::GetWindowThreadProcessId($h, [ref]$wpid)
  if ($pids -contains $wpid -and [CW1]::IsWindowVisible($h)) {
    $r = New-Object CW1+RECT
    [void][CW1]::GetWindowRect($h, [ref]$r)
    $area = ($r.Right - $r.Left) * ($r.Bottom - $r.Top)
    if ($area -gt $bestArea) { $script:bestArea = $area; $script:best = $h }
  }
  return $true
}
[void][CW1]::EnumWindows($cb, [IntPtr]::Zero)
if ($best -eq [IntPtr]::Zero -or $bestArea -le 0) { Write-Output "NO WINDOW"; exit 1 }

$r = New-Object CW1+RECT
[void][CW1]::GetWindowRect($best, [ref]$r)
$w = $r.Right - $r.Left; $hgt = $r.Bottom - $r.Top
$bmp = New-Object System.Drawing.Bitmap($w, $hgt)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
[void][CW1]::PrintWindow($best, $dc, 2)
$g.ReleaseHdc($dc)
$g.Dispose()
$bmp.Save($OutPng, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output ("CAPTURED hwnd={0} {1}x{2} -> {3}" -f $best, $w, $hgt, $OutPng)
