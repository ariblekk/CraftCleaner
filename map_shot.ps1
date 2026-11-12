param([string]$Name = 'now')
Add-Type -AssemblyName System.Drawing
$p = Join-Path $PSScriptRoot "shots\$Name.png"
if (-not (Test-Path $p)) { Write-Output "missing $p"; exit 1 }
$b = [System.Drawing.Bitmap]::FromFile($p)
Write-Output "size: $($b.Width)x$($b.Height)"

function Class($c) {
  $r = [int]$c.R; $g = [int]$c.G; $bl = [int]$c.B
  if ($bl -gt 60 -and $bl -gt ($r + 40) -and $g -lt ($bl - 20)) { return '#' }
  if ($r -lt 80 -and $g -lt 100 -and $bl -lt 110) { return 'N' }
  if ($r -gt 235 -and $g -gt 235 -and $bl -gt 235) { return '.' }
  if ($r -gt 230 -and $g -gt 235 -and $bl -gt 238) { return ' ' }
  if ($r -lt 210 -and $g -gt 140 -and $bl -lt 190 -and [math]::Abs($g - $bl) -gt 30) { return 'R' }
  if ($r -gt 150 -and $r -lt 230 -and $g -lt 120) { return 'W' }
  if ($r -lt 140 -and $g -lt 150 -and $bl -lt 160) { return 'T' }
  return '-'
}

for ($y = 2; $y -lt $b.Height; $y += 12) {
  $line = ''
  for ($x = 2; $x -lt $b.Width; $x += 8) {
    $line += (Class $b.GetPixel($x, $y))
  }
  Write-Output $line
}
$b.Dispose()
