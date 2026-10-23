<#
.SYNOPSIS
  Dumps the Xilem GUI's accessibility tree on Windows with accessibility-cli
  (UI Automation), for the tree check in ADR-0039. Linux and macOS use
  tree-dump.sh.

.DESCRIPTION
  Opens fixtures/t/reading.md with the silent `paced` backend and
  --background (never activated, off screen, no taskbar button), waits
  until the tree holds the document, and writes into -Out:
    raw.json      accessibility-cli --json, as dumped
    raw-tree.txt  accessibility-cli's own text tree, for a person to read
    tree.txt      the normalized tree (tools/a11y/tree_report.py)
    gui.log       the GUI's log
  It is meant for CI runners. It plays no audio and never takes the
  foreground.

.EXAMPLE
  powershell -File tools/a11y/tree-dump.ps1 -Out a11y-tree
#>
param(
    [Parameter(Mandatory = $true)] [string] $Out,
    [string] $Exe = '',
    [string] $Document = '',
    [string] $Cli = 'accessibility-cli',
    # The longest it waits for the tree, in seconds.
    [int] $WaitSeconds = 40
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
if (-not $Exe) {
    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $repo 'target' }
    $Exe = Join-Path $target 'debug\textweaver.exe'
}
if (-not $Document) { $Document = Join-Path $repo 'fixtures\t\reading.md' }
if (-not (Test-Path -LiteralPath $Exe)) { throw "Not built: $Exe (cargo build -p textweaver-xilem)." }
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path -LiteralPath $Out).Path
$home_dir = Join-Path $Out 'home'
New-Item -ItemType Directory -Force -Path $home_dir | Out-Null
$log = Join-Path $Out 'gui.log'
$raw = Join-Path $Out 'raw.json'
$errors = Join-Path $Out 'cli-errors.txt'

$guiArgs = @("`"$Document`"", '--backend', 'paced', '--background', '--exit-after', ($WaitSeconds + 30),
    '--home', "`"$home_dir`"", '--log-file', "`"$log`"")
$gui = Start-Process -FilePath $Exe -ArgumentList $guiArgs -PassThru
# Windows PowerShell turns a native command's redirected stderr into an
# error record, which 'Stop' makes fatal: the first try, made before the
# window is shown, ended the script and left the GUI running. The tool's
# exit code is checked instead.
$ErrorActionPreference = 'Continue'
$found = $false
$deadline = (Get-Date).AddSeconds($WaitSeconds)
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 2
    if ($gui.HasExited) {
        Write-Output "Fail: the GUI exited (code $($gui.ExitCode)) before its tree could be dumped."
        break
    }
    & $Cli --platform win --pid $gui.Id --json 2> $errors | Out-File -Encoding utf8 -FilePath $raw
    if ($LASTEXITCODE -eq 0) {
        $text = Get-Content -LiteralPath $raw -Raw
        if ($text -match '"role"' -and $text -match 'Reading check') { $found = $true; break }
    }
}
if (-not $gui.HasExited) {
    & $Cli --platform win --pid $gui.Id 2>&1 | Out-File -Encoding utf8 -FilePath (Join-Path $Out 'raw-tree.txt')
    Stop-Process -Id $gui.Id -ErrorAction SilentlyContinue
}
$ErrorActionPreference = 'Stop'
if (-not $found) {
    # "Could not find window for PID n (found m top-level windows)" means
    # the GUI had no window in UI Automation yet: m counts every top-level
    # window on the desktop, not the GUI's. The log shows how far it got.
    Write-Output "Fail: no tree with the document in $WaitSeconds seconds. accessibility-cli said:"
    if (Test-Path -LiteralPath $errors) { Get-Content -LiteralPath $errors }
    if (Test-Path -LiteralPath $log) {
        Write-Output "The GUI log ends:"
        Get-Content -LiteralPath $log -Tail 5
    }
    exit 1
}
$py = if (Get-Command python -ErrorAction SilentlyContinue) { 'python' } else { 'py' }
& $py (Join-Path $PSScriptRoot 'tree_report.py') normalize $raw (Join-Path $Out 'tree.txt')
exit $LASTEXITCODE
