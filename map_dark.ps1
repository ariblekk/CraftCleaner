param([string]$Name = 'cleaner')
Add-Type -AssemblyName System.Drawing
$p = Join-Path $PSScriptRoot "shots\$Name.png"
$b = [System.Drawing.Bitmap]::FromFile($p)
Write-Output "size: $($b.Width)x$($b.Height)"

function Class($c) {
  $r = [int]$c.R; $g = [int]$c.G; $bl = [int]$c.B
  if ($bl -gt 150 -and $g -gt 150 -and $r -lt 120) { return 'C' }          # cyan accent
  if ($r -gt 100 -and $bl -gt 180 -and $g -lt 130) { return 'V' }          # violet accent
  if ($r -gt 200 -and $g -gt 210 -and $bl -gt 220) { return 'T' }          # bright text
  if ($r -lt 28 -and $g -lt 30 -and $bl -lt 40) { return ' ' }             # page bg
  if ($r -lt 36 -and $g -lt 40 -and $bl -lt 50) { return 'N' }             # nav / panels
  if ($r -gt 40 -and $r -lt 100 -and $g -gt 45 -and $bl -gt 55) { return '.' } # cards
  if ($r -gt 120 -and $g -gt 120 -and $bl -gt 140) { return 'm' }          # muted text
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
