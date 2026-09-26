<#
.SYNOPSIS
  Converts a folder of Markdown (or other documents) with tw convert.

.DESCRIPTION
  Converts every document in the folder, and its subfolders, to HTML (the
  default), EPUB, PDF, DOCX, braille (brf), text, or Markdown. The output
  goes to a folder beside it named after the format, so notes becomes
  notes-html, with the subfolders mirrored. Files already up to date are
  skipped, so running it again converts only what changed.

  -TwArgs passes more options to tw convert (run tw convert --help). When
  the script is run with & inside PowerShell, arguments after -- work too.
  Works in Windows PowerShell 5.1 and PowerShell 7. GNU-style options work
  too: --to, --out, --watch, --force, --dry-run, --help.

.PARAMETER Folder
  The folder to convert.

.PARAMETER To
  html (the default), epub, pdf, docx, brf, txt, or md.

.PARAMETER Out
  The output folder (default: FOLDER-FORMAT beside the folder).

.PARAMETER Watch
  Keep watching the folder and convert new files as they arrive.

.PARAMETER Force
  Convert every file, even when its output is newer.

.PARAMETER TwArgs
  More tw convert options, as one string, such as "--flavor obsidian --smart".

.PARAMETER Tw
  The tw.exe to use (default: tw on PATH, then a build in this checkout).

.PARAMETER DryRun
  Print the tw convert command instead of running it.

.PARAMETER Yes
  Accepted for consistency; this script asks nothing.

.PARAMETER Help
  Show the help.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\convert-folder.ps1 -Folder notes

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\convert-folder.ps1 notes -To epub -TwArgs "--flavor obsidian"
#>
[CmdletBinding(PositionalBinding = $false)]
param(
    [string] $Folder,
    [string] $To = 'html',
    [string] $Out,
    [switch] $Watch,
    [switch] $Force,
    [string] $TwArgs,
    [string] $Tw,
    [switch] $DryRun,
    [switch] $Yes,
    [switch] $Help,
    [Parameter(ValueFromRemainingArguments = $true)] [string[]] $Rest
)

$ErrorActionPreference = 'Stop'

function Write-Line([string] $Text = '') { [Console]::Out.WriteLine($Text) }

$extra = @()
$i = 0
while ($null -ne $Rest -and $i -lt $Rest.Count) {
    $a = $Rest[$i]
    if ($a -eq '--') {
        if ($i + 1 -lt $Rest.Count) { $extra = $Rest[($i + 1)..($Rest.Count - 1)] }
        break
    }
    switch -Regex ($a) {
        '^(--help|-h|/\?|-\?)$' { $Help = $true }
        '^--dry-run$' { $DryRun = $true }
        '^(--yes|-y)$' { $Yes = $true }
        '^--watch$' { $Watch = $true }
        '^--force$' { $Force = $true }
        '^--to$' { $i++; $To = $Rest[$i] }
        '^--out$' { $i++; $Out = $Rest[$i] }
        '^--tw$' { $i++; $Tw = $Rest[$i] }
        default {
            if ($a -like '-*') { Write-Line "Error: unknown option $a. Run with -Help to see the options."; exit 2 }
            if ($Folder) { Write-Line 'Error: give one folder. To convert several, run the script once for each.'; exit 2 }
            $Folder = $a
        }
    }
    $i++
}

if ($Help) {
    Write-Line 'Usage: scripts\convert-folder.ps1 FOLDER [-To FORMAT] [-Out DIR] [-Watch] [-Force] [-TwArgs "..."] [-DryRun]'
    Write-Line
    Write-Line 'Converts every document in FOLDER, and its subfolders, with tw convert. The output'
    Write-Line 'goes beside it, in FOLDER-FORMAT (notes becomes notes-html). Up-to-date files are'
    Write-Line 'skipped, so running it again converts only what changed.'
    Write-Line
    Write-Line 'Options:'
    Write-Line '  -To FORMAT  html (the default), epub, pdf, docx, brf (braille), txt, or md.'
    Write-Line '              If your tw has no writer for a format yet, it says so.'
    Write-Line '  -Out DIR    The output folder.'
    Write-Line '  -Watch      Keep watching the folder. Press Control+C to stop.'
    Write-Line '  -Force      Convert every file, even when its output is newer.'
    Write-Line '  -Tw PATH    The tw.exe to use.'
    Write-Line '  -DryRun     Print the tw convert command instead of running it.'
    Write-Line '  -Help       Show this help.'
    Write-Line '  -TwArgs "..." More tw convert options, such as -TwArgs "--flavor obsidian --smart".'
    exit 0
}

if (-not $Folder) { Write-Line 'Error: name the folder to convert. Run with -Help for examples.'; exit 2 }
if (-not (Test-Path -LiteralPath $Folder -PathType Container)) { Write-Line "Error: $Folder is not a folder."; exit 2 }
switch ($To.ToLower()) {
    'markdown' { $To = 'md' }
    'text' { $To = 'txt' }
}
if (@('html', 'epub', 'pdf', 'docx', 'brf', 'txt', 'md') -notcontains $To.ToLower()) {
    Write-Line "Error: unknown format $To. Use html, epub, pdf, docx, brf, txt, or md."
    exit 2
}
$To = $To.ToLower()
$Folder = $Folder.TrimEnd('\', '/')
if (-not $Out) { $Out = "$Folder-$To" }

if (-not $Tw) {
    $cmd = Get-Command 'tw.exe' -ErrorAction SilentlyContinue
    if ($cmd) {
        $Tw = $cmd.Source
    } else {
        $root = Join-Path $PSScriptRoot '..'
        $Tw = @((Join-Path $PSScriptRoot '..\tw.exe'), (Join-Path $root 'target\release\tw.exe'), (Join-Path $root 'target\debug\tw.exe')) |
            Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    }
}
if (-not $Tw) {
    if (-not $DryRun) { Write-Line 'Error: tw.exe was not found. Install textweaver, or pass -Tw PATH.'; exit 1 }
    $Tw = 'tw'
}

$arguments = @('convert', $Folder, '--to', $To, '--out', $Out)
if ($Watch) { $arguments += '--watch' }
if ($Force) { $arguments += '--force' }
if ($TwArgs) { $arguments += @($TwArgs -split '\s+' | Where-Object { $_ }) }
$arguments += $extra

if ($Watch) {
    Write-Line "Watching $Folder. New and changed documents are converted to $To in $Out. Press Control+C to stop."
} else {
    Write-Line "Converting the documents in $Folder to $To, into $Out."
}
$shown = (@($Tw) + $arguments | ForEach-Object { if ($_ -match '\s') { '"' + $_ + '"' } else { $_ } }) -join ' '
if ($DryRun) { Write-Line "Would run: $shown"; exit 0 }
Write-Line "Running: $shown"
& $Tw @arguments
exit $LASTEXITCODE
