# Run from Visual Studio Developer PowerShell with the C++ tools and SDK loaded.
[CmdletBinding()]
param([switch]$Run)

$ErrorActionPreference = 'Stop'
if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) {
    throw 'Open Visual Studio Developer PowerShell with the C++ tools installed, then run this script again.'
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Install Rust with its MSVC toolchain and put cargo on PATH.'
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$outputDir = Join-Path $repoRoot 'target/thread-examples'
$manifest = Join-Path $repoRoot 'Cargo.toml'
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

foreach ($name in @('two_workers', 'owned_buffer', 'shared_readers', 'core_context')) {
    $sourcePath = Join-Path $PSScriptRoot "$name.qcore"
    $cPath = Join-Path $outputDir "$name.c"
    $objectPath = Join-Path $outputDir "$name.obj"
    $exePath = Join-Path $outputDir "$name.exe"

    Write-Host "Building $name"
    # `-p qcorec` names the package, which is unchanged; the binary it builds is
    # `quirin`. Selecting by package keeps this working from a checkout without
    # depending on the installed command name.
    & cargo run --quiet --manifest-path $manifest -p qcorec -- emit $sourcePath -o $cPath
    if ($LASTEXITCODE -ne 0) { throw "Quirin emission failed for $name." }

    & cl.exe /nologo /std:c11 /utf-8 /W4 /WX $cPath "/Fo$objectPath" "/Fe:$exePath"
    if ($LASTEXITCODE -ne 0) { throw "MSVC compilation failed for $name." }

    if ($Run) {
        Write-Host "Running $name"
        & $exePath
        if ($LASTEXITCODE -ne 0) { throw "$name exited with code $LASTEXITCODE." }
    }
}

Write-Host "Executables and generated C: $outputDir"
