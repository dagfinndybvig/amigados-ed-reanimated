# Send the WinUAE Amiga-reset combo (Ctrl + LeftWin + RightWin held
# together) to the foreground window, then release. Visible effect:
# the emulated Amiga reboots (boot banner appears).
param([string]$ProcName = "WinUAE")
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class RK2 {
  [StructLayout(LayoutKind.Sequential)] public struct INPUT {
    public uint type; public KEYBDINPUT ki; }
  [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT {
    public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr extra; }
  public const uint KEYDOWN = 0, KEYUP = 2;
  [DllImport("user32.dll", SetLastError = true)] public static extern uint SendInput(uint n, INPUT[] inputs, int size);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
}
"@
function Send-Key([UInt16]$vk, [UInt32]$flags) {
  $inp = New-Object RK2+INPUT
  $inp.type = 1
  $inp.ki.wVk = $vk
  $inp.ki.dwFlags = $flags
  [void][RK2]::SendInput(1, @($inp), [System.Runtime.InteropServices.Marshal]::SizeOf([type][RK2+INPUT]))
}
$p = Get-Process -Name $ProcName -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
[void][RK2]::SetForegroundWindow($p.MainWindowHandle)
Start-Sleep -Milliseconds 500
$VK_CONTROL = 0x11; $VK_LWIN = 0x5B; $VK_RWIN = 0x5C
Send-Key $VK_CONTROL ([RK2]::KEYDOWN)
Start-Sleep -Milliseconds 120
Send-Key $VK_LWIN    ([RK2]::KEYDOWN)
Start-Sleep -Milliseconds 120
Send-Key $VK_RWIN    ([RK2]::KEYDOWN)
Start-Sleep -Milliseconds 400
Send-Key $VK_RWIN    ([RK2]::KEYUP)
Send-Key $VK_LWIN    ([RK2]::KEYUP)
Send-Key $VK_CONTROL ([RK2]::KEYUP)
Write-Output "RESET COMBO SENT"
