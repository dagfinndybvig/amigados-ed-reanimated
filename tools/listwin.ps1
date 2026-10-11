param([string]$ProcName = "winuae64")
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices;
public class WE {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
$procs = Get-Process -Name $ProcName -ErrorAction SilentlyContinue
if (-not $procs) { Write-Output "NO PROCESS"; exit }
foreach ($p in $procs) {
  Write-Output ("PROCESS Id=" + $p.Id + " MainHwnd=" + $p.MainWindowHandle)
  $targetPid = $p.Id
  $found = @()
  $cb = [WE+EnumProc]{ param($h, $l)
    $wpid = 0
    [void][WE]::GetWindowThreadProcessId($h, [ref]$wpid)
    if ($wpid -eq $targetPid -and [WE]::IsWindowVisible($h)) {
      $sb = New-Object System.Text.StringBuilder 256
      [void][WE]::GetWindowText($h, $sb, 256)
      $r = New-Object WE+RECT
      [void][WE]::GetWindowRect($h, [ref]$r)
      $script:found += ("  HWND " + $h + " rect=" + $r.Left + "," + $r.Top + "," + $r.Right + "," + $r.Bottom + " :: " + $sb.ToString())
    }
    return $true
  }
  [void][WE]::EnumWindows($cb, [IntPtr]::Zero)
  $found
}
