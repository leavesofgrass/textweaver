<#
.SYNOPSIS
  Report build disk use and, only when asked, flush old build output.

.DESCRIPTION
  textweaver builds grow very large (per-agent target folders, cross
  builds, Docker). This script keeps the disk from filling up.

  Default: a report only. Shows free space on D:, each build folder's size,
  the shared compiler cache, and whether D: is below the 200 GB floor.
  Nothing is removed.

  -Flush lists exactly which build folders are flush candidates, with sizes.
  -Flush -Apply removes those folders, and only those.

  Safety rules (see CLAUDE.md, "Deleting files: hard rules"):
    * Only folders on the allow-list below, under D:\textweaver\target or a
      worktree's own target folder, can ever be candidates.
    * Every candidate's full path is resolved and checked to be inside one
      of those roots before removal. Nothing else is ever touched.
    * libclang, bench-corpus, dist and dist-build are never candidates.
    * An agent's folder is a candidate only if its branch is merged into
      main, or its worktree no longer exists.
    * debug, release, doc and cross-target folders are candidates only with
      -IncludeShared, since sccache makes rebuilding them cheap.

.EXAMPLE
  powershell -File tools\build-hygiene.ps1
  powershell -File tools\build-hygiene.ps1 -Flush
  powershell -File tools\build-hygiene.ps1 -Flush -IncludeShared -Apply
#>
param(
  [switch]$Flush,
  [switch]$IncludeShared,
  [switch]$Apply,
  [int]$FloorGB = 200
)

$ErrorActionPreference = 'Stop'
$repo = 'D:\textweaver'
$targetRoot = Join-Path $repo 'target'
$worktreeRoot = Join-Path $repo '.claude\worktrees'
$never = @('libclang', 'bench-corpus', 'dist', 'dist-build', 'CACHEDIR.TAG')
$shared = @('debug', 'release', 'doc', 'tmp', 'i686-pc-windows-msvc', 'aarch64-apple-darwin',
            'x86_64-pc-windows-gnu', 'x86_64-unknown-linux-gnu')

function Get-SizeGB([string]$path) {
  $bytes = 0
  try {
    $bytes = [IO.Directory]::EnumerateFiles($path, '*', [IO.SearchOption]::AllDirectories) |
      ForEach-Object { ([IO.FileInfo]$_).Length } | Measure-Object -Sum | Select-Object -Expand Sum
  } catch { }
  [math]::Round(($bytes / 1GB), 1)
}

function Test-Inside([string]$path, [string[]]$roots) {
  $full = [IO.Path]::GetFullPath($path).TrimEnd('\') + '\'
  foreach ($r in $roots) {
    $root = [IO.Path]::GetFullPath($r).TrimEnd('\') + '\'
    if ($full.StartsWith($root, [StringComparison]::OrdinalIgnoreCase) -and $full.Length -gt $root.Length) { return $true }
  }
  $false
}

Push-Location $repo
try {
  $free = [math]::Round((Get-PSDrive D).Free / 1GB)
  "Free on D: $free GB (floor $FloorGB GB)"
  if ($free -lt $FloorGB) { "WARNING: below the floor. Flush build output before building." }

  $merged = @(git branch --merged main --format '%(refname:short)')
  $worktrees = @(git worktree list --porcelain | Where-Object { $_ -like 'worktree *' } | ForEach-Object { $_.Substring(9).Replace('/', '\') })

  $rows = @()
  if (Test-Path $targetRoot) {
    foreach ($d in Get-ChildItem $targetRoot -Directory -Force) {
      $name = $d.Name
      $why = $null
      if ($never -contains $name) { $why = $null }
      elseif ($shared -contains $name) { if ($IncludeShared) { $why = 'shared build output (sccache rebuilds it quickly)' } }
      elseif ($name -match '^(w\d+[a-z]?|p\d+[a-z]?|agent-|orch|wave)') {
        $branch = $merged | Where-Object { $_ -match [regex]::Escape($name) -or ($name -match '^w(\d+)([a-z])' -and $_ -match "wave$($Matches[1])/$($Matches[2])-") } | Select-Object -First 1
        if ($branch) { $why = "agent output; branch $branch is merged" }
      }
      $rows += [pscustomobject]@{ Path = $d.FullName; GB = (Get-SizeGB $d.FullName); Candidate = [bool]$why; Reason = $why }
    }
  }
  if (Test-Path $worktreeRoot) {
    foreach ($w in Get-ChildItem $worktreeRoot -Directory -Force) {
      $t = Join-Path $w.FullName 'target'
      if (-not (Test-Path $t)) { continue }
      $live = $worktrees | Where-Object { $_ -ieq $w.FullName }
      $branch = if ($live) { (git -C $w.FullName rev-parse --abbrev-ref HEAD 2>$null) } else { $null }
      $why = $null
      if (-not $live -or -not (Test-Path (Join-Path $w.FullName '.git'))) { $why = 'worktree no longer exists' }
      elseif ($branch -and ($merged -contains $branch)) { $why = "worktree branch $branch is merged" }
      $rows += [pscustomobject]@{ Path = $t; GB = (Get-SizeGB $t); Candidate = [bool]$why; Reason = $why }
    }
  }
  $cache = if (Test-Path 'D:\sccache') { Get-SizeGB 'D:\sccache' } else { 0 }

  ''
  'Build folders:'
  $rows | Sort-Object GB -Descending | Format-Table Path, GB, Candidate, Reason -AutoSize | Out-String -Width 220
  "Shared compiler cache D:\sccache: $cache GB (sccache keeps it under its size limit itself)"

  if (-not $Flush) { return }

  $cands = @($rows | Where-Object Candidate)
  ''
  if ($cands.Count -eq 0) { 'No flush candidates.'; return }
  $total = ($cands | Measure-Object GB -Sum).Sum
  "Flush candidates: $($cands.Count) folders, $total GB:"
  $cands | ForEach-Object { "  $($_.Path)  ($($_.GB) GB)  - $($_.Reason)" }
  if (-not $Apply) { ''; 'Nothing removed. Run again with -Apply to remove exactly the folders listed above.'; return }

  $roots = @($targetRoot) + @(Get-ChildItem $worktreeRoot -Directory -Force -ErrorAction SilentlyContinue | ForEach-Object { Join-Path $_.FullName 'target' })
  foreach ($c in $cands) {
    if (-not (Test-Inside $c.Path $roots)) { "SKIPPED (outside the allowed roots): $($c.Path)"; continue }
    if ($never -contains (Split-Path $c.Path -Leaf)) { "SKIPPED (protected): $($c.Path)"; continue }
    Remove-Item -LiteralPath $c.Path -Recurse -Force
    "removed $($c.Path)"
  }
  "Free on D: now $([math]::Round((Get-PSDrive D).Free / 1GB)) GB"
} finally { Pop-Location }
