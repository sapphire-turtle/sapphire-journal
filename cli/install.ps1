$ErrorActionPreference = 'Stop'

$Repo = 'fluo10/sapphire-journal'
$InstallDir = Join-Path $HOME '.local\bin'
$Binary = 'sapphire-journal'

$Releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases"
# The CLI's releases are tagged `v*` (`cli-v*` up to 0.12); the desktop's,
# core's and the others' carry their own prefix.
$Version = ($Releases | Where-Object { $_.tag_name -match '^(cli-)?v\d' } | Select-Object -First 1).tag_name
if (-not $Version) {
    Write-Error 'Failed to fetch latest version.'
    exit 1
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$Asset = "$Binary-windows-x86_64.exe"
$Url = "https://github.com/$Repo/releases/download/$Version/$Asset"
$Dest = Join-Path $InstallDir "$Binary.exe"

Write-Host "Installing $Binary $Version to $InstallDir..."
Invoke-WebRequest -Uri $Url -OutFile $Dest
Write-Host "Done! $Dest installed."

$UserPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable('PATH', "$InstallDir;$UserPath", 'User')
    Write-Host ""
    Write-Host "Added $InstallDir to your PATH. Restart your terminal to apply."
}
