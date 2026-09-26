<#
.SYNOPSIS
  Builds textweaver-gui on Windows: finds the CMake, Ninja, and MSVC that
  Visual Studio bundles, and a libclang for bindgen, then runs cargo.

.DESCRIPTION
  wxdragon-sys (wxDragon's FFI crate) needs, at build time:
    - CMake and Ninja: it configures wxWidgets and its C++ wrapper with the
      Ninja generator on MSVC targets. Visual Studio bundles both under
      Common7\IDE\CommonExtensions\Microsoft\CMake\ (CMake\bin, Ninja).
    - The MSVC compiler environment (cl.exe, INCLUDE, LIB): imported here from
      the newest Visual Studio's vcvars64.bat (found with vswhere).
    - libclang (bindgen generates the FFI bindings): LIBCLANG_PATH if set,
      else LLVM's installer location, else Visual Studio's optional "C++ Clang
      tools" component, else a conda libclang13 (copied under target\libclang
      as libclang.dll, the name clang-sys looks for).
  Nothing is installed and no system setting is changed; PATH and the other
  variables are set for the cargo process only.

  The first build downloads the wxWidgets 3.3.3 source archive from GitHub
  (wxdragon-sys verifies its SHA-256) and compiles it: expect several minutes.

.EXAMPLE
  powershell -File crates/textweaver-gui/tools/build-windows.ps1
  powershell -File crates/textweaver-gui/tools/build-windows.ps1 -Cargo "clippy -p textweaver-gui --all-targets -- -D warnings"
#>
param(
    # The cargo command line, as one string (split at spaces); the default
    # builds the GUI in the dev profile.
    [string] $Cargo = 'build -p textweaver-gui',
    # Print the environment that would be used, and exit.
    [switch] $ShowEnv
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path

# --- Visual Studio: vcvars, CMake, Ninja -----------------------------------
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) { throw "vswhere.exe not found; install Visual Studio with the C++ workload." }
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw "No Visual Studio with the MSVC x64 tools was found." }
$cmakeBin = Join-Path $vs 'Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin'
$ninjaDir = Join-Path $vs 'Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja'
foreach ($p in @("$cmakeBin\cmake.exe", "$ninjaDir\ninja.exe")) {
    if (-not (Test-Path $p)) { throw "Not found: $p (install the 'C++ CMake tools for Windows' component)." }
}

# Import the x64 developer environment (cl.exe, INCLUDE, LIB, LIBPATH).
$vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
$lines = & cmd.exe /d /c "`"$vcvars`" >nul 2>&1 && set"
foreach ($line in $lines) {
    $i = $line.IndexOf('=')
    if ($i -gt 0) { [Environment]::SetEnvironmentVariable($line.Substring(0, $i), $line.Substring($i + 1), 'Process') }
}
$env:PATH = "$cmakeBin;$ninjaDir;$env:PATH"
$env:CMAKE_GENERATOR = 'Ninja'

# --- libclang for bindgen ---------------------------------------------------
function Test-LibClang([string] $dir) { $dir -and (Test-Path (Join-Path $dir 'libclang.dll')) }
$libclang = $env:LIBCLANG_PATH
if (-not (Test-LibClang $libclang)) {
    $candidates = @(
        (Join-Path $env:ProgramFiles 'LLVM\bin'),
        (Join-Path $vs 'VC\Tools\Llvm\x64\bin')
    )
    $libclang = $candidates | Where-Object { Test-LibClang $_ } | Select-Object -First 1
}
if (-not $libclang) {
    # A conda libclang13 ships the C API as libclang-13.dll; clang-sys only
    # looks for libclang.dll, so copy it (and its two runtime DLLs) aside.
    $conda = @("$env:USERPROFILE\radioconda", "$env:USERPROFILE\miniforge3", "$env:USERPROFILE\miniconda3", "$env:USERPROFILE\anaconda3") |
        Where-Object { Test-Path "$_\Library\bin\libclang-13.dll" } | Select-Object -First 1
    if ($conda) {
        $libclang = Join-Path $repo 'target\libclang'
        New-Item -ItemType Directory -Force $libclang | Out-Null
        Copy-Item "$conda\Library\bin\libclang-13.dll" "$libclang\libclang.dll" -Force
        foreach ($dll in 'zlib.dll', 'zstd.dll') {
            if (Test-Path "$conda\Library\bin\$dll") { Copy-Item "$conda\Library\bin\$dll" $libclang -Force }
        }
    }
}
if (-not $libclang) {
    throw "libclang not found. Install LLVM (winget install LLVM.LLVM) or Visual Studio's 'C++ Clang tools for Windows', or set LIBCLANG_PATH."
}
$env:LIBCLANG_PATH = $libclang
$env:PATH = "$libclang;$env:PATH"

Write-Host "Visual Studio : $vs"
Write-Host "CMake         : $((& "$cmakeBin\cmake.exe" --version | Select-Object -First 1))"
Write-Host "Ninja         : $((& "$ninjaDir\ninja.exe" --version))"
Write-Host "cl.exe        : $((Get-Command cl.exe).Source)"
Write-Host "LIBCLANG_PATH : $libclang"
if ($ShowEnv) { return }

# --- cargo ------------------------------------------------------------------
$CargoArgs = $Cargo -split '\s+' | Where-Object { $_ }
Push-Location $repo
try {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    & cargo @CargoArgs
    $code = $LASTEXITCODE
    $sw.Stop()
    Write-Host ("cargo {0}: exit {1} in {2:N1} s" -f ($CargoArgs -join ' '), $code, $sw.Elapsed.TotalSeconds)
    exit $code
} finally {
    Pop-Location
}
