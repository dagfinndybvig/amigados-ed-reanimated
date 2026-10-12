# OCR a screenshot and print each recognized word with its bounding box,
# relative to the source image (top-left origin).
param([string]$Png, [int]$Scale = 1, [int]$OffX = 0, [int]$OffY = 0)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Media.Ocr.OcrEngine, Windows.Media.Ocr, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics.Imaging, ContentType = WindowsRuntime]
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]

function Await($WinRTTask, $ResultType) {
  $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() |
    Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
  $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
  $netTask = $asTask.Invoke($null, @($WinRTTask))
  $netTask.Wait(-1) | Out-Null
  $netTask.Result
}

$path = (Get-Item $Png).FullName
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($path)) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$soft = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) { Write-Output "NO_OCR_ENGINE"; exit 1 }
$result = Await ($engine.RecognizeAsync($soft)) ([Windows.Media.Ocr.OcrResult])
foreach ($line in $result.Lines) {
  foreach ($word in $line.Words) {
    $r = $word.BoundingRect
    Write-Output ("{0,4},{1,4} {2,4}x{3,3}  {4}" -f ([int]($r.X / $Scale) + $OffX), ([int]($r.Y / $Scale) + $OffY), ([int]($r.Width / $Scale)), ([int]($r.Height / $Scale)), $word.Text)
  }
}
