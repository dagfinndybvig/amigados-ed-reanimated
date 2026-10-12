# Find all top-level windows whose rect covers a point (default: WinUAE
# client center) - hunting for invisible overlays that eat input.
param([int]$X = 700, [int]$Y = 400)
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices;
public class OV {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int i);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
$results = @()
$cb = [OV+EnumProc]{
  param($h, $l)
  $r = New-Object OV+RECT
  [void][OV]::GetWindowRect($h, [ref]$r)
  if ($r.Left -le $X -and $r.Right -ge $X -and $r.Top -le $Y -and $r.Bottom -ge $Y) {
    $wpid = 0
    [void][OV]::GetWindowThreadProcessId($h, [ref]$wpid)
    $sb = New-Object System.Text.StringBuilder 256
    [void][OV]::GetWindowText($h, $sb, 256)
    $ex = [OV]::GetWindowLong($h, -20)
    $script:results += ("hwnd=" + $h + " pid=" + $wpid + " visible=" + [OV]::IsWindowVisible($h) + " exstyle=0x" + $ex.ToString("X") + " rect=" + $r.Left + "," + $r.Top + "," + $r.Right + "," + $r.Bottom + " [" + $sb.ToString() + "]")
  }
  return $true
}
[void][OV]::EnumWindows($cb, [IntPtr]::Zero)
$results
