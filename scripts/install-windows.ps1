<#
.SYNOPSIS
  Installs textweaver on Windows, from the latest GitHub release or from
  source.

.DESCRIPTION
  By default the script downloads the newest release zip from GitHub, checks
  its SHA-256 against SHA256SUMS.txt, and extracts it to
  %LOCALAPPDATA%\Programs\textweaver. It then offers to add that folder to
  your user PATH and to create a Start menu shortcut that opens textweaver in
  a terminal. With -FromSource it builds the package with cargo xtask dist
  instead, after checking for the MSVC build tools, rustup, and the 32-bit
  Rust target.

  Every step is announced before it runs. The script asks before it changes
  your PATH, installs Rust, or creates a shortcut; -Yes answers yes. Running
  it again updates the install. It works in Windows PowerShell 5.1 and in
  PowerShell 7.

  GNU-style options work too: --help, --dry-run, --yes, --from-source,
  --uninstall, --release TAG, --install-dir DIR.

.PARAMETER Release
  Install this release tag, such as v0.1.0-alpha.3, instead of the newest.

.PARAMETER FromSource
  Build from source with cargo xtask dist.

.PARAMETER Source
  With -FromSource, the textweaver checkout to build (default: the checkout
  this script is in).

.PARAMETER InstallDir
  Where to install (default: %LOCALAPPDATA%\Programs\textweaver).

.PARAMETER NoShortcut
  Do not offer the Start menu shortcut.

.PARAMETER Uninstall
  Remove textweaver, its PATH entry, and its shortcut. Settings are kept.

.PARAMETER DryRun
  Print each step instead of doing it.

.PARAMETER Yes
  Answer yes to every question.

.PARAMETER Help
  Show the help.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1 -DryRun

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1 -FromSource
#>
[CmdletBinding(PositionalBinding = $false)]
param(
    [string] $Release,
    [switch] $FromSource,
    [string] $Source,
    [string] $InstallDir,
    [switch] $NoShortcut,
    [switch] $Uninstall,
    [switch] $DryRun,
    [switch] $Yes,
    [switch] $Help,
    [Parameter(ValueFromRemainingArguments = $true)] [string[]] $Rest
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$Repo = 'leavesofgrass/textweaver'
if ($env:TEXTWEAVER_REPO) { $Repo = $env:TEXTWEAVER_REPO }

# ------------------------------------------------------------- helpers --

function Write-Line([string] $Text = '') { [Console]::Out.WriteLine($Text) }

function Write-Section([string] $Title) {
    Write-Line
    Write-Line "== $Title =="
}

function Format-Command([string[]] $Parts) {
    ($Parts | ForEach-Object {
            if ($_ -eq '' -or $_ -match '[\s"]') { '"' + $_ + '"' } else { $_ }
        }) -join ' '
}

# Runs a program, or prints it in a dry run. Stops on a non-zero exit.
function Invoke-Native([string[]] $Command, [string] $WorkingDirectory) {
    $shown = Format-Command $Command
    if ($WorkingDirectory) { $shown = "$shown (in $WorkingDirectory)" }
    if ($script:DryRun) { Write-Line "Would run: $shown"; return }
    Write-Line "Running: $shown"
    if ($WorkingDirectory) { Push-Location $WorkingDirectory }
    try {
        $exe = $Command[0]
        $arguments = @()
        if ($Command.Count -gt 1) { $arguments = $Command[1..($Command.Count - 1)] }
        & $exe @arguments
        if ($LASTEXITCODE -ne 0) { throw "$exe failed with exit code $LASTEXITCODE." }
    } finally {
        if ($WorkingDirectory) { Pop-Location }
    }
}

# Describes a step and runs it, or only describes it in a dry run.
function Invoke-Step([string] $Description, [scriptblock] $Action) {
    if ($script:DryRun) { Write-Line "Would $Description"; return }
    Write-Line (($Description.Substring(0, 1).ToUpper()) + $Description.Substring(1) + '.')
    & $Action
}

# Asks a yes-or-no question. -Yes answers yes; no console answers no.
function Confirm-Choice([string] $Question) {
    if ($script:Yes) { Write-Line "$Question Answering yes, because of -Yes."; return $true }
    if ([Console]::IsInputRedirected -or -not [Environment]::UserInteractive) {
        Write-Line "$Question Answering no, because there is no console to ask on. Use -Yes to answer yes."
        return $false
    }
    $reply = Read-Host "$Question Type y for yes or n for no, then press Enter"
    return ($reply -match '^(y|yes)$')
}

# ----------------------------------------------------------- arguments --

$i = 0
while ($null -ne $Rest -and $i -lt $Rest.Count) {
    $a = $Rest[$i]
    switch -Regex ($a) {
        '^(--help|-h|/\?|-\?)$' { $Help = $true }
        '^--dry-run$' { $DryRun = $true }
        '^(--yes|-y)$' { $Yes = $true }
        '^--from-source$' { $FromSource = $true }
        '^--uninstall$' { $Uninstall = $true }
        '^--no-shortcut$' { $NoShortcut = $true }
        '^--release$' { $i++; $Release = $Rest[$i] }
        '^--source$' { $i++; $Source = $Rest[$i] }
        '^--(install-dir|prefix)$' { $i++; $InstallDir = $Rest[$i] }
        default { Write-Line "Error: unknown option $a. Run with -Help to see the options."; exit 2 }
    }
    $i++
}

if ($Help) {
    Write-Line 'Usage: scripts\install-windows.ps1 [-Release TAG | -FromSource [-Source DIR]] [-InstallDir DIR]'
    Write-Line '                                   [-NoShortcut] [-Uninstall] [-DryRun] [-Yes] [-Help]'
    Write-Line
    Write-Line 'Installs textweaver from the newest GitHub release: downloads the zip, checks its'
    Write-Line 'SHA-256 against SHA256SUMS.txt, and extracts it to %LOCALAPPDATA%\Programs\textweaver.'
    Write-Line 'Then it offers to add that folder to your user PATH and to create a Start menu'
    Write-Line 'shortcut. -FromSource builds the package with cargo xtask dist instead.'
    Write-Line
    Write-Line 'Options:'
    Write-Line '  -Release TAG     Install this release, such as v0.1.0-alpha.3.'
    Write-Line '  -FromSource      Build from source (needs the MSVC build tools and rustup).'
    Write-Line '  -Source DIR      The checkout to build (default: the one this script is in).'
    Write-Line '  -InstallDir DIR  Where to install (default: %LOCALAPPDATA%\Programs\textweaver).'
    Write-Line '  -NoShortcut      Do not offer the Start menu shortcut.'
    Write-Line '  -Uninstall       Remove textweaver, its PATH entry, and its shortcut. Settings are kept.'
    Write-Line '  -DryRun          Print each step instead of doing it.'
    Write-Line '  -Yes             Answer yes to every question.'
    Write-Line '  -Help            Show this help. --help, --dry-run, --yes and the other GNU-style'
    Write-Line '                   spellings work too.'
    Write-Line
    Write-Line 'Example: powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1 -DryRun'
    exit 0
}

if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\textweaver' }
$InstallDir = [IO.Path]::GetFullPath($InstallDir)
$ShortcutPath = Join-Path ([Environment]::GetFolderPath('Programs')) 'textweaver.lnk'
$Manifest = Join-Path $InstallDir 'install-manifest.txt'
$RepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$env:CARGO_TERM_PROGRESS_WHEN = 'never'
if ($env:NO_COLOR) { $env:CARGO_TERM_COLOR = 'never' }

# ---------------------------------------------------------------- PATH --

function Get-UserPath {
    $key = Get-Item -LiteralPath 'HKCU:\Environment'
    return [string] $key.GetValue('Path', '', 'DoNotExpandEnvironmentNames')
}

function Test-PathEntry([string] $PathValue, [string] $Dir) {
    foreach ($p in ($PathValue -split ';')) {
        if ($p -and ([Environment]::ExpandEnvironmentVariables($p).TrimEnd('\') -ieq $Dir.TrimEnd('\'))) { return $true }
    }
    return $false
}

# Writes the user PATH, keeping %VARIABLES% unexpanded, then tells Windows
# so new terminals see the change.
function Write-UserPath([string] $Value) {
    New-ItemProperty -LiteralPath 'HKCU:\Environment' -Name 'Path' -Value $Value -PropertyType ExpandString -Force | Out-Null
    [Environment]::SetEnvironmentVariable('TEXTWEAVER_PATH_REFRESH', '1', 'User')
    [Environment]::SetEnvironmentVariable('TEXTWEAVER_PATH_REFRESH', $null, 'User')
}

function Add-ToUserPath {
    Write-Section 'PATH'
    $current = Get-UserPath
    if (Test-PathEntry $current $InstallDir) {
        Write-Line "$InstallDir is already on your user PATH, so you can run textweaver and tw from any folder."
        return
    }
    Write-Line "$InstallDir is not on your user PATH. Adding it lets you run textweaver and tw from any folder, in new terminals."
    if (-not (Confirm-Choice "Add $InstallDir to your user PATH?")) {
        Write-Line "PATH unchanged. Run textweaver as $InstallDir\textweaver.exe."
        return
    }
    Invoke-Step "add $InstallDir to the user PATH (HKCU\Environment)" {
        $new = if ($current) { $current.TrimEnd(';') + ';' + $InstallDir } else { $InstallDir }
        Write-UserPath $new
        Write-Line 'Added. Open a new terminal to use it.'
    }
}

function Remove-FromUserPathEntry {
    $current = Get-UserPath
    if (-not (Test-PathEntry $current $InstallDir)) { return }
    if (-not (Confirm-Choice "Remove $InstallDir from your user PATH?")) { return }
    Invoke-Step "remove $InstallDir from the user PATH" {
        $kept = ($current -split ';') | Where-Object {
            $_ -and ([Environment]::ExpandEnvironmentVariables($_).TrimEnd('\') -ine $InstallDir.TrimEnd('\'))
        }
        Write-UserPath ($kept -join ';')
    }
}

# ------------------------------------------------------------ shortcut --

function Add-StartMenuShortcut {
    if ($NoShortcut) { return }
    Write-Section 'Start menu'
    $exe = Join-Path $InstallDir 'textweaver.exe'
    $wt = Get-Command 'wt.exe' -ErrorAction SilentlyContinue
    if ($wt) {
        $target = $wt.Source
        $arguments = "-d `"%USERPROFILE%`" `"$exe`""
        $how = 'Windows Terminal'
    } else {
        $target = Join-Path $env:SystemRoot 'System32\cmd.exe'
        $arguments = "/k `"`"$exe`"`""
        $how = 'a command prompt'
    }
    Write-Line "A Start menu shortcut named textweaver can open $how running textweaver."
    if (Test-Path -LiteralPath $ShortcutPath) { Write-Line 'The shortcut exists already; it will be updated.' }
    if (-not (Confirm-Choice 'Create the Start menu shortcut?')) { Write-Line 'No shortcut created.'; return }
    Invoke-Step "create the shortcut $ShortcutPath" {
        $shell = New-Object -ComObject WScript.Shell
        $link = $shell.CreateShortcut($ShortcutPath)
        $link.TargetPath = $target
        $link.Arguments = $arguments
        $link.WorkingDirectory = $env:USERPROFILE
        $link.Description = 'Read documents aloud, with a highlight that follows the spoken word'
        $link.IconLocation = "$exe,0"
        $link.Save()
    }
}

# ------------------------------------------------------------- release --

function Get-ReleasePackage {
    Write-Section 'Download'
    $tag = $Release
    if (-not $tag) {
        $api = "https://api.github.com/repos/$Repo/releases?per_page=1"
        if ($DryRun) {
            Write-Line "Would look up the newest release at $api"
            $tag = 'vVERSION'
        } else {
            Write-Line "Looking up the newest release at $api"
            $releases = @(Invoke-RestMethod -UseBasicParsing -Uri $api -Headers @{ 'User-Agent' = 'textweaver-installer' })
            if ($releases.Count -eq 0) { throw "No release was found on https://github.com/$Repo/releases." }
            $tag = $releases[0].tag_name
            Write-Line "The newest release is $tag."
        }
    }
    $version = $tag -replace '^v', ''
    $name = "textweaver-$version-windows-x86_64"
    $base = "https://github.com/$Repo/releases/download/$tag"
    $work = Join-Path ([IO.Path]::GetTempPath()) ("textweaver-install-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
    $zip = Join-Path $work "$name.zip"
    $sums = Join-Path $work 'SHA256SUMS.txt'
    Write-Line "Downloading $name.zip and SHA256SUMS.txt from release $tag."
    Invoke-Step "download $base/$name.zip to $zip" {
        New-Item -ItemType Directory -Force -Path $work | Out-Null
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$name.zip" -OutFile $zip
    }
    Invoke-Step "download $base/SHA256SUMS.txt to $sums" {
        Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS.txt" -OutFile $sums
    }
    Invoke-Step "check the SHA-256 of $name.zip against SHA256SUMS.txt" {
        $want = $null
        foreach ($line in (Get-Content -LiteralPath $sums)) {
            $parts = $line -split '\s+', 2
            if ($parts.Count -eq 2 -and $parts[1].TrimStart('*') -eq "$name.zip") { $want = $parts[0] }
        }
        if (-not $want) { throw "SHA256SUMS.txt has no line for $name.zip, so the download cannot be checked. Nothing was installed." }
        $got = (Get-FileHash -Algorithm SHA256 -LiteralPath $zip).Hash
        if ($got -ine $want) { throw "The checksum does not match (expected $want, got $got). The download may be damaged. Nothing was installed." }
        Write-Line "The checksum matches: $($got.ToLower())."
    }
    Invoke-Step "extract $name.zip" {
        Expand-Archive -LiteralPath $zip -DestinationPath $work -Force
    }
    return @{ Stage = (Join-Path $work $name); Work = $work; Tag = $tag; Version = $version }
}

# -------------------------------------------------------------- source --

function Test-MsvcTool {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere)) { return $null }
    $vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    return $vs
}

function Install-Rustup {
    Write-Line 'The script can download rustup-init.exe from https://win.rustup.rs/x86_64 and install Rust for you only, in %USERPROFILE%\.cargo and %USERPROFILE%\.rustup. It does not change your PATH.'
    if (-not (Confirm-Choice 'Install rustup now?')) { return $false }
    $init = Join-Path ([IO.Path]::GetTempPath()) 'rustup-init.exe'
    Invoke-Step 'download https://win.rustup.rs/x86_64' {
        Invoke-WebRequest -UseBasicParsing -Uri 'https://win.rustup.rs/x86_64' -OutFile $init
    }
    Invoke-Native @($init, '-y', '-q', '--no-modify-path', '--profile', 'minimal', '--default-toolchain', 'none')
    $env:PATH = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + $env:PATH
    return $true
}

function Build-FromSource {
    Write-Section 'Build from source'
    $src = $Source
    if (-not $src) { $src = $RepoRoot }
    $src = [IO.Path]::GetFullPath($src)
    if (-not (Test-Path -LiteralPath (Join-Path $src 'crates\textweaver-cli'))) {
        throw "$src is not a textweaver checkout. Clone https://github.com/$Repo and use -Source."
    }
    Write-Line "Using the textweaver source in $src."

    $vs = Test-MsvcTool
    if ($vs) {
        Write-Line "The MSVC build tools are installed: $vs."
    } elseif ($DryRun) {
        Write-Line 'The MSVC build tools were not found. A real run would stop here and explain how to install them.'
    } else {
        throw 'The MSVC build tools were not found. Install Visual Studio Build Tools with the "Desktop development with C++" workload (winget install Microsoft.VisualStudio.2022.BuildTools), then run this script again.'
    }

    $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if (-not (Get-Command 'rustup' -ErrorAction SilentlyContinue) -and (Test-Path -LiteralPath (Join-Path $cargoBin 'rustup.exe'))) {
        $env:PATH = "$cargoBin;$env:PATH"
    }
    if (Get-Command 'rustup' -ErrorAction SilentlyContinue) {
        Write-Line 'rustup is installed. It installs the Rust version pinned in rust-toolchain.toml.'
    } else {
        Write-Line 'rustup is not installed.'
        if (-not (Install-Rustup)) { throw 'Rust is needed. Install rustup from https://rustup.rs, then run this script again.' }
    }
    Invoke-Native @('rustup', 'toolchain', 'install') $src

    # A read-only check, so it runs in a dry run too.
    $haveX86 = $false
    if (Get-Command 'rustup' -ErrorAction SilentlyContinue) {
        Push-Location $src
        try { $haveX86 = [bool] (& rustup target list --installed | Where-Object { $_ -eq 'i686-pc-windows-msvc' }) } finally { Pop-Location }
    }
    if ($haveX86) {
        Write-Line 'The 32-bit target i686-pc-windows-msvc is installed. It builds the hosts for 32-bit engines such as Code Factory Eloquence.'
    } else {
        Write-Line 'The 32-bit Rust target i686-pc-windows-msvc builds the hosts for 32-bit speech engines, such as Code Factory Eloquence and most DECtalk and SAPI voices.'
        if (Confirm-Choice 'Add the i686-pc-windows-msvc target with rustup?') {
            Invoke-Native @('rustup', 'target', 'add', 'i686-pc-windows-msvc') $src
        } elseif (-not $DryRun) {
            throw 'cargo xtask dist needs the 32-bit target. Run: rustup target add i686-pc-windows-msvc'
        }
    }

    Write-Line 'Building the Windows package with cargo xtask dist. The first build takes several minutes.'
    Invoke-Native @('cargo', 'xtask', 'dist') $src

    $version = 'VERSION'
    $toml = Join-Path $src 'Cargo.toml'
    $inPackage = $false
    foreach ($line in (Get-Content -LiteralPath $toml)) {
        if ($line -match '^\[workspace\.package\]') { $inPackage = $true; continue }
        if ($inPackage -and $line -match '^version\s*=\s*"([^"]+)"') { $version = $Matches[1]; break }
    }
    $target = $env:CARGO_TARGET_DIR
    if (-not $target) { $target = Join-Path $src 'target' }
    return @{ Stage = (Join-Path $target "dist\textweaver-$version-windows-x86_64"); Work = $null; Tag = 'source'; Version = $version; Source = $src }
}

# ------------------------------------------------------------- install --

function Install-Stage($Package) {
    Write-Section 'Install'
    $stage = $Package.Stage
    Write-Line "Installing textweaver, tw, the engine hosts, and the dictionaries into $InstallDir."
    if (-not $DryRun -and -not (Test-Path -LiteralPath (Join-Path $stage 'tw.exe'))) {
        throw "$stage\tw.exe is missing, so there is nothing to install."
    }
    if (Test-Path -LiteralPath $InstallDir) {
        Invoke-Step "remove the previous files in $InstallDir" {
            Get-ChildItem -LiteralPath $InstallDir -Force | Remove-Item -Recurse -Force
        }
    }
    Invoke-Step "copy $stage to $InstallDir" {
        New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        Copy-Item -Path (Join-Path $stage '*') -Destination $InstallDir -Recurse -Force
        Get-ChildItem -LiteralPath $InstallDir -Recurse -File | Unblock-File
    }
    $scripts = @('install-windows.ps1', 'update.ps1', 'doctor.ps1', 'speech-check.ps1', 'convert-folder.ps1', 'README.md')
    $present = @($scripts | Where-Object { Test-Path -LiteralPath (Join-Path $PSScriptRoot $_) })
    $probe = Join-Path $RepoRoot 'tools\sapi_probe.ps1'
    if ($present.Count -gt 0) {
        Invoke-Step "copy the helper scripts ($($present -join ', ')) to $InstallDir\scripts" {
            $dest = Join-Path $InstallDir 'scripts'
            New-Item -ItemType Directory -Force -Path $dest | Out-Null
            foreach ($s in $present) { Copy-Item -LiteralPath (Join-Path $PSScriptRoot $s) -Destination $dest -Force }
            if (Test-Path -LiteralPath $probe) { Copy-Item -LiteralPath $probe -Destination $dest -Force }
        }
    }
    Invoke-Step "write $Manifest" {
        $lines = @('# textweaver install manifest, written by scripts/install-windows.ps1.')
        if ($Package.Tag -eq 'source') {
            $lines += 'kind=source'
            $lines += "source=$($Package.Source)"
        } else {
            $lines += 'kind=release'
            $lines += "tag=$($Package.Tag)"
        }
        $lines += "version=$($Package.Version)"
        [IO.File]::WriteAllLines($Manifest, [string[]] $lines)
    }
}

function Uninstall-Textweaver {
    Write-Line "This removes textweaver from $InstallDir, its user PATH entry, and its Start menu shortcut."
    Write-Line 'It keeps your settings, reading positions, and notes, and it does not remove Rust.'
    if (-not (Confirm-Choice "Remove textweaver from $InstallDir?")) { Write-Line 'Nothing removed.'; return }
    if ((Test-Path -LiteralPath $InstallDir) -or $DryRun) {
        Invoke-Step "remove $InstallDir" { Remove-Item -LiteralPath $InstallDir -Recurse -Force }
    } else {
        Write-Line "$InstallDir does not exist."
    }
    Remove-FromUserPathEntry
    if ((Test-Path -LiteralPath $ShortcutPath) -or $DryRun) {
        Invoke-Step "remove the shortcut $ShortcutPath" { Remove-Item -LiteralPath $ShortcutPath -Force }
    }
    Write-Line
    Write-Line 'textweaver is removed.'
}

# ---------------------------------------------------------------- main --

if ($DryRun) { Write-Line 'Dry run: nothing is changed. Each step is printed instead of done.' }

if ($Uninstall) {
    Uninstall-Textweaver
    exit 0
}

try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
} catch {
    Write-Verbose 'TLS 1.2 is already the default.'
}

if ($FromSource) {
    Write-Line "This script builds textweaver from source and installs it in $InstallDir."
    $package = Build-FromSource
} else {
    Write-Line "This script downloads the newest textweaver release for Windows, checks it, and installs it in $InstallDir."
    $package = Get-ReleasePackage
}
try {
    Install-Stage $package
} finally {
    if ($package.Work -and (Test-Path -LiteralPath $package.Work)) { Remove-Item -LiteralPath $package.Work -Recurse -Force }
}
Add-ToUserPath
Add-StartMenuShortcut

Write-Section 'Done'
if ($DryRun) {
    Write-Line 'Dry run finished. Nothing was changed.'
    exit 0
}
Write-Line "textweaver $($package.Version) is installed in $InstallDir."
Write-Line "Check your speech engines: & `"$InstallDir\tw.exe`" backends"
Write-Line "Read the quick start aloud: & `"$InstallDir\textweaver.exe`" `"$InstallDir\QUICKSTART.md`""
Write-Line 'To remove textweaver, run this script with -Uninstall.'
