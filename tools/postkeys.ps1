param(
  [long]$Hwnd,
  [string]$Keys = ""
)
# Send keys via PostMessage WM_KEYDOWN/WM_CHAR/WM_KEYUP directly to a
# window, bypassing focus/activation entirely. Interpret a compact syntax:
# printable chars type through; special: {ESC} {ENTER} {F12} {TAB} {BS} {DEL}
# {UP} {DOWN} {LEFT} {RIGHT} {CTRL} prefixes: ^X = ctrl-X
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class PM {
  [DllImport("user32.dll", SetLastError = true)] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr wParam, IntPtr lParam);
}
"@
$h = [IntPtr]$Hwnd
$WM_KEYDOWN = 0x0100; $WM_KEYUP = 0x0101; $WM_CHAR = 0x0102

function Post-Key([int]$vk, [int]$scan, [int]$ch) {
  $lpDown = [IntPtr](1 -bor ($scan -shl 16) -bor (1 -shl 24) -bor (1 -shl 30))
  $lpUp   = [IntPtr](1 -bor ($scan -shl 16) -bor (1 -shl 24) -bor (1 -shl 30) -bor (1 -shl 31))
  [void][PM]::PostMessage($h, $WM_KEYDOWN, [IntPtr]$vk, $lpDown)
  if ($ch -ne 0) { [void][PM]::PostMessage($h, $WM_CHAR, [IntPtr]$ch, $lpDown) }
  [void][PM]::PostMessage($h, $WM_KEYUP, [IntPtr]$vk, $lpUp)
  Start-Sleep -Milliseconds 30
}

$i = 0
while ($i -lt $Keys.Length) {
  $c = $Keys[$i]
  if ($c -eq '^' -and $i + 1 -lt $Keys.Length) {
    # ctrl-X: VK = letter code; also set the control bit via WM_CHAR control code
    $x = [char]::ToUpper($Keys[$i+1])
    $vk = [int][char]$x
    $scan = [int]([uintptr]$vk)
    Post-Key $vk $scan 0
    # control char: letter & 0x1F
    [void][PM]::PostMessage($h, $WM_KEYDOWN, [IntPtr]17, [IntPtr](1 -bor (0x1D -shl 16)))
    [void][PM]::PostMessage($h, $WM_CHAR, [IntPtr]($vk -band 0x1F), [IntPtr](1 -bor (([int][char]$x) -shl 16)))
    [void][PM]::PostMessage($h, $WM_KEYUP, [IntPtr]17, [IntPtr](1 -bor (0x1D -shl 16) -bor (1 -shl 31)))
    $i += 2
    continue
  }
  if ($c -eq '{') {
    $j = $Keys.IndexOf('}', $i)
    if ($j -gt $i) {
      $name = $Keys.Substring($i+1, $j-$i-1).ToUpper()
      $vk = switch ($name) {
        'ESC'   { 0x1B } 'ENTER' { 0x0D } 'TAB' { 0x09 } 'BS' { 0x08 }
        'DEL'   { 0x2E } 'F12' { 0x7B } 'UP' { 0x26 } 'DOWN' { 0x28 }
        'LEFT'  { 0x25 } 'RIGHT' { 0x27 } 'SPACE' { 0x20 }
        default { 0 }
      }
      $ch = switch ($name) {
        'ENTER' { 0x0D } 'ESC' { 0x1B } 'TAB' { 0x09 } 'BS' { 0x08 } 'SPACE' { 0x20 }
        default { 0 }
      }
      if ($vk -ne 0) { Post-Key $vk ($vk -band 0xFF) $ch }
      $i = $j + 1
      continue
    }
  }
  $vk = [int][char]$c
  Post-Key $vk ($vk -band 0xFF) $vk
  $i++
}
Write-Output ("POSTED: " + $Keys)
