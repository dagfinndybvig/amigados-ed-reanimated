# Inspect the GUI thread state of a window: which window holds keyboard
# focus, capture, caret inside the target thread.
param([string]$ProcName = "winuae64")
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class GTI {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct GUITHREADINFO {
    public int cbSize; public int flags;
    public IntPtr hwndActive; public IntPtr hwndFocus; public IntPtr hwndCapture;
    public IntPtr hwndMenuOwner; public IntPtr hwndMoveSize; public IntPtr hwndCaret;
    public RECT rcCaret;
  }
  [DllImport("user32.dll")] public static extern bool GetGUIThreadInfo(uint tid, ref GUITHREADINFO info);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
"@
$p = Get-Process -Name $ProcName -ErrorAction Stop
$h = $p.MainWindowHandle
$fg = [GTI]::GetForegroundWindow()
Write-Output ("foreground: " + $fg + "  target: " + $h + "  match: " + ($fg -eq $h))
$wpid = 0
$tid = [GTI]::GetWindowThreadProcessId($h, [ref]$wpid)
$g = New-Object GTI+GUITHREADINFO
$g.cbSize = [System.Runtime.InteropServices.Marshal]::SizeOf($g)
[void][GTI]::GetGUIThreadInfo($tid, [ref]$g)
foreach ($f in @('hwndActive','hwndFocus','hwndCapture','hwndCaret')) {
  $wh = $g.$f
  if ($wh -ne [IntPtr]::Zero) {
    $cn = New-Object System.Text.StringBuilder 256
    [void][GTI]::GetClassName($wh, $cn, 256)
    $tt = New-Object System.Text.StringBuilder 256
    [void][GTI]::GetWindowText($wh, $tt, 256)
    Write-Output ($f + ": " + $wh + " class=[" + $cn.ToString() + "] title=[" + $tt.ToString() + "]")
  } else {
    Write-Output ($f + ": ZERO")
  }
}
