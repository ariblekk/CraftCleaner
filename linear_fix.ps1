$p = Join-Path $PSScriptRoot 'src\main.rs'
$t = Get-Content -Raw $p
$t = $t -replace 'CornerRadius::same\(\d+\)', 'CornerRadius::ZERO'
Set-Content -NoNewline -Path $p -Value $t
Write-Output 'done'