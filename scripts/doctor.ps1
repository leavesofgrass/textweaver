<#
.SYNOPSIS
  One plain-text report about this Windows system and textweaver, to paste
  into a bug report.

.DESCRIPTION
  Covers Windows and its version, the terminal, the console code page and
  locale, the screen reader if one is running, the Rust toolchain if present,
  where textweaver is installed and where it keeps its settings (tw settings
  path), its speech engines (tw backends), whether the engine hosts and the
  pronunciation dictionaries sit beside tw.exe, and the optional tools.

  It reads nothing personal: no file contents, and no environment dump (only
  the names of TEXTWEAVER_ variables that are set). Your home folder is
  shown as ~. Works in Windows PowerShell 5.1 and PowerShell 7. GNU-style
  options work too: --help, --dry-run, --out FILE.

.PARAMETER Out
  Also write the report to this file.

.PARAMETER Tw
  The tw.exe to check (default: tw on PATH, then a build in this checkout).

.PARAMETER DryRun
  Print each command instead of running it.

.PARAMETER Yes
  Accepted for consistency; this script asks nothing.

.PARAMETER Help
  Show the help.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\doctor.ps1 -Out doctor.txt
#>
[CmdletBinding(PositionalBinding = $false)]
param(
    [string] $Out,
    [string] $Tw,
    [switch] $DryRun,
    [switch] $Yes,
    [switch] $Help,
    [Parameter(ValueFromRemainingArguments = $true)] [string[]] $Rest
)

$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'

$i = 0
while ($null -ne $Rest -and $i -lt $Rest.Count) {
    $a = $Rest[$i]
    switch -Regex ($a) {
        '^(--help|-h|/\?|-\?)$' { $Help = $true }
        '^--dry-run$' { $DryRun = $true }
        '^(--yes|-y)$' { $Yes = $true }
        '^--out$' { $i++; $Out = $Rest[$i] }
        '^--tw$' { $i++; $Tw = $Rest[$i] }
        default { [Console]::Out.WriteLine("Error: unknown option $a. Run with -Help to see the options."); exit 2 }
    }
    $i++
}

if ($Help) {
    [Console]::Out.WriteLine(@'
Usage: scripts\doctor.ps1 [-Out FILE] [-Tw PATH] [-DryRun]

Prints one plain-text report for bug reports: Windows and its version, the terminal,
the code page and locale, the screen reader if one is running, the Rust toolchain,
where textweaver is installed and keeps its settings, its speech engines, whether
the engine hosts and dictionaries sit beside tw.exe, and the optional tools.

Nothing personal is read: no file contents and no environment dump. Your home
folder is shown as ~.

Options:
  -Out FILE   Also write the report to FILE.
  -Tw PATH    The tw.exe to check.
  -DryRun     Print each command instead of running it.
  -Help       Show this help. --help, --dry-run, and --out FILE work too.
'@)
    exit 0
}

$report = New-Object System.Collections.Generic.List[string]

function Hide-UserFolder([string] $Line) {
    if (-not $env:USERPROFILE) { return $Line }
    return [regex]::Replace($Line, [regex]::Escape($env:USERPROFILE), '~', 'IgnoreCase')
}

function Add-Line([string] $Text = '') {
    $line = Hide-UserFolder $Text
    $report.Add($line)
    [Console]::Out.WriteLine($line)
}

function Add-Section([string] $Title) {
    Add-Line
    Add-Line "== $Title =="
}

# The first line a program prints, or "not installed".
function Get-FirstLine([string] $Exe, [string[]] $Arguments) {
    $cmd = Get-Command $Exe -ErrorAction SilentlyContinue
    if (-not $cmd) { return 'not installed' }
    if ($DryRun) { return "(would run: $Exe $($Arguments -join ' '))" }
    $lines = @(& $cmd.Source @Arguments 2>&1 | ForEach-Object { "$_" })
    if ($lines.Count -eq 0) { return '(no output)' }
    return $lines[0]
}

# Runs a program and adds its output, indented.
function Add-Output([string] $Exe, [string[]] $Arguments) {
    if ($DryRun) { Add-Line "  (would run: $Exe $($Arguments -join ' '))"; return }
    foreach ($l in @(& $Exe @Arguments 2>&1 | ForEach-Object { "$_" })) { Add-Line "  $l" }
}

function Test-Running([string] $Name) {
    if (Get-Process -Name $Name -ErrorAction SilentlyContinue) { return 'yes' }
    return 'no'
}

$RepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $Tw) {
    $cmd = Get-Command 'tw.exe' -ErrorAction SilentlyContinue
    if ($cmd) {
        $Tw = $cmd.Source
    } else {
        $found = @(
            (Join-Path $PSScriptRoot '..\tw.exe'),
            (Join-Path $RepoRoot 'target\release\tw.exe'),
            (Join-Path $RepoRoot 'target\debug\tw.exe')
        ) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
        if ($found) { $Tw = [IO.Path]::GetFullPath($found) }
    }
}

Add-Line 'textweaver doctor report'
Add-Line "Date: $((Get-Date).ToString('dddd, MMMM d, yyyy', [Globalization.CultureInfo]::InvariantCulture))"
if ($DryRun) { Add-Line 'Dry run: commands are printed instead of run.' }

Add-Section 'System'
$os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction SilentlyContinue
if ($os) {
    Add-Line "OS: $($os.Caption), version $($os.Version), build $($os.BuildNumber)"
    Add-Line "Architecture: $($os.OSArchitecture)"
} else {
    Add-Line "OS: $([Environment]::OSVersion.VersionString)"
}
$ubr = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion' -ErrorAction SilentlyContinue)
if ($ubr -and $ubr.DisplayVersion) { Add-Line "Windows release: $($ubr.DisplayVersion), update build $($ubr.UBR)" }
Add-Line "PowerShell: $($PSVersionTable.PSVersion) ($($PSVersionTable.PSEdition))"
Add-Line "64-bit process: $([Environment]::Is64BitProcess)"

Add-Section 'Terminal'
$terminal = 'a console window (conhost), or unknown'
if ($env:WT_SESSION) { $terminal = 'Windows Terminal' }
elseif ($env:TERM_PROGRAM) { $terminal = $env:TERM_PROGRAM }
elseif ($env:ConEmuPID) { $terminal = 'ConEmu or Cmder' }
Add-Line "Terminal: $terminal"
Add-Line "PowerShell host: $($Host.Name)"
foreach ($v in 'TERM', 'COLORTERM') {
    $value = [Environment]::GetEnvironmentVariable($v)
    if (-not $value) { $value = 'not set' }
    Add-Line "${v}: $value"
}
if ($env:NO_COLOR) { Add-Line 'NO_COLOR: set' } else { Add-Line 'NO_COLOR: not set' }
try {
    Add-Line "Console code page: output $([Console]::OutputEncoding.CodePage), input $([Console]::InputEncoding.CodePage)"
    if (-not [Console]::IsOutputRedirected) {
        Add-Line "Size: $([Console]::WindowWidth) columns by $([Console]::WindowHeight) lines"
    }
} catch {
    Add-Line 'Console: no console attached.'
}
Add-Line "Output redirected: $([Console]::IsOutputRedirected)"

Add-Section 'Locale'
Add-Line "Culture: $((Get-Culture).Name)"
Add-Line "UI culture: $((Get-UICulture).Name)"
Add-Line "System code page (ANSI): $([Text.Encoding]::Default.CodePage)"

Add-Section 'Screen reader'
Add-Line "NVDA running: $(Test-Running 'nvda')"
Add-Line "JAWS running: $(Test-Running 'jfw')"
Add-Line "Narrator running: $(Test-Running 'Narrator')"
Add-Line "ZoomText or Fusion running: $(Test-Running 'ZoomText')"

Add-Section 'Rust'
Add-Line "rustc: $(Get-FirstLine 'rustc' @('--version'))"
Add-Line "cargo: $(Get-FirstLine 'cargo' @('--version'))"
if (Get-Command 'rustup' -ErrorAction SilentlyContinue) {
    Add-Line "rustup toolchain: $(Get-FirstLine 'rustup' @('show', 'active-toolchain'))"
    $targets = if ($DryRun) { '(would run: rustup target list --installed)' } else { (@(& rustup target list --installed 2>$null) -join ', ') }
    Add-Line "Installed targets: $targets"
} else {
    Add-Line 'rustup: not installed'
}

Add-Section 'textweaver'
if ($Tw) {
    $dir = Split-Path -Parent $Tw
    Add-Line "tw: $Tw"
    Add-Line "tw version: $(Get-FirstLine $Tw @('--version'))"
    $tv = Join-Path $dir 'textweaver.exe'
    if (Test-Path -LiteralPath $tv) { Add-Line "textweaver version: $(Get-FirstLine $tv @('--version'))" } else { Add-Line 'textweaver.exe: not found beside tw.exe' }
    $manifest = Join-Path $dir 'install-manifest.txt'
    if (Test-Path -LiteralPath $manifest) {
        $kind = (Select-String -LiteralPath $manifest -Pattern '^kind=(.*)$' | Select-Object -First 1).Matches.Groups[1].Value
        $ver = (Select-String -LiteralPath $manifest -Pattern '^version=(.*)$' | Select-Object -First 1).Matches.Groups[1].Value
        Add-Line "Installed by: $kind install, version $ver"
    }
    $userPath = (Get-Item -LiteralPath 'HKCU:\Environment').GetValue('Path', '', 'DoNotExpandEnvironmentNames')
    $onPath = [bool] (($userPath -split ';') | Where-Object { $_ -and [Environment]::ExpandEnvironmentVariables($_).TrimEnd('\') -ieq $dir.TrimEnd('\') })
    Add-Line "Folder on the user PATH: $onPath"

    Add-Line
    Add-Line 'Settings (tw settings path):'
    Add-Output $Tw @('settings', 'path')

    Add-Line
    Add-Line 'Speech engines (tw backends):'
    Add-Output $Tw @('backends')

    Add-Line
    Add-Line "Engine hosts beside tw.exe, in ${dir}:"
    foreach ($h in 'textweaver-eci-host.exe', 'textweaver-eci-host-x86.exe', 'textweaver-sapi-host.exe', 'textweaver-sapi-host-x86.exe', 'textweaver-dectalk-host.exe', 'textweaver-dectalk-host-x86.exe') {
        if (Test-Path -LiteralPath (Join-Path $dir $h)) { Add-Line "  ${h}: present" } else { Add-Line "  ${h}: missing" }
    }
    $dict = Join-Path $dir 'ibmtts-dictionaries'
    if (Test-Path -LiteralPath $dict) {
        Add-Line "Pronunciation dictionaries: present, $(@(Get-ChildItem -LiteralPath $dict -File).Count) files."
    } else {
        Add-Line 'Pronunciation dictionaries: missing (ibmtts-dictionaries beside tw.exe).'
    }
} else {
    Add-Line 'tw.exe was not found on your PATH, beside this script, or in this checkout.'
}

Add-Section 'Windows voices'
foreach ($c in @(
        @{ Name = 'SAPI5, 64-bit'; Key = 'HKLM:\SOFTWARE\Microsoft\Speech\Voices\Tokens' },
        @{ Name = 'SAPI5, 32-bit'; Key = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Speech\Voices\Tokens' },
        @{ Name = 'OneCore'; Key = 'HKLM:\SOFTWARE\Microsoft\Speech_OneCore\Voices\Tokens' })) {
    $n = 0
    if (Test-Path -LiteralPath $c.Key) { $n = @(Get-ChildItem -LiteralPath $c.Key).Count }
    Add-Line "$($c.Name): $n voices. (Run scripts\speech-check.ps1 for the names.)"
}

Add-Section 'Optional tools'
Add-Line "ffmpeg: $(Get-FirstLine 'ffmpeg' @('-version'))"
Add-Line "pandoc: $(Get-FirstLine 'pandoc' @('--version'))"
Add-Line "whisper-cli: $(if (Get-Command 'whisper-cli' -ErrorAction SilentlyContinue) { 'present' } else { 'not installed' })"
Add-Line "git: $(Get-FirstLine 'git' @('--version'))"

Add-Section 'textweaver environment variables that are set (names only)'
$names = @(Get-ChildItem -Path 'Env:' | Where-Object { $_.Name -like 'TEXTWEAVER_*' } | ForEach-Object { $_.Name } | Sort-Object)
if ($names.Count -gt 0) { foreach ($n in $names) { Add-Line "  $n" } } else { Add-Line '  none' }

Add-Line
Add-Line 'End of report.'

if ($Out) {
    if ($DryRun) {
        [Console]::Out.WriteLine("Would write this report to $Out")
    } else {
        $full = $Out
        if (-not [IO.Path]::IsPathRooted($full)) { $full = Join-Path (Get-Location).Path $full }
        $full = [IO.Path]::GetFullPath($full)
        try {
            [IO.File]::WriteAllLines($full, [string[]] $report)
        } catch {
            [Console]::Out.WriteLine("Error: could not write $(Hide-UserFolder $full): $($_.Exception.Message)")
            exit 1
        }
        [Console]::Out.WriteLine('')
        [Console]::Out.WriteLine("The report is also in $(Hide-UserFolder $full).")
    }
}
