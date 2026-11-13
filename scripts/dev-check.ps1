<#
.SYNOPSIS
  Runs the checks CI runs, locally on Windows.

.DESCRIPTION
  Steps, in order. Every cargo call has --locked, and RUSTFLAGS is
  -D warnings for every step, as CI sets it:
    fmt       cargo fmt --all --check
    clippy    cargo clippy --workspace --all-targets
              --features textweaver-speech/omnivox -- -D warnings
    lean      cargo build -p textweaver-tui --no-default-features (the lean
              terminal reader), in its own target subfolder, lean
    test      cargo test --workspace --no-fail-fast
              --features textweaver-speech/omnivox, between the sentinel's
              record and check (cargo xtask sentinel; TEXTWEAVER_HOME is an
              empty folder in the target folder during the tests)
    sentinel  the sentinel's check: no test wrote to the real data folders
    doc       cargo doc --workspace --no-deps
              --features textweaver-speech/omnivox, with RUSTDOCFLAGS=-D warnings
    pseudo    cargo test -p textweaver-app --test it -- pseudo_locale:: (every
              message comes from the catalog: bracketed in en-XA, direction
              marks closed in ar-XB)
    generated cargo xtask regen --check (every generated file is current:
              the notices, the settings reference, docs\keyboard.md, the
              docs\site data, and the docs indexes and crate counts; one
              line each; cargo xtask regen rebuilds them). -Only keyboard
              or -Only site selects this step.
    deps      cargo xtask deps --check (the dependency direction between
              the workspace crates)
    parity    cargo xtask parity (word and sentence parity with star)
    deny      cargo deny check (licenses, advisories, duplicates, sources),
              skipped when cargo-deny is not installed
    links     py -3 tools\check_links.py (relative links and anchors in the
              docs resolve), skipped when Python 3 is not installed
    site-a11y py -3 tools\check_site_a11y.py (static accessibility checks
              of the docs\site pages), skipped when Python 3 is not installed
    hosts32   cargo build -p textweaver-host -p textweaver-eci
              -p textweaver-sapi -p textweaver-dectalk -p textweaver-espeak
              --bins --target i686-pc-windows-msvc (the combined 32-bit
              host, textweaver-host-x86.exe in the packages, and the
              separate development hosts the 32-bit tests use)
    scripts   PSScriptAnalyzer on scripts\*.ps1, when the module is installed

  The espeak feature needs libespeak-ng, which Windows does not have, so the
  checks use the omnivox feature instead of --all-features (the agent briefs).
  This matches CI's Windows leg, not its Linux leg: a green run here does
  not cover CI's Linux --all-features build and tests (espeak, speechd).
  -Docker runs scripts/dev-check.sh in the development container for the
  full Linux set.

  -Fast is the quicker check for one branch (docs\dev\testing.md, "Check
  tiers"): cargo xtask affected --base main names the crates changed since
  the merge base and every crate that depends on them, and fmt, clippy,
  the tests (cargo nextest when installed, else cargo test) and the doc
  tests run on those only. The lean step runs when textweaver-app,
  textweaver-tui or textweaver-components is in the set, and the generated
  and deps steps when the change touches the docs, the locale catalogs, the
  settings schema, the keymap or a Cargo.toml. It is not the full check:
  run that once per batch.

  Every step runs, and a summary says which passed; the exit code is 1 when
  any failed. Works in Windows PowerShell 5.1 and PowerShell 7. GNU-style
  options work too: --help, --dry-run, --only fmt,clippy, --fail-fast,
  --docker, --fast, --base REF.

.PARAMETER Only
  Run only these steps, such as fmt,clippy.

.PARAMETER FailFast
  Stop at the first failing step.

.PARAMETER Fast
  Check only the crates changed since the merge base of -Base, and the
  crates that depend on them.

.PARAMETER Base
  The branch -Fast compares with; main by default.

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
    [switch] $Fast,
    [string] $Base = 'main',
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
        '^--fast$' { $Fast = $true }
        '^--base$' { $i++; $Base = $Rest[$i] }
        '^--only$' { $i++; $Only += $Rest[$i] }
        default { Write-Line "Error: unknown option $a. Run with -Help to see the options."; exit 2 }
    }
    $i++
}
$Only = @($Only | ForEach-Object { $_ -split ',' } | Where-Object { $_ })

if ($Help) {
    Write-Line 'Usage: scripts\dev-check.ps1 [-Fast [-Base REF]] [-Only STEP,...] [-FailFast] [-Docker] [-DryRun] [-Help]'
    Write-Line
    Write-Line 'Runs the checks CI runs: fmt, clippy, lean (the lean terminal reader builds),'
    Write-Line 'test (between the sentinel record and check), sentinel (no test wrote to the'
    Write-Line 'real data folders), doc (rustdoc with -D warnings), pseudo (the pseudo-locale'
    Write-Line 'check), generated (cargo xtask regen --check: every generated file is current),'
    Write-Line 'deps (cargo xtask deps --check), parity (cargo xtask parity), deny (cargo deny'
    Write-Line 'check, when installed), links (tools\check_links.py), site-a11y'
    Write-Line '(tools\check_site_a11y.py), hosts32 (the 32-bit engine hosts), and scripts'
    Write-Line '(PSScriptAnalyzer, when installed). Every cargo call has --locked, tests have'
    Write-Line '--no-fail-fast, and RUSTFLAGS is -D warnings, as in CI.'
    Write-Line
    Write-Line 'Not covered: Windows uses --features textweaver-speech/omnivox instead of'
    Write-Line '--all-features, so a green run here does not cover the Linux --all-features'
    Write-Line 'leg of CI (espeak, speechd). Use -Docker for that.'
    Write-Line
    Write-Line 'Options:'
    Write-Line '  -Fast           Only the crates changed since the merge base of -Base, and'
    Write-Line '                  the crates that depend on them: fmt, clippy, the tests'
    Write-Line '                  (nextest when installed), doc tests; lean and generated'
    Write-Line '                  when the change calls for them. Not the full check.'
    Write-Line '  -Base REF       The branch -Fast compares with (default main).'
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
# As in CI, which sets it for every step: a warning that only rustc reports,
# in a test or doc build, fails here too.
$env:RUSTFLAGS = '-D warnings'
$env:RUSTUP_TERM_PROGRESS_WHEN = 'never'
if ($env:NO_COLOR) { $env:CARGO_TERM_COLOR = 'never'; $env:RUSTUP_TERM_COLOR = 'never' }

if ($Docker) {
    $shArgs = @()
    if ($Only.Count -gt 0) { $shArgs += @('--only', ($Only -join ',')) }
    if ($FailFast) { $shArgs += '--fail-fast' }
    if ($Fast) { $shArgs += @('--fast', '--base', $Base) }
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
    if ($Only.Count -eq 0 -or $Only -contains $Name) { return $true }
    # The keyboard and site checks are part of the generated step now.
    return ($Name -eq 'generated' -and ($Only -contains 'keyboard' -or $Only -contains 'site'))
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
Write-Line 'A green run here does not cover the Linux --all-features leg of CI; -Docker does.'

# The target folder: the lean build and the sentinel's files go inside it.
$targetDir = $env:CARGO_TARGET_DIR
if (-not $targetDir) { $targetDir = Join-Path $Root 'target' }
$targetDir = [IO.Path]::GetFullPath($targetDir)
$leanTarget = Join-Path $targetDir 'lean'
$sentinelFile = Join-Path $targetDir 'dev-check-sentinel.txt'
$sentinelHome = Join-Path $targetDir 'dev-check-sentinel-home'

function Add-Skipped([string] $Name, [string] $Why) {
    if (-not (Test-Wanted $Name)) { return }
    Write-Line
    Write-Line "== ${Name}: skipped =="
    Write-Line $Why
    $script:skipped += $Name
}

# Runs the tests between the sentinel's record and check.
function Invoke-TestsWithSentinel([string[]] $Command, [string] $What) {
    if (-not (Test-Wanted 'test')) { return }
    $recorded = $false
    if ($DryRun) {
        Write-Line
        Write-Line "Would run: TEXTWEAVER_HOME=`"$sentinelHome`" cargo --locked xtask sentinel record $sentinelFile"
        $recorded = $true
    } else {
        Write-Line
        Write-Line '== sentinel: listing the real data folders before the tests =='
        if (-not (Test-Path -LiteralPath $sentinelHome)) { New-Item -ItemType Directory -Path $sentinelHome | Out-Null }
        $savedHome = $env:TEXTWEAVER_HOME
        $env:TEXTWEAVER_HOME = $sentinelHome
        & cargo --locked xtask sentinel record $sentinelFile
        $recorded = ($LASTEXITCODE -eq 0)
        [Environment]::SetEnvironmentVariable('TEXTWEAVER_HOME', $savedHome)
        if (-not $recorded) { Write-Line 'The sentinel could not list the folders, so its check is left out.' }
    }
    Invoke-CheckStep 'test' $What $Command @{ TEXTWEAVER_ESPEAK_OUTPUT = 'virtual'; TEXTWEAVER_HOME = $sentinelHome }
    if ($recorded) {
        Invoke-CheckStep 'sentinel' 'no test touched the real data folders' @('cargo', '--locked', 'xtask', 'sentinel', 'check', $sentinelFile)
    } elseif (-not $DryRun) {
        $script:failed += 'sentinel'
    }
}

$leanStep = @('cargo', 'build', '--locked', '-p', 'textweaver-tui', '--no-default-features')

if ($Fast) {
    Write-Line
    Write-Line "== affected: the crates changed since the merge base of $Base, and their dependents =="
    $set = @(& cargo --locked xtask affected --base $Base | Where-Object { $_ })
    if ($LASTEXITCODE -ne 0) { Write-Line "affected FAILED: cargo xtask affected could not list the crates."; $script:failed += 'affected'; Write-Summary; exit 1 }
    $pkgArgs = @()
    $line = & cargo --locked xtask affected --base $Base --args --feature textweaver-speech/omnivox
    if ($line) { $pkgArgs = @("$line" -split ' ' | Where-Object { $_ }) }
    $generatedToo = ((& cargo --locked xtask affected --base $Base --generated) -eq 'yes')
    Write-Line "$($set.Count) crates: $($set -join ', ')."
    if ($set.Count -eq 0) {
        foreach ($n in @('fmt', 'clippy', 'test', 'doctest')) { Add-Skipped $n "No workspace crate changed since the merge base of $Base." }
    } else {
        $fmtArgs = @($set | ForEach-Object { @('-p', $_) })
        Invoke-CheckStep 'fmt' 'formatting of the changed crates' (@('cargo', 'fmt', '--check') + $fmtArgs)
        Invoke-CheckStep 'clippy' 'lints of the changed crates, warnings are errors' (@('cargo', 'clippy', '--locked') + $pkgArgs + @('--all-targets', '--', '-D', 'warnings'))
        $nextest = $false
        if (-not $DryRun) { & cargo nextest --version *> $null; $nextest = ($LASTEXITCODE -eq 0) } else { $nextest = $true }
        if ($nextest) {
            Invoke-TestsWithSentinel (@('cargo', 'nextest', 'run', '--locked', '--no-fail-fast') + $pkgArgs) 'tests of the changed crates (cargo nextest)'
        } else {
            Write-Line 'cargo-nextest is not installed, so the tests run with cargo test.'
            Invoke-TestsWithSentinel (@('cargo', 'test', '--locked', '--no-fail-fast') + $pkgArgs) 'tests of the changed crates (cargo test)'
        }
        # nextest does not run doc tests; cargo test --doc does.
        Invoke-CheckStep 'doctest' 'doc tests of the changed crates' (@('cargo', 'test', '--locked', '--doc', '--no-fail-fast') + $pkgArgs) @{ TEXTWEAVER_ESPEAK_OUTPUT = 'virtual' }
    }
    if (@($set | Where-Object { $_ -in @('textweaver-app', 'textweaver-tui', 'textweaver-components') }).Count -gt 0) {
        Invoke-CheckStep 'lean' 'the lean terminal reader builds, warnings are errors' $leanStep @{ CARGO_TARGET_DIR = $leanTarget }
    } else {
        Add-Skipped 'lean' 'Not needed: textweaver-app, textweaver-tui and textweaver-components did not change.'
    }
    if ($generatedToo) {
        Invoke-CheckStep 'generated' 'every generated file is current' @('cargo', '--locked', 'xtask', 'regen', '--check')
        Invoke-CheckStep 'deps' 'the dependency direction between the workspace crates' @('cargo', '--locked', 'xtask', 'deps', '--check')
    } else {
        Add-Skipped 'generated' 'Not needed: no docs, locale catalog, settings schema, keymap or Cargo.toml changed.'
        Add-Skipped 'deps' 'Not needed: no Cargo.toml changed.'
    }
    Write-Line
    Write-Line 'The fast check leaves out doc, pseudo, parity, deny, links, site-a11y, hosts32 and scripts. Run the full check once per batch.'
    if ($DryRun) {
        Write-Line
        Write-Line 'Dry run finished. Only cargo xtask affected was run.'
        exit 0
    }
    Write-Summary
    if ($script:failed.Count -gt 0) { exit 1 }
    exit 0
}

Invoke-CheckStep 'fmt' 'formatting' @('cargo', 'fmt', '--all', '--check')
Invoke-CheckStep 'clippy' 'lints, warnings are errors' (@('cargo', 'clippy', '--locked', '--workspace', '--all-targets') + $features + @('--', '-D', 'warnings'))
# The lean reader in its own target folder, so the main one is not rebuilt.
Invoke-CheckStep 'lean' 'the lean terminal reader builds, warnings are errors' $leanStep @{ CARGO_TARGET_DIR = $leanTarget }
# Tests never play audio: eSpeak NG writes to a virtual output, as in CI.
Invoke-TestsWithSentinel (@('cargo', 'test', '--locked', '--workspace', '--no-fail-fast') + $features) 'tests'
Invoke-CheckStep 'doc' 'API documentation, warnings are errors' (@('cargo', 'doc', '--locked', '--workspace', '--no-deps') + $features) @{ RUSTDOCFLAGS = '-D warnings' }
Invoke-CheckStep 'pseudo' 'the interface in the pseudo-locales en-XA and ar-XB' @('cargo', 'test', '--locked', '-p', 'textweaver-app', '--test', 'it', '--', 'pseudo_locale::') @{ TEXTWEAVER_ESPEAK_OUTPUT = 'virtual' }
Invoke-CheckStep 'generated' 'every generated file is current' @('cargo', '--locked', 'xtask', 'regen', '--check')
Invoke-CheckStep 'deps' 'the dependency direction between the workspace crates' @('cargo', '--locked', 'xtask', 'deps', '--check')
Invoke-CheckStep 'parity' 'word and sentence parity with star' @('cargo', '--locked', 'xtask', 'parity')
if (Test-Wanted 'deny') {
    $haveDeny = $true
    if (-not $DryRun) { & cargo deny --version *> $null; $haveDeny = ($LASTEXITCODE -eq 0) }
    if ($haveDeny) {
        Invoke-CheckStep 'deny' 'licenses, advisories, duplicates and sources (deny.toml)' @('cargo', 'deny', '--locked', 'check')
    } else {
        Add-Skipped 'deny' 'cargo-deny is not installed, so the deny check cannot run. CI runs it; tools.toml names the version.'
    }
}
# The py launcher first: python on Windows may be the Microsoft Store stub.
$python = @()
if (Get-Command 'py' -ErrorAction SilentlyContinue) {
    $python = @('py', '-3')
} else {
    foreach ($candidate in @('python3', 'python')) {
        if (Get-Command $candidate -ErrorAction SilentlyContinue) { $python = @($candidate); break }
    }
}
foreach ($pyStep in @('links', 'site-a11y')) {
    if (-not (Test-Wanted $pyStep)) { continue }
    if ($python.Count -eq 0) {
        Write-Line
        Write-Line "== ${pyStep}: skipped =="
        Write-Line "Python 3 is not installed, so the $pyStep check cannot run."
        $script:skipped += $pyStep
        continue
    }
    if ($pyStep -eq 'links') {
        Invoke-CheckStep 'links' 'links and anchors in the docs resolve' ($python + @('tools\check_links.py'))
    } else {
        Invoke-CheckStep 'site-a11y' 'static accessibility checks of docs\site' ($python + @('tools\check_site_a11y.py'))
    }
}
Invoke-CheckStep 'hosts32' 'the 32-bit engine hosts build' @('cargo', 'build', '--locked', '-p', 'textweaver-host', '-p', 'textweaver-eci', '-p', 'textweaver-sapi', '-p', 'textweaver-dectalk', '-p', 'textweaver-espeak', '--bins', '--target', 'i686-pc-windows-msvc')

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
