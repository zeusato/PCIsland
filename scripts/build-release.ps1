[CmdletBinding()]
param([switch] $NoBundle)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $repoRoot
try {
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
    & (Join-Path $PSScriptRoot 'clean-build.ps1')
} finally {
    Pop-Location
}
