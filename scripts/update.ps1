<#
.SYNOPSIS
  Updates an installed textweaver on Windows.

.DESCRIPTION
  Reads install-manifest.txt in the install folder to see how textweaver was
  installed. A release install runs install-windows.ps1 again, which
  downloads, checks, and installs the newest release. A source install pulls
  the newest code with git pull --ff-only and runs install-windows.ps1
  -FromSource. Your settings and reading positions are not touched.

  Works in Windows PowerShell 5.1 and PowerShell 7. GNU-style options work
  too: --help, --dry-run, --yes, --install-dir DIR.

.PARAMETER InstallDir
  Where textweaver is installed (default: %LOCALAPPDATA%\Programs\textweaver).

.PARAMETER DryRun
  Print each step instead of doing it.

.PARAMETER Yes
  Answer yes to every question.

.PARAMETER Help
  Show the help.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\update.ps1
#>
[CmdletBinding(PositionalBinding = $false)]
param(
    [string] $InstallDir,
    [switch] $DryRun,
    [switch] $Yes,
    [switch] $Help,
    [Parameter(ValueFromRemainingArguments = $true)] [string[]] $Rest
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Write-Line([string] $Text = '') { [Console]::Out.WriteLine($Text) }

$i = 0
while ($null -ne $Rest -and $i -lt $Rest.Count) {
    $a = $Rest[$i]
    switch -Regex ($a) {
        '^(--help|-h|/\?|-\?)$' { $Help = $true }
        '^--dry-run$' { $DryRun = $true }
        '^(--yes|-y)$' { $Yes = $true }
        '^--(install-dir|prefix)$' { $i++; $InstallDir = $Rest[$i] }
        default { Write-Line "Error: unknown option $a. Run with -Help to see the options."; exit 2 }
    }
    $i++
}

if ($Help) {
    Write-Line 'Usage: scripts\update.ps1 [-InstallDir DIR] [-DryRun] [-Yes] [-Help]'
    Write-Line
    Write-Line 'Updates textweaver where install-windows.ps1 put it. A release install gets the'
    Write-Line 'newest release; a source install is pulled with git and rebuilt. Settings and'
    Write-Line 'reading positions are not touched.'
    Write-Line
    Write-Line 'Options:'
    Write-Line '  -InstallDir DIR  Where textweaver is installed (default: %LOCALAPPDATA%\Programs\textweaver).'
    Write-Line '  -DryRun          Print each step instead of doing it.'
    Write-Line '  -Yes             Answer yes to every question.'
    Write-Line '  -Help            Show this help.'
    exit 0
}

if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\textweaver' }
$InstallDir = [IO.Path]::GetFullPath($InstallDir)
$manifest = Join-Path $InstallDir 'install-manifest.txt'
$RepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))

function Get-ManifestValue([string] $Name) {
    $m = Select-String -LiteralPath $manifest -Pattern "^$Name=(.*)$" | Select-Object -First 1
    if ($m) { return $m.Matches[0].Groups[1].Value }
    return ''
}

if ($DryRun) { Write-Line 'Dry run: nothing is changed. Each step is printed instead of done.' }

$kind = ''
$src = ''
if (Test-Path -LiteralPath $manifest) {
    $kind = Get-ManifestValue 'kind'
    $src = Get-ManifestValue 'source'
    Write-Line "Found a $kind install of textweaver $(Get-ManifestValue 'version') in $InstallDir."
} elseif (Test-Path -LiteralPath (Join-Path $RepoRoot 'crates\textweaver-cli')) {
    $kind = 'source'
    $src = $RepoRoot
    Write-Line "No install was found in $InstallDir, so this updates the checkout in $src and installs from it."
} else {
    Write-Line "Error: no textweaver install was found in $InstallDir. Use -InstallDir, or install with scripts\install-windows.ps1."
    exit 1
}

$installer = @((Join-Path $PSScriptRoot 'install-windows.ps1'), (Join-Path $InstallDir 'scripts\install-windows.ps1')) |
    Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
$arguments = @('-InstallDir', $InstallDir)
if ($DryRun) { $arguments += '-DryRun' }
if ($Yes) { $arguments += '-Yes' }

if ($kind -eq 'source') {
    if (-not (Test-Path -LiteralPath (Join-Path $src 'crates\textweaver-cli'))) {
        Write-Line "Error: the source folder $src is missing or is not a textweaver checkout."
        exit 1
    }
    if (Test-Path -LiteralPath (Join-Path $src '.git')) {
        Write-Line "Pulling the newest code into $src."
        if ($DryRun) {
            Write-Line "Would run: git -C $src pull --ff-only"
        } else {
            Write-Line "Running: git -C $src pull --ff-only"
            & git -C $src pull --ff-only
            if ($LASTEXITCODE -ne 0) {
                Write-Line "Error: git pull could not fast-forward $src. It may have local changes; look with: git -C $src status"
                exit 1
            }
        }
    } else {
        Write-Line "$src is not a git checkout, so it is rebuilt as it is."
    }
    $installer = Join-Path $src 'scripts\install-windows.ps1'
    $arguments += @('-FromSource', '-Source', $src)
} elseif ($kind -ne 'release') {
    Write-Line "Error: the manifest names an unknown kind of install: $kind."
    exit 1
}

if (-not $installer -or -not (Test-Path -LiteralPath $installer)) {
    Write-Line 'Error: install-windows.ps1 was not found. Download it from https://github.com/leavesofgrass/textweaver/tree/main/scripts and run it.'
    exit 1
}

# The installer runs in a dry run too, with -DryRun, so it prints its steps.
$shell = (Get-Process -Id $PID).Path
Write-Line "Running: $installer $($arguments -join ' ')"
& $shell -NoProfile -ExecutionPolicy Bypass -File $installer @arguments
exit $LASTEXITCODE
