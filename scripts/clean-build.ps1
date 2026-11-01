[CmdletBinding(SupportsShouldProcess = $true)]
param()

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$repoPrefix = $repoRoot.TrimEnd('\') + '\'

function Get-ExeHash([string] $Path) {
    $stream = [IO.File]::OpenRead($Path)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '') }
    finally { $stream.Dispose(); $sha.Dispose() }
}

# Only generated compiler/cache paths. Never remove target/, a profile wholesale,
# release packages, portable data, installed dependencies, or global tool caches.
$relativePaths = @(
    'src-tauri/target/debug/.fingerprint', 'src-tauri/target/debug/build',
    'src-tauri/target/debug/deps', 'src-tauri/target/debug/examples',
    'src-tauri/target/debug/incremental', 'src-tauri/target/debug/pcisland.exe',
    'src-tauri/target/debug/pcisland.pdb', 'src-tauri/target/debug/pcisland.d',
    'src-tauri/target/debug/libpcisland_lib.rlib', 'src-tauri/target/debug/libpcisland_lib.d',
    'src-tauri/target/release/.fingerprint', 'src-tauri/target/release/build',
    'src-tauri/target/release/deps', 'src-tauri/target/release/examples',
    'src-tauri/target/release/incremental', 'src-tauri/target/release/nsis',
    'src-tauri/target/release/pcisland.pdb', 'src-tauri/target/release/pcisland.d',
    'src-tauri/target/release/libpcisland_lib.rlib', 'src-tauri/target/release/libpcisland_lib.d',
    'src-tauri/target/release/pcisland-running-previous.exe',
    'src-tauri/target/release/pcisland-running-previous-link.exe',
    'src-tauri/target/qa-0.1.1/pcisland-qa.exe',
    'node_modules/.vite', 'node_modules/.vite-temp', 'node_modules/.vitest-cache'
)

function Assert-SafePath([string] $Path) {
    $absolute = [IO.Path]::GetFullPath($Path)
    if (-not $absolute.StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Cleanup target is outside the repository: $absolute"
    }
    # Resolve the existing path and reject junctions/symlinks in every ancestor.
    if ((Resolve-Path -LiteralPath $absolute).Provider.Name -ne 'FileSystem') {
        throw "Not a filesystem path: $absolute"
    }
    $cursor = $absolute
    while ($cursor.Length -ge $repoRoot.Length) {
        $item = Get-Item -LiteralPath $cursor -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Refusing to follow a link during cleanup: $cursor"
        }
        $cursor = Split-Path -Parent $cursor
    }
    return $absolute
}

function Get-SafeTreeSize([string] $Path) {
    $pending = New-Object 'System.Collections.Generic.Stack[string]'
    $pending.Push($Path)
    [long] $bytes = 0
    while ($pending.Count -gt 0) {
        $item = Get-Item -LiteralPath $pending.Pop() -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Refusing to delete a tree containing a link: $($item.FullName)"
        }
        if ($item.PSIsContainer) {
            if ($item.Name -eq 'data') { throw "Unexpected data directory: $($item.FullName)" }
            foreach ($child in Get-ChildItem -LiteralPath $item.FullName -Force) {
                $pending.Push($child.FullName)
            }
        } else {
            if ($item.Name -in @('pcisland.db', 'settings.json')) {
                throw "Unexpected application data: $($item.FullName)"
            }
            $bytes += $item.Length
        }
    }
    return $bytes
}

# Fail closed when process inspection is unavailable. Do not interrupt user apps.
$processes = Get-CimInstance Win32_Process -Filter "Name='cargo.exe' OR Name='rustc.exe' OR Name='node.exe' OR Name='pcisland.exe' OR Name='pcisland-qa.exe'"
foreach ($process in $processes) {
    $building = $process.Name -in @('cargo.exe', 'rustc.exe')
    $nodeBusy = $process.Name -eq 'node.exe' -and $process.CommandLine -like "*$repoRoot*" -and $process.CommandLine -match 'vite|vitest|tauri'
    $testApp = $process.ExecutablePath -and (
        $process.ExecutablePath.StartsWith((Join-Path $repoRoot 'src-tauri\target\debug\'), [StringComparison]::OrdinalIgnoreCase) -or
        $process.ExecutablePath.StartsWith((Join-Path $repoRoot 'src-tauri\target\qa-0.1.1\'), [StringComparison]::OrdinalIgnoreCase)
    )
    if ($building -or $nodeBusy -or $testApp) {
        throw "Cleanup postponed: $($process.Name) (PID $($process.ProcessId)) is still running."
    }
}

$latestExe = Join-Path $repoRoot 'src-tauri\target\release\pcisland.exe'
$hashBefore = if (Test-Path -LiteralPath $latestExe) { Get-ExeHash $latestExe } else { $null }
$volume = New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($repoRoot))
$freeBefore = $volume.AvailableFreeSpace

# Validate all targets before any deletion; never pass computed paths to another shell.
$plan = @(
    foreach ($relative in $relativePaths) {
        $path = Join-Path $repoRoot $relative
        if (Test-Path -LiteralPath $path) {
            $absolute = Assert-SafePath $path
            [pscustomobject]@{ Path = $absolute; Bytes = (Get-SafeTreeSize $absolute) }
        }
    }
)
[long] $removed = 0
foreach ($entry in $plan) {
    Write-Host ('{0:N3} GiB  {1}' -f ($entry.Bytes / 1GB), $entry.Path)
    if ($PSCmdlet.ShouldProcess($entry.Path, 'Remove generated build cache')) {
        try {
            Remove-Item -LiteralPath $entry.Path -Recurse -Force
            $removed += $entry.Bytes
        } catch {
            if ([IO.Path]::GetFileName($entry.Path) -in @('pcisland-running-previous.exe', 'pcisland-running-previous-link.exe')) {
                Write-Warning "Previous app is still using $($entry.Path); retry clean:build after exiting that app."
            } else { throw }
        }
    }
}
if ($hashBefore -and (Get-ExeHash $latestExe) -ne $hashBefore) {
    throw 'The release executable changed during cleanup.'
}
Write-Host ('Removed file sizes: {0:N3} GiB; disk free-space change: {1:N3} GiB.' -f ($removed / 1GB), (($volume.AvailableFreeSpace - $freeBefore) / 1GB))
Write-Host 'Kept release exe, installers/packages, dist, dependencies, and application data. Next Rust build will rebuild its cache.'
