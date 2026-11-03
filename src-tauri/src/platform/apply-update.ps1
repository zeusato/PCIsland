$ErrorActionPreference = 'Stop'
$stage = [IO.Path]::GetDirectoryName($env:PCI_ASSET)
$temporary = $null
try {
    $parentProcess = Get-Process -Id ([int]$env:PCI_PARENT) -ErrorAction SilentlyContinue
    if ($parentProcess -and !$parentProcess.WaitForExit(120000)) { throw 'App did not exit in time' }
    $stream = [IO.File]::OpenRead($env:PCI_ASSET)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $actual = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '') }
    finally { $sha.Dispose(); $stream.Dispose() }
    if ($actual -ne $env:PCI_DIGEST) { throw 'Update checksum mismatch' }
    if ([IO.Path]::GetFileName($env:PCI_ASSET) -eq 'setup.exe') {
        $setup = Start-Process -FilePath $env:PCI_ASSET -ArgumentList '/S','/UPDATE' -Wait -PassThru
        if ($setup.ExitCode -ne 0) { throw "Installer exited with $($setup.ExitCode)" }
    } else {
        $temporary = [IO.Path]::Combine([IO.Path]::GetDirectoryName($env:PCI_TARGET), [IO.Path]::GetRandomFileName())
        [IO.File]::Copy($env:PCI_ASSET, $temporary, $false)
        [IO.File]::Replace($temporary, $env:PCI_TARGET, [IO.Path]::Combine($stage, 'unused-backup'))
    }
    if ($env:PCI_RESTART -eq '1') {
        if ($env:PCI_DATA_DIR) { & $env:PCI_TARGET --data-dir $env:PCI_DATA_DIR }
        else { & $env:PCI_TARGET }
    }
    [IO.File]::Delete($env:PCI_ASSET)
    [IO.File]::Delete([IO.Path]::Combine($stage, 'unused-backup'))
    [IO.Directory]::Delete($stage)
} catch {
    [IO.File]::WriteAllText([IO.Path]::Combine($stage, 'update-error.txt'), $_.Exception.Message)
    exit 1
} finally {
    if ($temporary) { [IO.File]::Delete($temporary) }
}
