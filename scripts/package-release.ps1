[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$version = (Get-Content -LiteralPath (Join-Path $repoRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Expected a numeric release version.' }
$exe = Join-Path $repoRoot 'src-tauri/target/release/pcisland.exe'
$installer = Join-Path $repoRoot "src-tauri/target/release/bundle/nsis/PCisland_${version}_x64-setup.exe"
foreach ($source in @($exe, $installer)) {
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Build output missing: $source" }
}
if ((Get-Item -LiteralPath $exe).VersionInfo.ProductVersion -ne $version) { throw 'Executable version differs from source.' }
$out = Join-Path $repoRoot "release/$version"
New-Item -ItemType Directory -Force -Path $out | Out-Null
$zipPath = Join-Path $out "PCisland-$version-portable.zip"
$setupPath = Join-Path $out "PCisland-$version-setup.exe"
foreach ($destination in @($zipPath, $setupPath, "$zipPath.sha256", "$setupPath.sha256")) {
    if (Test-Path -LiteralPath $destination) { throw "Release asset already exists: $destination" }
}
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
# An explicit allowlist prevents packaging local databases, recordings or .env.
$zip = [IO.Compression.ZipFile]::Open($zipPath, [IO.Compression.ZipArchiveMode]::Create)
try {
    [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $exe, 'PCisland.exe', [IO.Compression.CompressionLevel]::Optimal) | Out-Null
    [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, (Join-Path $PSScriptRoot 'README-portable.txt'), 'README-portable.txt', [IO.Compression.CompressionLevel]::Optimal) | Out-Null
    $zip.CreateEntry('data/.portable') | Out-Null
} finally { $zip.Dispose() }
[IO.File]::Copy($installer, $setupPath, $false)
foreach ($asset in @($zipPath, $setupPath)) {
    $stream = [IO.File]::OpenRead($asset)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $hash = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $sha.Dispose() }
    [IO.File]::WriteAllText("$asset.sha256", "$hash  $([IO.Path]::GetFileName($asset))`n", [Text.Encoding]::ASCII)
    Write-Host "$hash  $asset"
}
Write-Host "Release assets prepared in $out. Publish only after checks pass."
