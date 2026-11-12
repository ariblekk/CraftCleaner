Add-Type -AssemblyName System.Drawing
$p = Join-Path $PSScriptRoot 'shots\now.png'
$i = Get-Item $p
Write-Output "bytes: $($i.Length)"
$b = [System.Drawing.Bitmap]::FromFile($i.FullName)
Write-Output "size: $($b.Width)x$($b.Height)"
$c = @{}
for ($x = 0; $x -lt $b.Width; $x += 11) {
  for ($y = 0; $y -lt $b.Height; $y += 11) {
    $k = $b.GetPixel($x, $y).ToArgb()
    $c[$k] = 1
  }
}
Write-Output "distinct colours in sample: $($c.Count)"
$b.Dispose()
