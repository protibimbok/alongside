# Install concurtui from GitHub releases (Windows x64).
#
# Usage:
#   irm https://raw.githubusercontent.com/protibimbok/concurtui/master/scripts/install.ps1 | iex
#
# Options (environment variables, since piped scripts take no arguments):
#   $env:CONCURTUI_VERSION      Release tag to install (default: latest), e.g. v0.1.0
#   $env:CONCURTUI_INSTALL_DIR  Install directory (default: %LOCALAPPDATA%\Programs\concurtui)
#
# The install directory is added to your user PATH. No administrator rights needed.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue' # Invoke-WebRequest is much faster without the progress bar

$Repo = 'protibimbok/concurtui'
$Binary = 'concurtui'

function Info($msg) { Write-Host "[concurtui] $msg" -ForegroundColor Green }
function Fail($msg) { Write-Host "[concurtui] error: $msg" -ForegroundColor Red; exit 1 }

$arch = $env:PROCESSOR_ARCHITECTURE
if ($env:PROCESSOR_ARCHITEW6432) { $arch = $env:PROCESSOR_ARCHITEW6432 }
# arm64 Windows runs the x64 build through emulation.
if ($arch -notin @('AMD64', 'ARM64')) { Fail "unsupported architecture: $arch (supported: x64)" }

$version = if ($env:CONCURTUI_VERSION) { $env:CONCURTUI_VERSION } else { 'latest' }
if ($version -eq 'latest') {
    try {
        $version = (Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest").tag_name
    } catch {
        Fail "could not resolve the latest release (no release yet, or GitHub rate limit): $_"
    }
} elseif (-not $version.StartsWith('v')) {
    $version = "v$version"
}

$installDir = if ($env:CONCURTUI_INSTALL_DIR) { $env:CONCURTUI_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\$Binary" }
$archive = "${Binary}_windows_amd64.zip"
$base = "https://github.com/$Repo/releases/download/$version"

Info "Installing concurtui $version (windows/amd64) to $installDir..."

$tmp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    Info "Downloading $base/$archive"
    Invoke-WebRequest "$base/$archive" -OutFile (Join-Path $tmp $archive)
    Invoke-WebRequest "$base/checksums.txt" -OutFile (Join-Path $tmp 'checksums.txt')

    $expected = Get-Content (Join-Path $tmp 'checksums.txt') |
        ForEach-Object { $parts = $_ -split '\s+'; if ($parts[1] -eq $archive) { $parts[0] } } |
        Select-Object -First 1
    if (-not $expected) { Fail "checksum not found for $archive" }
    $actual = (Get-FileHash (Join-Path $tmp $archive) -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $expected.ToLower()) { Fail "checksum mismatch for $archive" }
    Info 'Checksum verified'

    Expand-Archive (Join-Path $tmp $archive) -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Copy-Item (Join-Path $tmp "$Binary.exe") (Join-Path $installDir "$Binary.exe") -Force
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not (($userPath -split ';') -contains $installDir)) {
    $newPath = if ($userPath) { "$userPath;$installDir" } else { $installDir }
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    $env:Path = "$env:Path;$installDir"
    Info "Added $installDir to your user PATH (open a new terminal to pick it up)"
}

Info "Installed: $(& (Join-Path $installDir "$Binary.exe") --version)"
