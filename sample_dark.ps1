param([string]$Name = 'cleaner')
Add-Type -AssemblyName System.Drawing
$b = [System.Drawing.Bitmap]::FromFile((Join-Path $PSScriptRoot "shots\$Name.png"))
Write-Output "size: $($b.Width)x$($b.Height)"

# 3-char codes: near-black page bg, nav bg, card, card-soft, line, violet, cyan, white text, muted text
function Code($c) {
  $r = [int]$c.R; $g = [int]$c.G; $bl = [int]$c.B
  if ($r -gt 200 -and $g -gt 200) { return 'TXT' }
  if ($r -gt 120 -and $bl -gt 200 -and $g -lt 140) { return 'VIO' }
  if ($bl -gt 180 -and $g -gt 160 -and $r -lt 90) { return 'CYA' }
  if ($r -ge 26 -and $r -le 30 -and $g -ge 30 -and $g -le 36) { return 'nav' }
  if ($r -ge 30 -and $r -le 38 -and $bl -ge 40 -and $bl -le 52) { return 'crd' }
  if ($r -ge 36 -and $r -le 44 -and $bl -ge 50 -and $bl -le 60) { return 'sof' }
  if ($r -lt 26 -and $bl -lt 34) { return '...' }
  if ($r -ge 44 -and $r -le 60 -and $bl -ge 60 -and $bl -le 78) { return 'lin' }
  if ($r -ge 110 -and $r -le 170 -and $bl -ge 140) { return 'mut' }
  return ('{0:X2}{1:X2}{2:X2}' -f $r, $g, $bl)
}

for ($y = 20; $y -lt $b.Height; $y += 30) {
  $line = ('{0,4} ' -f $y)
  for ($x = 210; $x -lt $b.Width; $x += 26) {
    $line += (Code $b.GetPixel($x, $y)) + ' '
  }
  Write-Output $line
}
$b.Dispose()
