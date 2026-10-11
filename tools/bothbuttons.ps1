param(
  [string]$ProcName = "winuae64",
  [int]$HoldMs = 1200,
  [int]$DelayMs = 0
)
# Press both Amiga mouse buttons (left+right of the emulator window) at the
# window center - used to open the Kickstart early-startup menu at boot.
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class M {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public MOUSEINPUT mi; }
  [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr extra; }
  public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004, RIGHTDOWN = 0x0008, RIGHTUP = 0x0010;
  [DllImport("user32.dll", SetLastError = true)] public static extern uint SendInput(uint n, INPUT[] inputs, int size);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
}
"@
$p = Get-Process -Name $ProcName -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
[void][M]::SetForegroundWindow($p.MainWindowHandle)
$r = New-Object M+RECT
[void][M]::GetWindowRect($p.MainWindowHandle, [ref]$r)
$cx = [int](($r.Left + $r.Right) / 2); $cy = [int](($r.Top + $r.Bottom) / 2)
[void][M]::SetCursorPos($cx, $cy)
if ($DelayMs -gt 0) { Start-Sleep -Milliseconds $DelayMs }

function Send-Mouse($flags) {
  $inp = New-Object M+INPUT
  $inp.type = 0  # INPUT_MOUSE
  $inp.mi.dwFlags = $flags
  [void][M]::SendInput(1, @($inp), [System.Runtime.InteropServices.Marshal]::SizeOf([type][M+INPUT]))
}

Send-Mouse ([M]::LEFTDOWN + [M]::RIGHTDOWN)
Start-Sleep -Milliseconds $HoldMs
Send-Mouse ([M]::LEFTUP + [M]::RIGHTUP)
Write-Output "BOTH_BUTTONS_PRESSED at ${cx},${cy} held ${HoldMs}ms"
