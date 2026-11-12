Add-Type -AssemblyName System.Drawing
Get-ChildItem (Join-Path $PSScriptRoot 'shots\*.png') | ForEach-Object {
  $b = [System.Drawing.Bitmap]::FromFile($_.FullName)
  $seen = @{}
  for ($y = 4; $y -lt $b.Height; $y += 17) {
    for ($x = 4; $x -lt $b.Width; $x += 17) { $seen[$b.GetPixel($x, $y).ToArgb()] = 1 }
  }
  Write-Output ("{0} {1}x{2} colours={3}" -f $_.Name, $b.Width, $b.Height, $seen.Count)
  $b.Dispose()
}
