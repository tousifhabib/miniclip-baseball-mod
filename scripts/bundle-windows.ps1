# Builds the game as a Windows app: target\app\Baseball Mod, a folder that
# can be put anywhere and started by its program, and the same folder in a
# zip beside it to hand to someone else.
#
#   scripts\bundle-windows.ps1           the app with the art beside it
#   scripts\bundle-windows.ps1 -NoArt    the app alone; it then looks for the art in
#                                        %APPDATA%\<id>\extracted
#
# From a prompt that will not run scripts as they are:
#
#   powershell -ExecutionPolicy Bypass -File scripts\bundle-windows.ps1
#
# The art is whatever is in extracted\. The program is built to be started
# as an app, with no console window of its own, and with the icon drawn
# from assets\icon.svg inside it.

[CmdletBinding()]
param(
    [switch] $NoArt
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
Set-Location (Split-Path -Parent $PSScriptRoot)

$AppName = 'Baseball Mod'
$AppId = 'io.github.tousifhabib.baseball-mod'   # must match APP_ID in crates/game/src/locate.rs
$Program = "$AppName.exe"

# Runs a program and stops here if it fails, as `set -e` would.
function Invoke-Checked {
    param([string] $What, [scriptblock] $Command)
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$What failed (exit code $LASTEXITCODE)"
    }
}

$version = Select-String -Path Cargo.toml -Pattern '^version = "(.*)"$' |
    Select-Object -First 1 |
    ForEach-Object { $_.Matches[0].Groups[1].Value }
if (-not $version) {
    throw 'no version found in Cargo.toml'
}
$hostLine = rustc -vV | Select-String -Pattern '^host: (.*)$'
$arch = ($hostLine.Matches[0].Groups[1].Value -split '-')[0]

if (-not $NoArt -and -not (Test-Path extracted\manifest.json)) {
    throw 'There is no art in extracted\. Pass -NoArt to build without it.'
}

# The icon, in every size Windows asks for, made first so that the build can
# put it inside the program.
Write-Host "Building version $version for $arch..."
Invoke-Checked 'building svg-png' { cargo build --release -p bb-modtools --bin svg-png }
New-Item -ItemType Directory -Force target\app | Out-Null
$icon = Join-Path (Resolve-Path target\app).Path 'icon.ico'
$widths = 16, 24, 32, 48, 64, 128, 256 | ForEach-Object { '--width', $_ }
Invoke-Checked 'drawing the icon' { target\release\svg-png.exe assets\icon.svg @widths --out $icon }

$env:BB_WINDOWS_ICON = $icon
try {
    Invoke-Checked 'building the game' { cargo build --profile dist -p bb-game --features app }
} finally {
    Remove-Item Env:\BB_WINDOWS_ICON
}

# The app is put together out of sight and moved into place when it is
# whole, so that a build that fails half way leaves the last one as it was.
$final = "target\app\$AppName"
$staging = 'target\app\staging'
$app = Join-Path $staging $AppName
if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
New-Item -ItemType Directory -Force $app | Out-Null

Copy-Item target\dist\bb-game.exe (Join-Path $app $Program)
if (-not $NoArt) {
    Copy-Item -Recurse extracted (Join-Path $app 'extracted')
}

if (Test-Path $final) { Remove-Item -Recurse -Force $final }
Move-Item $app $final
Remove-Item -Recurse -Force $staging

# Not signed. Windows warns before running a program from the internet that
# is not, until enough people have run it; giving the app out widely wants a
# code signing certificate.
$zip = "target\app\$($AppName -replace ' ', '-')-$version-windows-$arch.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $final -DestinationPath $zip

$size = (Get-ChildItem -Recurse -File $final | Measure-Object -Property Length -Sum).Sum
Write-Host ("Made {0} ({1:N0} MB) and {2}" -f $final, ($size / 1MB), $zip)
Write-Host "Kept files go in %APPDATA%\$AppId."
