# M-LINK.0 Windows spike.
#
# Links the executable objects emitted by vut-objgen with three candidate
# linkers (link.exe / lld-link / cl.exe) with NO rustc/cargo involved in the
# link step, runs each fixture, and records evidence under report/windows/.
#
# Requires: Visual Studio Build Tools (link.exe, cl.exe, Windows SDK) and a
# rustup toolchain that provides rust-lld (used standalone as lld-link).

[CmdletBinding()]
param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
# Native tools (link/lld-link/cl) write diagnostics to stderr; do not promote
# that to a terminating error.
$PSNativeCommandUseErrorActionPreference = $false
$Root = Split-Path -Parent $PSScriptRoot          # spikes/mlink0
$Repo = Resolve-Path (Join-Path $Root '..\..')
$Out = Join-Path $Root 'out'
$Report = Join-Path $Root 'report\windows'
New-Item -ItemType Directory -Force -Path $Out, $Report | Out-Null

function Write-Log {
    param([string]$Message)
    Write-Host $Message
    Add-Content -Path (Join-Path $Report 'spike.log') -Value $Message
}

# ---- discover MSVC (vcvars) + rust-lld --------------------------------
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) { throw 'vswhere.exe not found' }
$vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsPath) { throw 'Visual Studio C++ tools not found' }
$vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
Write-Log "vcvars: $vcvars"

$toolchainLib = Join-Path $env:USERPROFILE '.rustup\toolchains'
$hostTriple = (& rustc -vV | Select-String '^host:').ToString().Split(':')[1].Trim()
$rustLld = Get-ChildItem (Join-Path $toolchainLib "*\lib\rustlib\$hostTriple\bin\rust-lld.exe") -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $rustLld) { throw 'rust-lld.exe not found' }
$lldLink = Join-Path $Out 'lld-link.exe'
Copy-Item $rustLld.FullName $lldLink -Force
Write-Log "rust-lld (as lld-link): $($rustLld.FullName)"

# import vcvars into this process
$envLines = cmd /c "`"$vcvars`" >nul 2>&1 && set"
foreach ($line in $envLines) {
    if ($line -match '^([^=]+)=(.*)$') {
        [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
    }
}

# ---- build reference runtime archives + spike tooling -----------------
$core = Join-Path $Repo 'target\debug\vut_runtime.lib'
$stdlib = Join-Path $Repo 'target\debug\vut_stdlib_native.lib'
if (-not $SkipBuild) {
    Write-Log 'building reference staticlibs (cargo)...'
    & cargo build -p vut-runtime -p vut-stdlib-native --manifest-path (Join-Path $Repo 'Cargo.toml') 2>&1 |
        Tee-Object -FilePath (Join-Path $Report 'cargo-build.log') | Out-Null

    $objgen = Join-Path $Root 'objgen\target\release\vut-objgen.exe'
    if (-not (Test-Path $objgen)) {
        & cargo build --release --manifest-path (Join-Path $Root 'objgen\Cargo.toml') 2>&1 |
            Tee-Object -FilePath (Join-Path $Report 'objgen-build.log') | Out-Null
    }
}
$objgen = Join-Path $Root 'objgen\target\release\vut-objgen.exe'
if (-not (Test-Path $core)) { throw "missing $core" }
if (-not (Test-Path $stdlib)) { throw "missing $stdlib" }
if (-not (Test-Path $objgen)) { throw "missing $objgen" }

# ---- startup object ---------------------------------------------------
$startupObj = Join-Path $Out 'vut-startup.obj'
& rustc --edition 2024 --crate-type=lib --emit=obj -C panic=abort -C opt-level=2 -o $startupObj (Join-Path $Root 'startup\vut-startup.rs')
if ($LASTEXITCODE -ne 0) { throw 'startup object build failed' }

# ---- fixtures ---------------------------------------------------------
$fixtures = [ordered]@{
    'fx-core-only' = @($core)
    'fx-async'     = @($core)
    'fx-stdlib'    = @($core, $stdlib)
    'fx-http'      = @($core, $stdlib)
}

$crtLibs = @('msvcrt.lib', 'vcruntime.lib', 'ucrt.lib', 'oldnames.lib')
$sysLibs = @('kernel32.lib', 'ntdll.lib', 'userenv.lib', 'ws2_32.lib', 'dbghelp.lib', 'bcrypt.lib', 'advapi32.lib')

function Invoke-Run {
    param([string]$Exe, [string]$Tag)
    $stdout = Join-Path $Report "$Tag.stdout.txt"
    $proc = Start-Process -FilePath $Exe -NoNewWindow -Wait -PassThru -RedirectStandardOutput $stdout -RedirectStandardError (Join-Path $Report "$Tag.stderr.txt")
    $raw = Get-Content $stdout -Raw -ErrorAction SilentlyContinue
    $text = if ($null -eq $raw) { '' } else { $raw.Trim() }
    Write-Log "  run  exit=$($proc.ExitCode) stdout='$text'"
    return $proc.ExitCode
}

function Invoke-Backend {
    param(
        [string]$Fixture,
        [string]$Backend,
        [string]$FilePath,
        [string[]]$Arguments
    )
    $exe = Join-Path $Out "$Fixture-$Backend.exe"
    $log = Join-Path $Report "$Fixture-$Backend.log"
    & $FilePath @Arguments *> $log
    $linkCode = $LASTEXITCODE
    $errs = (Select-String -Path $log -Pattern 'error LNK|error:|fatal error|fatal:' | Measure-Object).Count
    $runCode = 'n/a'
    if ($linkCode -eq 0 -and (Test-Path $exe)) {
        $runCode = Invoke-Run $exe "$Fixture-$Backend"
    }
    Write-Log "  $Backend exit=$linkCode errors=$errs run=$runCode"
    return [pscustomobject]@{ Link = $linkCode; Run = $runCode }
}

$summary = @()
foreach ($fx in $fixtures.Keys) {
    Write-Log "=== $fx ==="
    $obj = Join-Path $Out "$fx.obj"
    & $objgen (Join-Path $Root "fixtures\$fx") $obj | Out-Null
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path $obj)) { throw "objgen failed for $fx" }

    $runtimeLibs = @($fixtures[$fx])

    $linkArgs = @('/NOLOGO', '/SUBSYSTEM:CONSOLE', "/OUT:$(Join-Path $Out "$fx-link.exe")") +
        @($startupObj, $obj) + $runtimeLibs + $crtLibs + $sysLibs
    $link = Invoke-Backend -Fixture $fx -Backend 'link' -FilePath 'link.exe' -Arguments $linkArgs

    $lldArgs = @('/NOLOGO', '/SUBSYSTEM:CONSOLE', "/OUT:$(Join-Path $Out "$fx-lld.exe")") +
        @($startupObj, $obj) + $runtimeLibs + $crtLibs + $sysLibs
    $lld = Invoke-Backend -Fixture $fx -Backend 'lld' -FilePath $lldLink -Arguments $lldArgs

    $clArgs = @('/nologo', "/Fe:$(Join-Path $Out "$fx-cl.exe")") +
        @($startupObj, $obj) + $runtimeLibs + $crtLibs + $sysLibs
    $cl = Invoke-Backend -Fixture $fx -Backend 'cl' -FilePath 'cl.exe' -Arguments $clArgs

    $summary += [pscustomobject]@{
        Fixture  = $fx
        LinkLink = $link.Link; LinkRun = $link.Run
        LldLink  = $lld.Link; LldRun = $lld.Run
        ClLink   = $cl.Link; ClRun = $cl.Run
    }
}

$summary | Format-Table -AutoSize | Out-String | Tee-Object -FilePath (Join-Path $Report 'summary.txt')
Write-Log 'M-LINK.0 Windows spike complete.'
