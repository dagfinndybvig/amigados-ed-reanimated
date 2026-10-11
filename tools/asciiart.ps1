param(
  [string]$Png = "C:\Users\dybvig\Reverse\Ed\emulator\shot2.png",
  [int]$Cols = 110,
  [int]$Rows = 40
)
# Render a PNG as coarse ASCII art (luminance ramp) for text-model inspection.
Add-Type -AssemblyName System.Drawing
$src = [System.Drawing.Image]::FromFile($Png)
$bmp = New-Object System.Drawing.Bitmap($src)
$w = $bmp.Width; $h = $bmp.Height
$ramp = " .:-=+*#%@"
for ($ry = 0; $ry -lt $Rows; $ry++) {
  $line = ""
  for ($rx = 0; $rx -lt $Cols; $rx++) {
    $x0 = [int]($rx * $w / $Cols); $y0 = [int]($ry * $h / $Rows)
    $x1 = [int](($rx + 1) * $w / $Cols); $y1 = [int](($ry + 1) * $h / $Rows)
    if ($x1 -le $x0) { $x1 = $x0 + 1 }
    if ($y1 -le $y0) { $y1 = $y0 + 1 }
    $sum = 0; $n = 0
    for ($y = $y0; $y -lt $y1; $y += 2) {
      for ($x = $x0; $x -lt $x1; $x += 2) {
        $c = $bmp.GetPixel($x, $y)
        $sum += ($c.R * 299 + $c.G * 587 + $c.B * 114) / 1000
        $n++
      }
    }
    $lum = $sum / [Math]::Max($n, 1)
    $idx = [int]($lum / 256 * ($ramp.Length - 1))
    $line += $ramp[$idx]
  }
  Write-Output $line
}
$src.Dispose(); $bmp.Dispose()
