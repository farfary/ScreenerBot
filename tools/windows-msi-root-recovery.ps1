param(
    [Parameter(Mandatory = $true)]
    [string]$ProductCode,

    [switch]$Execute
)

$ErrorActionPreference = 'Stop'

if (-not [Environment]::Is64BitProcess) {
    throw 'Run this script from 64-bit Windows PowerShell.'
}

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this script from an elevated PowerShell window.'
}

$parsedCode = [Guid]::Empty
if (-not [Guid]::TryParse($ProductCode, [ref]$parsedCode)) {
    throw 'ProductCode must be a Windows Installer product GUID.'
}
$productCode = '{' + $parsedCode.ToString().ToUpperInvariant() + '}'
$keyPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$productCode.msq"
$registration = Get-ItemProperty -LiteralPath $keyPath

if ($registration.DisplayName -notin @('ScreenerBot', 'ScreenerBot (Machine)') -or
    $registration.Publisher -ne 'ScreenerBot') {
    throw 'The product code does not identify a ScreenerBot machine MSI registration.'
}

$installer = New-Object -ComObject WindowsInstaller.Installer
$installedName = $installer.ProductInfo($productCode, 'ProductName')
$installedVersion = $installer.ProductInfo($productCode, 'VersionString')
if ($installedName -ne 'ScreenerBot (Machine - MSI)' -or
    ($installedVersion -ne $registration.DisplayVersion -and
     $installedVersion -ne "$($registration.DisplayVersion).0")) {
    throw 'The Windows Installer product does not match the custom uninstall registration.'
}

$installPath = [string]$registration.InstallPath
$isDriveRoot = $installPath -match '^[A-Za-z]:\\$'
$guardPrefix = Join-Path $env:ProgramData 'ScreenerBot-MSI-Recovery-'
$guardPattern = '^' + [regex]::Escape($guardPrefix) + '[0-9a-f]{32}\\$'
$isPrepared = $installPath -match $guardPattern
if (-not $isDriveRoot -and -not $isPrepared) {
    throw "The recorded install path is neither a drive root nor a prepared recovery folder: $installPath"
}
if ($isPrepared) {
    $preparedFolder = Get-Item -LiteralPath $installPath.TrimEnd('\')
    if (-not $preparedFolder.PSIsContainer -or
        ($preparedFolder.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
        (Get-ChildItem -LiteralPath $preparedFolder.FullName -Force | Select-Object -First 1)) {
        throw 'The prepared recovery folder is not an empty, ordinary directory.'
    }
}

$running = Get-Process -Name 'ScreenerBot', 'screenerbot' -ErrorAction SilentlyContinue
if ($running) {
    throw 'Close ScreenerBot before removing its MSI registration.'
}

Write-Output "Product: $installedName $installedVersion ($productCode)"
Write-Output "Recorded install path: $installPath"
if (-not $Execute) {
    Write-Output 'Dry run only. -Execute uses an empty recovery folder for recursive cleanup, then runs MSI uninstall.'
    exit 0
}

if ($isDriveRoot) {
    $guardPath = Join-Path $env:ProgramData ('ScreenerBot-MSI-Recovery-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $guardPath -ErrorAction Stop | Out-Null
    $guardPath = (Get-Item -LiteralPath $guardPath).FullName.TrimEnd('\') + '\'
    if (Get-ChildItem -LiteralPath $guardPath -Force | Select-Object -First 1) {
        throw 'The recovery folder is not empty.'
    }

    Set-ItemProperty -LiteralPath $keyPath -Name InstallPath -Value $guardPath
    if ((Get-ItemProperty -LiteralPath $keyPath).InstallPath -cne $guardPath) {
        throw 'The safe install path was not persisted; MSI uninstall was not started.'
    }
}

$logPath = Join-Path $env:TEMP ('ScreenerBot-msi-uninstall-' + $parsedCode.ToString('N') + '.log')
$arguments = "/x $productCode /qn /norestart /l*v `"$logPath`""
$result = Start-Process -FilePath msiexec.exe -ArgumentList $arguments -Wait -PassThru
if ($result.ExitCode -notin @(0, 3010)) {
    throw "MSI uninstall failed with exit code $($result.ExitCode). The safe path remains registered. Log: $logPath"
}

Write-Output "MSI uninstall completed with exit code $($result.ExitCode). Log: $logPath"
