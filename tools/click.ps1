param(
  [int]$X, [int]$Y,
  [switch]$Double,
  [switch]$Middle,
  [string]$ProcName = "winuae64"
)
# Click (or double-click) host-screen coordinates, after focusing the
# emulator window. Host coordinates map proportionally to the Amiga mouse.
# -Middle sends a middle-button click (WinUAE: untrap mouse).
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class CK {
  [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public MOUSEINPUT mi; }
  [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr extra; }
  public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004, MIDDLEDOWN = 0x0020, MIDDLEUP = 0x0040;
  [DllImport("user32.dll", SetLastError = true)] public static extern uint SendInput(uint n, INPUT[] inputs, int size);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
}
"@
function Send-Mouse($flags) {
  $inp = New-Object CK+INPUT
  $inp.type = 0
  $inp.mi.dwFlags = $flags
  [void][CK]::SendInput(1, @($inp), [System.Runtime.InteropServices.Marshal]::SizeOf([type][CK+INPUT]))
}

if ($Middle) {
  $mp = Get-Process -Name $ProcName -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  [void][CK]::SetForegroundWindow($mp.MainWindowHandle)
  Start-Sleep -Milliseconds 250
  Send-Mouse ([CK]::MIDDLEDOWN); Start-Sleep -Milliseconds 50; Send-Mouse ([CK]::MIDDLEUP)
  Write-Output "MIDDLE CLICK"
  exit 0
}

$p = Get-Process -Name $ProcName -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
[void][CK]::SetForegroundWindow($p.MainWindowHandle)
Start-Sleep -Milliseconds 250
[void][CK]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 200

Send-Mouse ([CK]::LEFTDOWN); Send-Mouse ([CK]::LEFTUP)
if ($Double) {
  Start-Sleep -Milliseconds 80
  Send-Mouse ([CK]::LEFTDOWN); Send-Mouse ([CK]::LEFTUP)
}
Write-Output ("CLICKED ${X},${Y}" + $(if ($Double) { " (double)" }))
