<#
.SYNOPSIS
  A plain report on textweaver's speech engines and the Windows voices.

.DESCRIPTION
  Prints textweaver's version, its speech engines (tw backends), the first
  few voices of each available engine, and whether Eloquence was found
  (tw eloquence). Then it lists the installed SAPI5 voices (64-bit and
  32-bit) and OneCore voices from the registry, which loads no engine.

  -Probe goes further: it runs tools\sapi_probe.ps1 in 64-bit and 32-bit
  PowerShell, which synthesizes a test sentence with each voice into a
  temporary WAV file (nothing is played) to check its word events. That
  loads every voice's engine, so -SkipVoice leaves out the ones you name.

  Nothing is spoken unless you add -Speak. Works in Windows PowerShell 5.1
  and PowerShell 7. GNU-style options work too: --help, --dry-run, --speak,
  --probe, --skip ID.

.PARAMETER Speak
  Speak one short test sentence with textweaver's default engine.

.PARAMETER Probe
  Run the SAPI word-event probe (tools\sapi_probe.ps1) for each voice.

.PARAMETER SkipVoice
  With -Probe, a regular expression of voice names to leave out
  (default: Eloquence).

.PARAMETER Skip
  textweaver engine ids whose voices are not listed, such as eci. Listing
  voices starts the engine.

.PARAMETER Voices
  How many voices to show per engine (default: 5).

.PARAMETER Tw
  The tw.exe to check (default: tw on PATH, then a build in this checkout).

.PARAMETER DryRun
  Print each command instead of running it.

.PARAMETER Yes
  Accepted for consistency; this script asks nothing.

.PARAMETER Help
  Show the help.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\speech-check.ps1

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\speech-check.ps1 -Probe -SkipVoice 'Eloquence|OpenEVV'
#>
[CmdletBinding(PositionalBinding = $false)]
param(
    [switch] $Speak,
    [switch] $Probe,
    [string] $SkipVoice = 'Eloquence',
    [string[]] $Skip = @(),
    [int] $Voices = 5,
    [string] $Tw,
    [switch] $DryRun,
    [switch] $Yes,
    [switch] $Help,
    [Parameter(ValueFromRemainingArguments = $true)] [string[]] $Rest
)

$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'

function Write-Line([string] $Text = '') { [Console]::Out.WriteLine($Text) }

function Write-Section([string] $Title) {
    Write-Line
    Write-Line "== $Title =="
}

# Shows the home folder as ~, so the report does not include the user name.
function Format-UserPath([string] $Path) {
    if ($Path -and $env:USERPROFILE -and $Path.StartsWith($env:USERPROFILE, [StringComparison]::OrdinalIgnoreCase)) {
        return '~' + $Path.Substring($env:USERPROFILE.Length)
    }
    return $Path
}

# Replaces the home folder with ~ anywhere in a line of output.
function Hide-UserFolder([string] $Line) {
    if (-not $env:USERPROFILE) { return $Line }
    return [regex]::Replace($Line, [regex]::Escape($env:USERPROFILE), '~', 'IgnoreCase')
}

# Runs a read-only command and indents its output (at most $Limit lines).
function Invoke-Probe([string[]] $Command, [int] $Limit = 0) {
    $shown = ($Command | ForEach-Object { if ($_ -match '\s') { '"' + $_ + '"' } else { $_ } }) -join ' '
    if ($DryRun) {
        if ($Limit -gt 0) { Write-Line "Would run: $shown, and show the first $Limit lines" } else { Write-Line "Would run: $shown" }
        return 0
    }
    Write-Line "> $(Format-UserPath $shown)"
    $exe = $Command[0]
    $arguments = @()
    if ($Command.Count -gt 1) { $arguments = $Command[1..($Command.Count - 1)] }
    $lines = @(& $exe @arguments 2>&1 | ForEach-Object { "$_" })
    $code = $LASTEXITCODE
    $show = $lines
    if ($Limit -gt 0 -and $lines.Count -gt $Limit) { $show = $lines[0..($Limit - 1)] }
    foreach ($l in $show) { Write-Line "  $(Hide-UserFolder $l)" }
    if ($Limit -gt 0 -and $lines.Count -gt $Limit) {
        Write-Line "  ... and $($lines.Count - $Limit) more lines."
    }
    return $code
}

$i = 0
while ($null -ne $Rest -and $i -lt $Rest.Count) {
    $a = $Rest[$i]
    switch -Regex ($a) {
        '^(--help|-h|/\?|-\?)$' { $Help = $true }
        '^--dry-run$' { $DryRun = $true }
        '^(--yes|-y)$' { $Yes = $true }
        '^--speak$' { $Speak = $true }
        '^--probe$' { $Probe = $true }
        '^--skip$' { $i++; $Skip += $Rest[$i] }
        '^--skip-voice$' { $i++; $SkipVoice = $Rest[$i] }
        '^--voices$' { $i++; $Voices = [int] $Rest[$i] }
        '^--tw$' { $i++; $Tw = $Rest[$i] }
        default { Write-Line "Error: unknown option $a. Run with -Help to see the options."; exit 2 }
    }
    $i++
}
$Skip = @($Skip | ForEach-Object { $_ -split ',' } | Where-Object { $_ })

if ($Help) {
    Write-Line 'Usage: scripts\speech-check.ps1 [-Speak] [-Probe [-SkipVoice REGEX]] [-Skip ID,...] [-Voices N] [-Tw PATH] [-DryRun]'
    Write-Line
    Write-Line "Prints a plain report on speech: textweaver's version, its engines (tw backends),"
    Write-Line 'the first voices of each available engine, tw eloquence, and the SAPI5 and OneCore'
    Write-Line 'voices installed on Windows. Nothing is spoken unless you add -Speak.'
    Write-Line
    Write-Line 'Options:'
    Write-Line '  -Speak            Speak one short test sentence (off by default).'
    Write-Line '  -Probe            Check word events of each SAPI voice with tools\sapi_probe.ps1.'
    Write-Line '                    It writes temporary WAV files and plays nothing, but it loads'
    Write-Line '                    every voice engine.'
    Write-Line '  -SkipVoice REGEX  With -Probe, voices to leave out (default: Eloquence).'
    Write-Line '  -Skip ID,...      textweaver engines whose voices are not listed, such as eci.'
    Write-Line '  -Voices N         Voices to show per engine (default: 5).'
    Write-Line '  -Tw PATH          The tw.exe to check.'
    Write-Line '  -DryRun           Print each command instead of running it.'
    Write-Line '  -Help             Show this help. GNU-style spellings (--help, --speak) work too.'
    exit 0
}

$RepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))

# tw: -Tw, PATH, this install's folder, then this checkout's builds.
if (-not $Tw) {
    $cmd = Get-Command 'tw.exe' -ErrorAction SilentlyContinue
    if ($cmd) {
        $Tw = $cmd.Source
    } else {
        $candidates = @(
            (Join-Path $PSScriptRoot '..\tw.exe'),
            (Join-Path $RepoRoot 'target\release\tw.exe'),
            (Join-Path $RepoRoot 'target\debug\tw.exe')
        )
        $found = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
        if ($found) { $Tw = [IO.Path]::GetFullPath($found) }
    }
}

Write-Line 'textweaver speech check'
Write-Line 'This report is read-only. It speaks only with -Speak.'
if ($DryRun) { Write-Line 'Dry run: each command is printed instead of run.' }

Write-Section 'textweaver'
if ($Tw) {
    Write-Line "tw: $(Format-UserPath $Tw)"
    Invoke-Probe @($Tw, '--version') | Out-Null

    Write-Section 'Speech engines (tw backends)'
    Invoke-Probe @($Tw, 'backends') | Out-Null

    Write-Section 'Voices'
    if ($DryRun) {
        Write-Line "Would run: $Tw voices --backend ID, for each available engine except null, and show the first $Voices voices"
    } else {
        $available = @(& $Tw backends 2>$null | ForEach-Object {
                if ($_ -match '^([A-Za-z0-9_-]+): .*\. Available\.') { $Matches[1] }
            })
        $shown = 0
        foreach ($id in $available) {
            if ($id -eq 'null') { continue }
            if ($Skip -contains $id) { Write-Line "Engine ${id}: skipped, as asked."; continue }
            $shown++
            Write-Line "Engine ${id}:"
            $code = Invoke-Probe @($Tw, 'voices', '--backend', $id) $Voices
            if ($code -ne 0) { Write-Line "  Could not list the voices of $id." }
        }
        if ($shown -eq 0) { Write-Line 'No speech engine is listed apart from null (silence).' }
    }

    Write-Section 'Eloquence (tw eloquence)'
    Invoke-Probe @($Tw, 'eloquence') | Out-Null
} else {
    Write-Line 'tw.exe was not found on your PATH, beside this script, or in this checkout. Install textweaver (scripts\install-windows.ps1), or pass -Tw PATH.'
}

Write-Section 'Windows voices (from the registry; no engine is loaded)'
$categories = @(
    @{ Name = 'SAPI5, 64-bit'; Key = 'HKLM:\SOFTWARE\Microsoft\Speech\Voices\Tokens' },
    @{ Name = 'SAPI5, 32-bit'; Key = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Speech\Voices\Tokens' },
    @{ Name = 'SAPI5, this user'; Key = 'HKCU:\SOFTWARE\Microsoft\Speech\Voices\Tokens' },
    @{ Name = 'OneCore'; Key = 'HKLM:\SOFTWARE\Microsoft\Speech_OneCore\Voices\Tokens' }
)
foreach ($c in $categories) {
    if ($DryRun) { Write-Line "Would list the voice names under $($c.Key)"; continue }
    $names = @()
    if (Test-Path -LiteralPath $c.Key) {
        $names = @(Get-ChildItem -LiteralPath $c.Key | ForEach-Object {
                $n = $_.GetValue('')
                if (-not $n) { $n = $_.PSChildName }
                $n
            })
    }
    Write-Line "$($c.Name): $($names.Count) voices."
    foreach ($n in $names) { Write-Line "  $n" }
}
if (-not $DryRun) {
    $default = (Get-ItemProperty -LiteralPath 'HKCU:\SOFTWARE\Microsoft\Speech\Voices' -ErrorAction SilentlyContinue).DefaultTokenId
    if ($default) { Write-Line "Default SAPI5 voice: $($default -replace '^.*\\', '')." }
}

if ($Probe) {
    Write-Section 'SAPI word-event probe'
    $probeScript = @((Join-Path $RepoRoot 'tools\sapi_probe.ps1'), (Join-Path $PSScriptRoot 'sapi_probe.ps1')) |
        Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $probeScript) {
        Write-Line 'tools\sapi_probe.ps1 was not found, so the probe cannot run.'
    } else {
        Write-Line "Each voice speaks a test sentence into a temporary WAV file; nothing is played. Voices matching '$SkipVoice' are skipped."
        $work = Join-Path ([IO.Path]::GetTempPath()) ('textweaver-speech-check-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
        $shells = @(
            @{ Name = '64-bit'; Exe = (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') },
            @{ Name = '32-bit'; Exe = (Join-Path $env:SystemRoot 'SysWOW64\WindowsPowerShell\v1.0\powershell.exe') }
        )
        foreach ($s in $shells) {
            if (-not (Test-Path -LiteralPath $s.Exe)) { Write-Line "$($s.Name) PowerShell is not available."; continue }
            Write-Line "$($s.Name) voices:"
            if ($DryRun) {
                Write-Line "Would run: $($s.Exe) -NoProfile -ExecutionPolicy Bypass -File $probeScript -Out TEMPFOLDER -Skip '$SkipVoice'"
                continue
            }
            New-Item -ItemType Directory -Force -Path $work | Out-Null
            $out = @(& $s.Exe -NoProfile -ExecutionPolicy Bypass -File $probeScript -Out $work -Skip $SkipVoice 2>&1 | ForEach-Object { "$_" })
            # One line per voice: the lines that start with "==".
            foreach ($l in $out) { if ($l -match '^== ') { Write-Line "  $($l.Substring(3))" } }
        }
        if (Test-Path -LiteralPath $work) { Remove-Item -LiteralPath $work -Recurse -Force }
    }
}

if ($Speak) {
    Write-Section 'Speaking a test sentence'
    if ($Tw) {
        Invoke-Probe @($Tw, 'speak', 'This is the textweaver speech check. If you can hear this, speech works.') | Out-Null
    } else {
        Write-Line 'tw.exe was not found, so nothing can be spoken.'
    }
}

Write-Section 'End of report'
Write-Line 'Copy this report into a bug report if speech does not work. It holds no personal data: your home folder is shown as ~.'
