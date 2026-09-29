<#
.SYNOPSIS
  Runs the checks CI runs, locally on Windows.

.DESCRIPTION
  Steps, in order:
    fmt       cargo fmt --all --check
    clippy    cargo clippy --workspace --all-targets
              --features textweaver-speech/omnivox -- -D warnings
    test      cargo test --workspace
              --features textweaver-speech/omnivox
    doc       cargo doc --workspace --no-deps
              --features textweaver-speech/omnivox, with RUSTDOCFLAGS=-D warnings
    keyboard  cargo xtask keyboard --check
    pseudo    cargo test -p textweaver-app --test pseudo_locale (every
              message comes from the catalog: bracketed in en-XA, direction
              marks closed in ar-XB)
    hosts32   cargo build -p textweaver-eci -p textweaver-sapi --bins
              --target i686-pc-windows-msvc (the 32-bit engine hosts)
    scripts   PSScriptAnalyzer on scripts\*.ps1, when the module is installed

  The espeak feature needs libespeak-ng, which Windows does not have, so the
  checks use the omnivox feature instead of --all-features (the agent briefs).
  -Docker runs scripts/dev-check.sh in the development container for the
  full Linux set.

  Every step runs, and a summary says which passed; the exit code is 1 when
  any failed. Works in Windows PowerShell 5.1 and PowerShell 7. GNU-style
  options work too: --help, --dry-run, --only fmt,clippy, --fail-fast,
  --docker.

.PARAMETER Only
  Run only these steps, such as fmt,clippy.

.PARAMETER FailFast
  Stop at the first failing step.

.PARAMETER Docker
  Run scripts/dev-check.sh in the Docker development container instead.

.PARAMETER DryRun
  Print each command instead of running it.

.PARAMETER Yes
  Accepted for consistency; this script asks nothing.

.PARAMETER Help
  Show the help.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\dev-check.ps1

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\dev-check.ps1 -Only fmt,clippy
#>
[CmdletBinding(PositionalBinding = $false)]
param(
    [string[]] $Only = @(),
    [switch] $FailFast,
    [switch] $Docker,
    [switch] $DryRun,
    [switch] $Yes,
    [switch] $Help,
    [Parameter(ValueFromRemainingArguments = $true)] [string[]] $Rest
)

$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'

function Write-Line([string] $Text = '') { [Console]::Out.WriteLine($Text) }

$i = 0
while ($null -ne $Rest -and $i -lt $Rest.Count) {
    $a = $Rest[$i]
    switch -Regex ($a) {
        '^(--help|-h|/\?|-\?)$' { $Help = $true }
        '^--dry-run$' { $DryRun = $true }
        '^(--yes|-y)$' { $Yes = $true }
        '^--fail-fast$' { $FailFast = $true }
        '^--docker$' { $Docker = $true }
        '^--only$' { $i++; $Only += $Rest[$i] }
        default { Write-Line "Error: unknown option $a. Run with -Help to see the options."; exit 2 }
    }
    $i++
}
$Only = @($Only | ForEach-Object { $_ -split ',' } | Where-Object { $_ })

if ($Help) {
    Write-Line 'Usage: scripts\dev-check.ps1 [-Only STEP,...] [-FailFast] [-Docker] [-DryRun] [-Help]'
    Write-Line
    Write-Line 'Runs the checks CI runs: fmt, clippy, test, doc (rustdoc with -D warnings),'
    Write-Line 'keyboard (cargo xtask keyboard --check), pseudo (the pseudo-locale check),'
    Write-Line 'hosts32 (the 32-bit engine hosts),'
    Write-Line 'links (tools\check_links.py: links and anchors in the docs resolve), site'
    Write-Line '(tools\gen_site_data.py --check: the docs\site data is current), site-a11y'
    Write-Line '(tools\check_site_a11y.py: static accessibility checks of docs\site), and'
    Write-Line 'scripts (PSScriptAnalyzer, when installed). Windows uses'
    Write-Line '--features textweaver-speech/omnivox instead of --all-features.'
    Write-Line
    Write-Line 'Options:'
    Write-Line '  -Only STEP,...  Run only these steps.'
    Write-Line '  -FailFast       Stop at the first failing step.'
    Write-Line '  -Docker         Run scripts/dev-check.sh in the development container instead.'
    Write-Line '  -DryRun         Print each command instead of running it.'
    Write-Line '  -Help           Show this help.'
    exit 0
}

$Root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $Root
$env:CARGO_TERM_PROGRESS_WHEN = 'never'
$env:RUSTUP_TERM_PROGRESS_WHEN = 'never'
if ($env:NO_COLOR) { $env:CARGO_TERM_COLOR = 'never'; $env:RUSTUP_TERM_COLOR = 'never' }

if ($Docker) {
    $shArgs = @()
    if ($Only.Count -gt 0) { $shArgs += @('--only', ($Only -join ',')) }
    if ($FailFast) { $shArgs += '--fail-fast' }
    # The image sets CARGO_TERM_COLOR=always; ask for plain output instead.
    $color = 'auto'
    if ($env:NO_COLOR) { $color = 'never' }
    $cmd = @('compose', '-p', 'textweaver', 'run', '--rm', '-T', '-e', 'CARGO_TARGET_DIR=/target/dev-check', '-e', 'NO_COLOR', '-e', "CARGO_TERM_COLOR=$color", 'dev', 'bash', 'scripts/dev-check.sh') + $shArgs
    Write-Line 'Running the checks in the development container (docker compose run dev).'
    if ($DryRun) { Write-Line "Would run: docker $($cmd -join ' ')"; exit 0 }
    Write-Line "Running: docker $($cmd -join ' ')"
    & docker @cmd
    exit $LASTEXITCODE
}

$features = @('--features', 'textweaver-speech/omnivox')
$script:passed = @()
$script:failed = @()
$script:skipped = @()

function Test-Wanted([string] $Name) {
    return ($Only.Count -eq 0 -or $Only -contains $Name)
}

function Write-Summary {
    Write-Line
    Write-Line '== Summary =='
    if ($script:passed.Count) { Write-Line "Passed: $($script:passed -join ', ')." } else { Write-Line 'Passed: none.' }
    if ($script:failed.Count) { Write-Line "Failed: $($script:failed -join ', ')." } else { Write-Line 'Failed: none.' }
    if ($script:skipped.Count) { Write-Line "Skipped: $($script:skipped -join ', ')." }
}

# Runs one step: a program and its arguments, with optional environment.
function Invoke-CheckStep([string] $Name, [string] $What, [string[]] $Command, [hashtable] $Environment = @{}) {
    if (-not (Test-Wanted $Name)) { return }
    Write-Line
    Write-Line "== ${Name}: $What =="
    $envText = ($Environment.Keys | ForEach-Object { "$_=`"$($Environment[$_])`" " }) -join ''
    $shown = $envText + ($Command -join ' ')
    if ($DryRun) { Write-Line "Would run: $shown"; return }
    Write-Line "Running: $shown"
    $saved = @{}
    foreach ($k in $Environment.Keys) {
        $saved[$k] = [Environment]::GetEnvironmentVariable($k)
        [Environment]::SetEnvironmentVariable($k, $Environment[$k])
    }
    $sw = [Diagnostics.Stopwatch]::StartNew()
    try {
        $exe = $Command[0]
        $arguments = @()
        if ($Command.Count -gt 1) { $arguments = $Command[1..($Command.Count - 1)] }
        & $exe @arguments
        $code = $LASTEXITCODE
    } finally {
        foreach ($k in $saved.Keys) { [Environment]::SetEnvironmentVariable($k, $saved[$k]) }
    }
    $seconds = [int] $sw.Elapsed.TotalSeconds
    if ($code -eq 0) {
        Write-Line "$Name passed, in $seconds seconds."
        $script:passed += $Name
    } else {
        Write-Line "$Name FAILED (exit code $code), after $seconds seconds."
        $script:failed += $Name
        if ($FailFast) { Write-Summary; exit 1 }
    }
}

Write-Line "Running the CI checks in $Root."
Write-Line "Features: $($features -join ' ') (the espeak feature needs libespeak-ng, which Windows does not have)."

Invoke-CheckStep 'fmt' 'formatting' @('cargo', 'fmt', '--all', '--check')
Invoke-CheckStep 'clippy' 'lints, warnings are errors' (@('cargo', 'clippy', '--workspace', '--all-targets') + $features + @('--', '-D', 'warnings'))
Invoke-CheckStep 'test' 'tests' (@('cargo', 'test', '--workspace') + $features)
Invoke-CheckStep 'doc' 'API documentation, warnings are errors' (@('cargo', 'doc', '--workspace', '--no-deps') + $features) @{ RUSTDOCFLAGS = '-D warnings' }
Invoke-CheckStep 'keyboard' 'docs/keyboard.md is current' @('cargo', 'xtask', 'keyboard', '--check')
Invoke-CheckStep 'pseudo' 'the interface in the pseudo-locales en-XA and ar-XB' @('cargo', 'test', '-p', 'textweaver-app', '--test', 'pseudo_locale')
$python = $null
foreach ($candidate in @('python', 'python3')) {
    if (Get-Command $candidate -ErrorAction SilentlyContinue) { $python = $candidate; break }
}
foreach ($pyStep in @('links', 'site', 'site-a11y')) {
    if (-not (Test-Wanted $pyStep)) { continue }
    if (-not $python) {
        Write-Line
        Write-Line "== ${pyStep}: skipped =="
        Write-Line "Python 3 is not installed, so the $pyStep check cannot run."
        $script:skipped += $pyStep
        continue
    }
    if ($pyStep -eq 'links') {
        Invoke-CheckStep 'links' 'links and anchors in the docs resolve' @($python, 'tools\check_links.py')
    } elseif ($pyStep -eq 'site') {
        Invoke-CheckStep 'site' 'the docs\site data is current' @($python, 'tools\gen_site_data.py', '--check')
    } else {
        Invoke-CheckStep 'site-a11y' 'static accessibility checks of docs\site' @($python, 'tools\check_site_a11y.py')
    }
}
Invoke-CheckStep 'hosts32' 'the 32-bit engine hosts build' @('cargo', 'build', '-p', 'textweaver-eci', '-p', 'textweaver-sapi', '--bins', '--target', 'i686-pc-windows-msvc')

if (Test-Wanted 'scripts') {
    if (Get-Module -ListAvailable -Name PSScriptAnalyzer) {
        Write-Line
        Write-Line '== scripts: PSScriptAnalyzer on scripts\*.ps1, warnings and errors =='
        if ($DryRun) {
            Write-Line 'Would run: Invoke-ScriptAnalyzer -Path scripts -Recurse -Severity Warning,Error'
        } else {
            $findings = @(Invoke-ScriptAnalyzer -Path (Join-Path $Root 'scripts') -Recurse -Severity Warning, Error)
            foreach ($f in $findings) { Write-Line "$($f.ScriptName) line $($f.Line): $($f.RuleName): $($f.Message)" }
            if ($findings.Count -eq 0) { Write-Line 'scripts passed.'; $script:passed += 'scripts' } else { Write-Line "scripts FAILED: $($findings.Count) findings."; $script:failed += 'scripts' }
        }
    } else {
        Write-Line
        Write-Line '== scripts: skipped =='
        Write-Line 'PSScriptAnalyzer is not installed. Install it with: Install-Module PSScriptAnalyzer -Scope CurrentUser'
        $script:skipped += 'scripts'
    }
}

if ($DryRun) {
    Write-Line
    Write-Line 'Dry run finished. Nothing was run.'
    exit 0
}
Write-Summary
if ($script:failed.Count -gt 0) { exit 1 }
exit 0
