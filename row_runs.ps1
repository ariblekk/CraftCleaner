param([string]$Name = 'now', [int]$Y = 200)
Add-Type -AssemblyName System.Drawing
$p = Join-Path $PSScriptRoot "shots\$Name.png"
$b = [System.Drawing.Bitmap]::FromFile($p)
Write-Output "size: $($b.Width)x$($b.Height)  row y=$Y"

$prev = ''
$start = 0
$runs = @()
for ($x = 0; $x -lt $b.Width; $x++) {
  $c = $b.GetPixel($x, $Y)
  $r = [int]$c.R; $g = [int]$c.G; $bl = [int]$c.B
  $k = "$r,$g,$bl"
  if ($k -ne $prev) {
    if ($prev -ne '') { $runs += [pscustomobject]@{ From = $start; To = $x - 1; Colour = $prev; Len = $x - $start } }
    $prev = $k; $start = $x
  }
}
$runs += [pscustomobject]@{ From = $start; To = $b.Width - 1; Colour = $prev; Len = $b.Width - $start }
$b.Dispose()
$runs | Where-Object { $_.Len -ge 2 } | ForEach-Object {
  Write-Output ("x {0,4}..{1,4}  len {2,4}  rgb({3})" -f $_.From, $_.To, $_.Len, $_.Colour)
}
