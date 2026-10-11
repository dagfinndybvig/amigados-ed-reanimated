param([int]$TimeoutSec = 40)
# Watch for a "WinUAE message" dialog from winuae64 and screenshot+OCR it
# the moment it appears.
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices;
public class WC {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
$deadline = (Get-Date).AddSeconds($TimeoutSec)
while ((Get-Date) -lt $deadline) {
  $procs = Get-Process -Name winuae64 -ErrorAction SilentlyContinue
  foreach ($p in $procs) {
    $targetPid = $p.Id
    $hits = @()
    $cb = [WC+EnumProc]{ param($h, $l)
      $wpid = 0
      [void][WC]::GetWindowThreadProcessId($h, [ref]$wpid)
      if ($wpid -eq $targetPid -and [WC]::IsWindowVisible($h)) {
        $sb = New-Object System.Text.StringBuilder 256
        [void][WC]::GetWindowText($h, $sb, 256)
        if ($sb.ToString() -like "*message*") { $script:hits += $h }
      }
      return $true
    }
    $script:hits = @()
    [void][WC]::EnumWindows($cb, [IntPtr]::Zero)
    if ($hits.Count -gt 0) {
      $h = $hits[0]
      $r = New-Object WC+RECT
      [void][WC]::GetWindowRect($h, [ref]$r)
      $w = $r.Right - $r.Left; $hgt = $r.Bottom - $r.Top
      if ($w -gt 0 -and $hgt -gt 0) {
        $bmp = New-Object System.Drawing.Bitmap($w, $hgt)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $hgt)))
        $out = "C:\Users\dybvig\Reverse\Ed\emulator\dialog.png"
        $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
        $g.Dispose(); $bmp.Dispose()
        Write-Output ("CAPTURED ${w}x${hgt} -> $out")
        exit 0
      }
    }
  }
  Start-Sleep -Milliseconds 250
}
Write-Output "TIMEOUT: no message dialog seen"
exit 1
