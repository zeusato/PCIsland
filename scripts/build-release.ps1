[CmdletBinding()]
param(
    [switch] $NoBundle,
    # Keep the Rust cache for fast rebuilds; clean only when it outgrows this cap (docs/build-cleanup.md).
    [double] $MaxCacheGiB = 20,
    [switch] $Clean
)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $repoRoot
try {
    # audiopus builds libopus from source; Visual Studio ships a suitable CMake.
    if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        $cmakePath = if (Test-Path -LiteralPath $vswhere) {
            & $vswhere -latest -products '*' -find '**\CMake\bin\cmake.exe' | Select-Object -First 1
        }
        if (-not $cmakePath) { throw 'CMake is required for Opus. Install the C++ CMake tools in Visual Studio Build Tools.' }
        $env:PATH = (Split-Path -Parent $cmakePath) + ';' + $env:PATH
    }
    $buildArgs = @('tauri', 'build')
    if ($NoBundle) { $buildArgs += '--no-bundle' }
    & pnpm.cmd @buildArgs
    if ($LASTEXITCODE -ne 0) { throw "Build failed (exit $LASTEXITCODE); cleanup was not run." }
    $exe = Join-Path $repoRoot 'src-tauri\target\release\pcisland.exe'
    if (-not (Test-Path -LiteralPath $exe)) { throw 'Build did not produce the release executable.' }
    $stream = [IO.File]::OpenRead($exe)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { Write-Host ('SHA256: ' + [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '')) }
    finally { $stream.Dispose(); $sha.Dispose() }
    $target = Join-Path $repoRoot 'src-tauri\target'
    $cacheGiB = ((Get-ChildItem -LiteralPath $target -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum) / 1GB
    Write-Host ('Rust target size: {0:N2} GiB (cap {1} GiB).' -f $cacheGiB, $MaxCacheGiB)
    if ($Clean -or $cacheGiB -gt $MaxCacheGiB) {
        & (Join-Path $PSScriptRoot 'clean-build.ps1')
    } else {
        Write-Host 'Cache kept for faster rebuilds.'
    }
} finally {
    Pop-Location
}
